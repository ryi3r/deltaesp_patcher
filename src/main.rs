#![windows_subsystem = "windows"]

#[allow(unused_imports)]
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    sync::Arc,
    thread::JoinHandle,
    time::Instant,
};

use parking_lot::Mutex;
#[allow(unused_imports)]
use sola_raylib::{
    audio::RaylibAudio,
    color::Color,
    drawing::{RaylibDraw, RaylibTextureModeExt},
    ffi::{self, KeyboardKey, TraceLogLevel},
    math::{Rectangle, Vector2},
    text::RaylibFont,
    texture::{Image, Texture2D},
    window::{self, WindowState},
};
#[allow(unused_imports)]
use oxidelta::compress::{CompressOptions, encoder, decoder};

use crate::{
    image_depth_renderer::ImageDepthRenderer, image_soul_blur_renderer::ImageSoulBlurRenderer,
    typer::Typer,
};

pub mod image_depth_renderer;
pub mod image_soul_blur_renderer;
pub mod typer;

fn ensure_deltarune_in_folder(pb: &Path) -> bool {
    let paths = [
        vec!["DELTARUNE.exe"],
        vec!["data.win"],
        vec!["chapter1_windows", "data.win"],
        vec!["chapter2_windows", "data.win"],
        vec!["chapter3_windows", "data.win"],
        vec!["chapter4_windows", "data.win"],
    ];
    for path in paths {
        let mut npb = pb.to_path_buf();
        for p in path {
            npb.push(p);
        }
        if !npb.exists() || !npb.is_file() {
            println!("not found: {npb:?}->{pb:?}");
            return false;
        }
    }
    if cfg!(feature = "uninstaller") {
        let mut walk = vec![pb.to_path_buf()];
        let mut count = 0usize;
        while let Some(w) = walk.pop() {
            if let Ok(fd) = fs::read_dir(w) {
                for f in fd.flatten() {
                    if let Some(ext) = f.path().extension()
                        && ext == "bak"
                    {
                        count += 1;
                    }
                }
            }
        }
        if count == 0 {
            return false;
        }
    }
    true
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(i8)]
#[cfg(feature = "uninstaller")]
pub enum PatcherState {
    Init = -1,
    StartTalkA,
    StartTalkB,
    FindFolder,
    DeltaruneNotFound,
    DeltaruneFound,
    UninstallPrompt,
    Uninstalling,
    EndUninstalledTalkA,
    EndUninstalledTalkB,
    EndKeptTalkA,
    EndKeptTalkB,
    CloseTimed,
}
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(i8)]
#[cfg(not(feature = "uninstaller"))]
pub enum PatcherState {
    Init = -1,
    StartTalkA,
    StartTalkB,
    SoulEntrance,
    StartTalkC,
    StartTalkD,
    SoulExit,
    StartTalkE,
    StartPatchQuestion,
    RefuseStartPatch,
    FindFolder,
    DeltaruneNotFound,
    DeltaruneFound,
    Patch,
    PatchFailed,
    PatchSuccess,
    EndTalkA,
    CloseTimed,
}

fn main() {
    let files =
        postcard::from_bytes::<HashMap<String, Vec<u8>>>(&fs::read("data.bin").unwrap()).unwrap();
    let (mut rl, thread) = sola_raylib::init()
        .hidden()
        .size(640, 480)
        .title("DeltaESP Patcher")
        .vsync()
        .build();
    rl.set_trace_log(TraceLogLevel::LOG_WARNING);
    rl.set_exit_key(None);
    let mut scale = 1;
    {
        rl.clear_window_state(WindowState::default().set_window_hidden(true));
        let cm = window::get_current_monitor();
        let w = unsafe { ffi::GetMonitorWidth(cm) }; // sola_raylib plz fix
        let h = unsafe { ffi::GetMonitorHeight(cm) };
        while w > 640 * (scale + 1) && h > 480 * (scale + 1) {
            scale += 1;
        }
        rl.set_window_size(640 * scale, 480 * scale);
    }
    rl.set_window_icon(
        Image::load_image_from_mem(
            ".png",
            &files[if cfg!(feature = "uninstaller") {
                "uninstaller.png"
            } else {
                "icon.png"
            }],
        )
        .unwrap(),
    );
    let audio = RaylibAudio::init_audio_device().unwrap();
    let mut render = rl.load_render_texture(&thread, 640, 480).unwrap();
    let fnt = {
        let c = whoami::username()
            .unwrap_or(String::from("JUGADOR"))
            .to_uppercase()
            + "0123456789QWERTYUIOPASDFGHJKLZXCVBNMqwertyuiopasdfghjklzxcvbnm,./<>?:;\"'{}[]¿¡~`áéíóúÁÉÍÓÚñÑ!@#$%^&*()_+-=\\";
        rl.load_font_from_memory(
            &thread,
            ".ttf",
            &files["8bitoperator_jve.ttf"],
            16,
            Some(&c),
        )
        .unwrap()
    };
    let img_depth_tex = rl
        .load_texture_from_image(
            &thread,
            &Image::load_image_from_mem(
                ".png",
                &files[if cfg!(feature = "uninstaller") {
                    "image_depth_gray.png"
                } else {
                    "image_depth.png"
                }],
            )
            .unwrap(),
        )
        .unwrap();

    let timer = Instant::now();
    let mut idr = Vec::new();
    let mut idr_time = Instant::now();

    let typer_progress = Arc::new(Mutex::new(0f32));
    let progress = Arc::new(Mutex::new(String::new()));
    let folder = Arc::new(Mutex::new(PathBuf::new()));

    if let Ok(sl) = steamlocate::locate_all() {
        for lib in sl {
            if let Ok(Some(a)) = lib.find_app(1_671_210) { // deltarune's steam id
                *folder.lock() = a.1.resolve_app_dir(&a.0);
                println!("UPDATE FOLDER");
            }
        }
    }

    let mut state = PatcherState::Init;
    let img_soul_blur_tex = rl
        .load_texture_from_image(
            &thread,
            &Image::load_image_from_mem(".png", &files["image_soul_blur.png"]).unwrap(),
        )
        .unwrap();
    #[allow(unused)]
    let mut soul = ImageSoulBlurRenderer::new(img_soul_blur_tex);
    #[allow(unused)]
    let aud_appearance = audio
        .new_sound_from_wave(
            &audio
                .new_wave_from_memory(".wav", &files["audio_appearance.wav"])
                .unwrap(),
        )
        .unwrap();
    let aud_another_him = audio
        .new_sound_from_wave(
            &audio
                .new_wave_from_memory(".ogg", &files["audio_anotherhim.ogg"])
                .unwrap(),
        )
        .unwrap();
    #[cfg(feature = "uninstaller")]
    let aud_face = audio
        .new_sound_from_wave(
            &audio
                .new_wave_from_memory(".ogg", &files["face.ogg"])
                .unwrap(),
        )
        .unwrap();
    #[allow(unused)]
    let mut play_another_him = cfg!(feature = "uninstaller");
    #[allow(unused)]
    let mut pitch_another_him = if cfg!(feature = "uninstaller") {
        0.001
    } else {
        0.2
    };
    let mut typer = Typer::default();
    typer.pos = Vector2::new(320., 220.);
    #[cfg(feature = "uninstaller")]
    {
        typer.pos.y = 100.;
    }
    let mut galpha = 1f32;
    let mut falpha = 1f32;
    let mut inst_alpha = 0f32;
    let mut goner_alpha = 1f32;
    let mut patch_task = None::<JoinHandle<()>>;
    #[allow(unused)]
    let mut goner = None::<Texture2D>;
    let mut qalpha = 0f32;
    let mut qoption = 0;
    let mut qanswered = false;
    #[allow(unused)]
    let mut skipped = false;
    #[allow(unused_mut)]
    let mut skipping = false;

    let mut close = false;
    #[cfg(feature = "uninstaller")]
    let show_friend = rand::random_range(1..=20) == 15;
    #[cfg(feature = "uninstaller")]
    let mut shown_friend = false;
    #[cfg(feature = "uninstaller")]
    let img_friend = rl
        .load_texture_from_image(
            &thread,
            &Image::load_image_from_mem(".png", &files["friend.png"]).unwrap(),
        )
        .unwrap();
    #[cfg(feature = "uninstaller")]
    let mut friend_scale = 1f32;

    //state = PatcherState::CloseTimed;
    //galpha = 2.;

    //let mut cl = 0;
    while !rl.window_should_close() && !close {
        let mut d = rl.begin_drawing(&thread);
        let delta = d.get_frame_time();
        d.draw_texture_mode(&thread, &mut render, |mut d| {
            d.clear_background(Color::BLACK);
            if typer.playing {
                if skipping {
                    if falpha > 0. {
                        falpha = (falpha - delta).max(0.);
                    }
                } else if falpha < 1. {
                    falpha = (falpha + delta).min(1.);
                }
            }
            #[cfg(not(feature = "uninstaller"))]
            if (d.is_key_pressed(KeyboardKey::KEY_C) || d.is_key_pressed(KeyboardKey::KEY_LEFT_CONTROL) || d.is_key_pressed(KeyboardKey::KEY_RIGHT_CONTROL)) && state < PatcherState::SoulExit {
                skipped = true;
                state = PatcherState::SoulExit;
                idr_time = Instant::now();
                play_another_him = true;
                typer.increase_typer = false;
                galpha = 0.;
                skipping = true;
                if soul.time >= soul.get_max_time() {
                    soul.momentum = -0.5;
                    soul.time = soul.get_max_time();
                    aud_appearance.play();
                } else {
                    soul.time = -100.;
                    soul.momentum = 0.;
                }
            }
            let is_skip_pressed = d.is_key_down(KeyboardKey::KEY_X)
                || d.is_key_down(KeyboardKey::KEY_LEFT_SHIFT)
                || d.is_key_down(KeyboardKey::KEY_RIGHT_SHIFT);
            /*{
                let f = files.keys().filter(|x| x.starts_with("goners/")).collect::<Vec<_>>();
                if d.is_key_down(KeyboardKey::KEY_A) {
                    goner = None;
                    cl = (cl + 1) % f.len();
                }
                if goner.is_none() {
                    goner = Some(d.load_texture_from_image(&thread, &Image::load_image_from_mem(".png", &files[f[cl]]).unwrap()).unwrap());
                    goner_alpha = 1.;
                    *typer_progress.lock() = 1.;
                }
            }*/
            match state {
                PatcherState::Init => {
                    if galpha > -1. {
                        galpha -= delta * (if is_skip_pressed { 2. } else { 1. });
                    }
                    if inst_alpha < 1. {
                        inst_alpha = (inst_alpha + (delta * (if is_skip_pressed { 2. } else { 1. }))).min(1.);
                    }
                    if galpha <= -1. && inst_alpha >= 1. {
                        state = PatcherState::StartTalkA;
                    }
                }
                PatcherState::StartTalkA => {
                    if !typer.playing {
                        galpha = 2.;
                        typer.start(if cfg!(feature = "uninstaller") {
                            "VEO QUE QUIERES^^10\nCORTAR LA CONEXIÓN."
                        } else {
                            "¿ESTÁS^^10 AHÍ?"
                        });
                    } else if typer.is_finished() {
                        if galpha > -1. {
                            galpha -= delta * (if is_skip_pressed { 2. } else { 1. });
                        } else {
                            typer.stop();
                            state = PatcherState::StartTalkB;
                        }
                    }
                }
                PatcherState::StartTalkB => {
                    if !typer.playing {
                        galpha = 2.;
                        typer.start(if cfg!(feature = "uninstaller") {
                            "ACEPTO TU DECISIÓN."
                        } else {
                            "¿ESTAMOS^^10 CONECTADOS?"
                        });
                    } else if typer.is_finished() {
                        if galpha > -1. {
                            galpha -= delta * (if is_skip_pressed { 2. } else { 1. });
                        } else {
                            typer.stop();
                            #[cfg(not(feature = "uninstaller"))]
                            {
                                state = PatcherState::SoulEntrance;
                                aud_appearance.play();
                            }
                            #[cfg(feature = "uninstaller")]
                            {
                                let mut fl = folder.lock();
                                state = if ensure_deltarune_in_folder(&fl) {
                                    PatcherState::UninstallPrompt
                                } else {
                                    *fl = PathBuf::new();
                                    println!("CLEAR FOLDER (2)");
                                    PatcherState::FindFolder
                                }
                            }
                        }
                    }
                }
                #[cfg(not(feature = "uninstaller"))]
                PatcherState::SoulEntrance => {
                    if soul.time > soul.get_max() {
                        if galpha > 0. {
                            galpha -= delta * (if is_skip_pressed { 2. } else { 1. });
                        } else {
                            state = PatcherState::StartTalkC;
                        }
                    } else {
                        galpha = 1.;
                    }
                }
                #[cfg(not(feature = "uninstaller"))]
                PatcherState::StartTalkC => {
                    if !typer.playing {
                        galpha = 2.;
                        typer.pos.y = 100.;
                        typer.start("EXCELENTE.");
                    } else if typer.is_finished() {
                        if galpha > -1. {
                            galpha -= delta * (if is_skip_pressed { 2. } else { 1. });
                        } else {
                            typer.stop();
                            state = PatcherState::StartTalkD;
                        }
                    }
                }
                #[cfg(not(feature = "uninstaller"))]
                PatcherState::StartTalkD => {
                    if !typer.playing {
                        galpha = 2.;
                        typer.start("PODEMOS^^10 COMENZAR.");
                    } else if typer.is_finished() {
                        if galpha > -1. {
                            galpha -= delta * (if is_skip_pressed { 2. } else { 1. });
                        } else {
                            typer.stop();
                            state = PatcherState::SoulExit;
                            idr_time = Instant::now();
                            play_another_him = true;
                            galpha = 0.;
                            soul.momentum = -0.5;
                            soul.time = soul.get_max_time();
                            aud_appearance.play();
                        }
                    }
                }
                #[cfg(not(feature = "uninstaller"))]
                PatcherState::SoulExit => {
                    galpha += delta;
                    if inst_alpha > 0. {
                        inst_alpha = (inst_alpha - delta).max(0.);
                    }
                    if pitch_another_him < 0.96 {
                        pitch_another_him = (pitch_another_him + 0.02 * (delta / (1. / 30.))).min(0.96);
                    }
                    if galpha >= 1. && pitch_another_him >= 0.96 && inst_alpha <= 0. {
                        typer.pos.y = 100.;
                        skipping = false;
                        state = if skipped {
                            typer.stop();
                            typer.instant_show = true;
                            PatcherState::StartPatchQuestion
                        } else {
                            PatcherState::StartTalkE
                        };
                    }
                }
                #[cfg(not(feature = "uninstaller"))]
                PatcherState::StartTalkE => {
                    if !typer.playing {
                        galpha = 2.;
                        typer.start("AHORA.");
                    } else if typer.is_finished() {
                        if galpha > -1. {
                            galpha -= delta * (if is_skip_pressed { 2. } else { 1. });
                        } else {
                            typer.stop();
                            state = PatcherState::StartPatchQuestion;
                        }
                    }
                }
                #[cfg(not(feature = "uninstaller"))]
                PatcherState::StartPatchQuestion => {
                    if !typer.playing {
                        typer.start("¿DESEAS INICIAR^^10\nLA CONVERSIÓN^^10\nDE FORMA AUTOMÁTICA?");
                        galpha = 2.;
                        qoption = 0;
                        qalpha = 0.;
                        qanswered = false;
                    } else {
                        if !qanswered {
                            qalpha = (qalpha + delta).min(1.);
                            if d.is_key_down(KeyboardKey::KEY_Z) || d.is_key_down(KeyboardKey::KEY_ENTER) || d.is_key_down(KeyboardKey::KEY_KP_ENTER) {
                                qanswered = true;
                            }
                        } else {
                            qalpha = (qalpha - delta).max(0.);
                        }
                        if typer.is_finished()
                            && qanswered {
                                if galpha > -1. {
                                    galpha -= delta * (if is_skip_pressed { 2. } else { 1. });
                                } else {
                                    typer.stop();
                                    let mut fl = folder.lock();
                                    state = if qoption == 0 && ensure_deltarune_in_folder(&fl) {
                                        PatcherState::Patch
                                    } else {
                                        println!("CLEAR FOLDER (1)");
                                        *fl = PathBuf::new();
                                        PatcherState::RefuseStartPatch
                                    };
                                }
                            }
                    }
                }
                #[cfg(not(feature = "uninstaller"))]
                PatcherState::RefuseStartPatch => {
                    if !typer.playing {
                        galpha = 2.;
                        typer.start("ENTENDIDO.");
                    } else if typer.is_finished() {
                        if galpha > -1. {
                            galpha -= delta * (if is_skip_pressed { 2. } else { 1. });
                        } else {
                            typer.stop();
                            state = PatcherState::FindFolder;
                        }
                    }
                }
                PatcherState::FindFolder => {
                    if d.is_file_dropped() {
                        for f in d.load_dropped_files().paths() {
                            if f.is_empty() {
                                continue;
                            }
                            let f = PathBuf::from(f);
                            if f.exists() {
                                if f.is_file() {
                                    *folder.lock() = f.parent().unwrap().to_path_buf();
                                } else if f.is_dir() {
                                    *folder.lock() = f;
                                }
                                println!("UPDATE FOLDER");
                            }
                        }
                    }
                    if !typer.playing {
                        galpha = 2.;
                        typer.pos.y = 100.;
                        typer.start("ARRASTRA LA CARPETA\nDE DELTARUNE^^10\nA ESTA VENTANA.");
                    } else if typer.is_finished() && !folder.lock().is_empty() {
                        if galpha > -1. {
                            galpha -= delta * (if is_skip_pressed { 2. } else { 1. });
                        } else {
                            typer.stop();
                            state = if ensure_deltarune_in_folder(&folder.lock()) {
                                PatcherState::DeltaruneFound
                            } else {
                                PatcherState::DeltaruneNotFound
                            };
                        }
                    }
                }
                PatcherState::DeltaruneNotFound => {
                    *folder.lock() = PathBuf::new();
                    if !typer.playing {
                        galpha = 2.;
                        typer.start("DELTARUNE^^10\nNO FUE ENCONTRADO.");
                    } else if typer.is_finished() {
                        if galpha > -1. {
                            galpha -= delta * (if is_skip_pressed { 2. } else { 1. });
                        } else {
                            typer.stop();
                            state = PatcherState::FindFolder;
                        }
                    }
                }
                PatcherState::DeltaruneFound => {
                    if !typer.playing {
                        galpha = 2.;
                        typer.start("EXCELENTE.");
                    } else if typer.is_finished() {
                        if galpha > -1. {
                            galpha -= delta * (if is_skip_pressed { 2. } else { 1. });
                        } else {
                            typer.stop();

                            #[cfg(not(feature = "uninstaller"))]
                            {
                                state = PatcherState::Patch;
                            }
                            #[cfg(feature = "uninstaller")]
                            {
                                state = PatcherState::UninstallPrompt;
                            }
                        }
                    }
                }
                #[cfg(feature = "uninstaller")]
                PatcherState::UninstallPrompt => {
                    if !typer.playing {
                        typer.start("¿COMENZAR^^10\nLA ESCISIÓN?");
                        galpha = 2.;
                        qoption = 0;
                        qalpha = 0.;
                        qanswered = false;
                    } else {
                        if !qanswered {
                            qalpha = (qalpha + delta).min(1.);
                            if d.is_key_down(KeyboardKey::KEY_Z) || d.is_key_down(KeyboardKey::KEY_ENTER) || d.is_key_down(KeyboardKey::KEY_KP_ENTER) {
                                qanswered = true;
                            }
                        } else {
                            qalpha = (qalpha - delta).max(0.);
                        }
                        if typer.is_finished()
                            && qanswered {
                                if galpha > -1. {
                                    galpha -= delta * (if is_skip_pressed { 2. } else { 1. });
                                } else {
                                    typer.stop();
                                    let mut fl = folder.lock();
                                    state = if qoption == 0 {
                                        PatcherState::Uninstalling
                                    } else {
                                        *fl = PathBuf::new();
                                        PatcherState::EndKeptTalkA
                                    };
                                }
                            }
                    }
                }
                #[cfg(feature = "uninstaller")]
                PatcherState::Uninstalling => {
                    if patch_task.is_none() {
                        *typer_progress.lock() = 0.;
                        *progress.lock() = String::new();
                        {
                            let progress = progress.clone();
                            let folder = folder.clone();
                            let typer_progress = typer_progress.clone();
                            patch_task = Some(std::thread::spawn(move || {
                                *progress.lock() = "Desinstalando parche...".into();
                                let init_path = std::env::current_dir().unwrap();
                                let mut dirs = vec![init_path.clone()];
                                dirs[0].push("patch");
                                let comp_count = dirs[0].components().count();
                                let mut count = 0usize;
                                {
                                    let mut dirs = dirs.clone();
                                    while let Some(dir) = dirs.pop() {
                                        if let Ok(d) = fs::read_dir(dir) {
                                            for f in d.flatten() {
                                                if let Ok(ft) = f.file_type() && ft.is_dir() {
                                                    dirs.push(f.path());
                                                } else {
                                                    count += 1;
                                                }
                                            }
                                        }
                                    }
                                }
                                println!("count! {count}");
                                let mut done = 0usize;
                                while let Some(dir) = dirs.pop() {
                                    if let Ok(d) = fs::read_dir(&dir) {
                                        for f in d.flatten() {
                                            if let Ok(ft) = f.file_type() && ft.is_dir() {
                                                dirs.push(f.path());
                                            }
                                        }
                                    }
                                    // ensure that the folder exists in the output destination...
                                    let _ = fs::create_dir_all(&dir);
                                    if let Ok(d) = fs::read_dir(&dir) {
                                        for f in d.flatten() {
                                            if let Ok(ft) = f.file_type() && ft.is_file() {
                                                let mut show_path = f.path().components().skip(comp_count).collect::<PathBuf>().to_string_lossy().to_string().replace(".xdelta", "");
                                                println!("processing: {show_path}");
                                                if show_path.len() > 20 {
                                                    show_path = show_path[(show_path.len() - 20)..].to_string();
                                                    let mut idx = show_path.find('/');
                                                    if idx.is_none() {
                                                        idx = show_path.find('\\');
                                                    }
                                                    if let Some(idx) = idx {
                                                        show_path = show_path[idx..].to_string();
                                                    }
                                                    show_path = String::from("...") + &show_path;
                                                }

                                                *progress.lock() = format!("Procesando {show_path}...");
                                                let mut spath = folder.lock().clone();
                                                spath.extend({
                                                    let p = f.path();
                                                    if let Some(ext) = p.extension() && ext == "xdelta" {
                                                        p.with_extension("")
                                                    } else {
                                                        p
                                                    }
                                                }.components().skip(comp_count));
                                                let bpath = spath.with_added_extension("bak");
                                                if bpath.exists() && bpath.is_file() {
                                                    if fs::metadata(&bpath).unwrap().len() > 0 {
                                                        if let Ok(patched) = decoder::decode_all(&fs::read(&spath).unwrap(), &fs::read(&bpath).unwrap()) {
                                                            fs::write(spath, &patched).unwrap();
                                                        } else {
                                                            println!("unable to apply backup patch! was it an update?");
                                                        }
                                                        //fs::copy(&bpath, &spath).unwrap();
                                                    } else {
                                                        fs::remove_file(&spath).unwrap();
                                                    }
                                                    fs::remove_file(&bpath).unwrap();
                                                    //println!("spath:{spath:?},bpath:{bpath:?}");
                                                }

                                                done += 1;
                                                let tp = *typer_progress.lock();
                                                *typer_progress.lock() = tp.max(done as f32 / count as f32);
                                            }
                                        }
                                    }
                                }
                                *progress.lock() = "Finalizado.".into();
                                *typer_progress.lock() = 1.;
                            }));
                        }
                    }
                    if !typer.playing {
                        galpha = 2.;
                        goner_alpha = galpha;
                        typer.start("LA ESCISIÓN^^10\nESTÁ TOMANDO LUGAR.");
                    } else if patch_task.as_ref().map(|x| x.is_finished()).unwrap_or(false) && typer.is_finished() {
                        if galpha > -1. {
                            galpha -= delta * (if is_skip_pressed { 2. } else { 1. });
                        } else {
                            *progress.lock() = String::new();
                            typer.stop();
                            if let Some(pt) = patch_task.take()
                                && let Err(e) = pt.join() {
                                    let mut f = BufWriter::new(File::create("uninstalling_error.txt").unwrap());
                                    f.write_all(if let Some(s) = e.downcast_ref::<&str>() {
                                        s.as_bytes()
                                    } else if let Some(s) = e.downcast_ref::<String>() {
                                        s.as_bytes()
                                    } else {
                                        "???".as_bytes()
                                    }).unwrap();
                                    f.flush().unwrap();
                                }
                            state = PatcherState::EndUninstalledTalkA;
                        }
                    }
                }
                #[cfg(not(feature = "uninstaller"))]
                PatcherState::Patch => {
                    if patch_task.is_none() {
                        {
                            let f = files.keys().filter(|x| x.starts_with("goners/")).collect::<Vec<_>>();
                            goner = Some(d.load_texture_from_image(&thread, &Image::load_image_from_mem(".png", &files[f[rand::random_range(0..f.len())]]).unwrap()).unwrap());
                        }
                        *typer_progress.lock() = 0.;
                        *progress.lock() = String::new();
                        {
                            let progress = progress.clone();
                            let folder = folder.clone();
                            let typer_progress = typer_progress.clone();
                            patch_task = Some(std::thread::spawn(move || {
                                *progress.lock() = "Inicializando parche...".into();
                                let init_path = std::env::current_dir().unwrap();
                                let mut dirs = vec![init_path.clone()];
                                dirs[0].push("patch");
                                let comp_count = dirs[0].components().count();
                                let mut count = 0usize;
                                {
                                    let mut dirs = dirs.clone();
                                    while let Some(dir) = dirs.pop() {
                                        if let Ok(d) = fs::read_dir(dir) {
                                            for f in d.flatten() {
                                                if let Ok(ft) = f.file_type() && ft.is_dir() {
                                                    dirs.push(f.path());
                                                } else {
                                                    count += 1;
                                                }
                                            }
                                        }
                                    }
                                }
                                println!("count! {count}");
                                let mut done = 0usize;
                                while let Some(dir) = dirs.pop() {
                                    if let Ok(d) = fs::read_dir(&dir) {
                                        for f in d.flatten() {
                                            if let Ok(ft) = f.file_type() && ft.is_dir() {
                                                dirs.push(f.path());
                                            }
                                        }
                                    }
                                    // ensure that the folder exists in the output destination...
                                    let _ = fs::create_dir_all(&dir);
                                    if let Ok(d) = fs::read_dir(&dir) {
                                        for f in d.flatten() {
                                            if let Ok(ft) = f.file_type() && ft.is_file() {
                                                let mut show_path = f.path().components().skip(comp_count).collect::<PathBuf>().to_string_lossy().to_string().replace(".xdelta", "");
                                                println!("processing: {show_path}");
                                                if show_path.len() > 20 {
                                                    show_path = show_path[(show_path.len() - 20)..].to_string();
                                                    let mut idx = show_path.find('/');
                                                    if idx.is_none() {
                                                        idx = show_path.find('\\');
                                                    }
                                                    if let Some(idx) = idx {
                                                        show_path = show_path[idx..].to_string();
                                                    }
                                                    show_path = String::from("...") + &show_path;
                                                }

                                                *progress.lock() = format!("Procesando {show_path}...");
                                                let is_xdelta = f.path().extension().map(|ext| ext == "xdelta").unwrap_or(false);
                                                let mut spath = folder.lock().clone();
                                                spath.extend({
                                                    let p = f.path();
                                                    if is_xdelta {
                                                        p.with_extension("")
                                                    } else {
                                                        p
                                                    }
                                                }.components().skip(comp_count));
                                                let bpath = spath.with_added_extension("bak");
                                                //println!("spath:{spath:?},bpath:{bpath:?}");
                                                if !is_xdelta {
                                                    if spath.exists() && if bpath.exists() {
                                                        fs::metadata(&bpath).unwrap().len() > 0
                                                    } else {
                                                        true
                                                    } {
                                                        let mut data = fs::read(&spath).unwrap();
                                                        if bpath.exists() {
                                                            println!("decoding backup xdelta");
                                                            *progress.lock() = format!("Decodificando el backup de {show_path}...");
                                                            if let Ok(d) = decoder::decode_all(&data, &fs::read(&bpath).unwrap()) {
                                                                data = d;
                                                            } else {
                                                                println!("backup decode failed! was it an update?");
                                                            }
                                                        }
                                                        println!("making xdelta for backup");
                                                        *progress.lock() = format!("Haciendo backup de {show_path}...");
                                                        let mut fb = BufWriter::new(File::create(&bpath).unwrap());
                                                        encoder::encode_all(
                                                            &mut fb,
                                                            &fs::read(f.path()).unwrap(),
                                                            &data,
                                                            CompressOptions::default())
                                                            .unwrap();
                                                        fb.flush().unwrap();
                                                        //fs::copy(&spath, &bpath).unwrap();
                                                    } else {
                                                        fs::write(&bpath, []).unwrap();
                                                    }
                                                }
                                                if is_xdelta {
                                                    let mut read = fs::read(&spath).unwrap();
                                                    if bpath.exists() && bpath.is_file() {
                                                        *progress.lock() = format!("Decodificando el backup de {show_path}...");
                                                        if let Ok(r) = decoder::decode_all(&read, &fs::read(&bpath).unwrap()) {
                                                            read = r;
                                                        } else {
                                                            println!("backup decode failed! was it an update?");
                                                        }
                                                    }
                                                    //println!("og:{spath:?},patch:{:?}", f.path());
                                                    *progress.lock() = format!("Aplicando el parche para {show_path}...");
                                                    let dec = decoder::decode_all(&read, &fs::read(f.path()).unwrap()).unwrap();
                                                    
                                                    println!("making xdelta for backup");
                                                    *progress.lock() = format!("Realizando backup de {show_path}...");
                                                    {
                                                        //fs::write(bpath.with_added_extension("source"), &dec).unwrap();
                                                        //fs::write(bpath.with_added_extension("target"), &read).unwrap();
                                                        let mut fb = BufWriter::new(File::create(&bpath).unwrap());
                                                        encoder::encode_all(&mut fb, &dec, &read, CompressOptions {
                                                            level: 3,
                                                            ..Default::default()
                                                        }).unwrap();
                                                        fb.flush().unwrap();
                                                    }

                                                    {
                                                        let mut fw = BufWriter::new(File::create(&spath).unwrap());
                                                        fw.write_all(&dec).unwrap();
                                                        fw.flush().unwrap();
                                                    }
                                                    println!("wrote delta");

                                                    *progress.lock() = format!("{show_path} OK");
                                                } else {
                                                    *progress.lock() = format!("Copiando archivo {show_path}");
                                                    //println!("{:?},{spath:?}", f.path());
                                                    fs::copy(f.path(), &spath).unwrap();
                                                    *progress.lock() = format!("{show_path} OK");
                                                    println!("copied");
                                                }

                                                done += 1;
                                                let tp = *typer_progress.lock();
                                                *typer_progress.lock() = tp.max(done as f32 / count as f32);
                                            }
                                        }
                                    }
                                }
                                *progress.lock() = "Finalizado.".into();
                                *typer_progress.lock() = 1.;
                            }));
                        }
                    }
                    if !typer.playing {
                        galpha = 2.;
                        goner_alpha = galpha;
                        typer.start("AHORA.^^10\nSE REALIZARÁ LA CONVERSIÓN.");
                    } else if patch_task.as_ref().map(|x| x.is_finished()).unwrap_or(false) && typer.is_finished() {
                        if galpha > -1. {
                            galpha -= delta * (if is_skip_pressed { 2. } else { 1. });
                        } else {
                            *progress.lock() = String::new();
                            typer.stop();
                            let mut success = false;
                            if let Some(pt) = patch_task.take() {
                                if let Err(e) = pt.join() {
                                    let mut f = BufWriter::new(File::create("patching_error.txt").unwrap());
                                    f.write_all(if let Some(s) = e.downcast_ref::<&str>() {
                                        s.as_bytes()
                                    } else if let Some(s) = e.downcast_ref::<String>() {
                                        s.as_bytes()
                                    } else {
                                        "???".as_bytes()
                                    }).unwrap();
                                    f.flush().unwrap();
                                } else {
                                    success = true;
                                }
                            }
                            state = if success {
                                PatcherState::PatchSuccess
                            } else {
                                PatcherState::PatchFailed
                            };
                        }
                        if state != PatcherState::PatchFailed {
                            goner_alpha = galpha;
                        }
                    }
                }
                #[cfg(not(feature = "uninstaller"))]
                PatcherState::PatchFailed => {
                    let mut go_lower = true;
                    if !typer.playing {
                        goner = Some(d.load_texture_from_image(&thread, &Image::load_image_from_mem(".png", &files["error.png"]).unwrap()).unwrap());
                        *typer_progress.lock() = 1.;
                        galpha = 2.;
                        typer.start("LA CONVERSIÓN^5 HA FALLADO.^^10\nASEGÚRATE^5 DE TENER LOS^^10\nARCHIVOS ORIGINALES.");
                    } else if typer.is_finished() {
                        go_lower = false;
                        if galpha > -1. {
                            galpha -= delta * (if is_skip_pressed { 2. } else { 1. });
                        } else if pitch_another_him >= 0.96 {
                            typer.stop();
                            println!("CLEAR FOLDER");
                            *folder.lock() = PathBuf::new();
                            state = PatcherState::FindFolder;
                        }
                        goner_alpha = galpha;
                    }

                    if go_lower {
                        if pitch_another_him > 0.02 {
                            pitch_another_him = (pitch_another_him - (0.02 * (d.get_frame_time() / (1. / 30.)))).max(0.02);
                        } else {
                            play_another_him = false;
                        }
                    } else {
                        if !play_another_him {
                            play_another_him = true;
                        }
                        if pitch_another_him < 0.96 {
                            pitch_another_him = (pitch_another_him + (0.02 * (d.get_frame_time() / (1. / 30.)))).min(0.96);
                        }
                    }
                }
                #[cfg(not(feature = "uninstaller"))]
                PatcherState::PatchSuccess => {
                    if !typer.playing {
                        galpha = 2.;
                        typer.start("LA CONVERSIÓN.^^10\nFUE REALIZADA^5 CON ÉXITO.");
                    } else if typer.is_finished() {
                        if galpha > -1. {
                            galpha -= delta * (if is_skip_pressed { 2. } else { 1. });
                        } else {
                            typer.stop();
                            state = PatcherState::EndTalkA;
                        }
                    }
                }
                #[cfg(not(feature = "uninstaller"))]
                PatcherState::EndTalkA => {
                    if !typer.playing {
                        galpha = 2.;
                        typer.start(&format!("NOS VEMOS PRONTO^^10\n{}.", whoami::username().unwrap_or(String::from("JUGADOR")).to_uppercase()));
                    } else if typer.is_finished() {
                        if galpha > -1. {
                            galpha -= delta * (if is_skip_pressed { 2. } else { 1. });
                        } else {
                            typer.stop();
                            state = PatcherState::CloseTimed;
                            galpha = 3.;
                        }
                    }

                    if pitch_another_him > 0.02 {
                        pitch_another_him = (pitch_another_him - (0.02 * (d.get_frame_time() / (1. / 30.)))).max(0.02);
                    } else {
                        play_another_him = false;
                    }
                }
                #[cfg(feature = "uninstaller")]
                PatcherState::EndUninstalledTalkA => {
                    if !typer.playing {
                        galpha = 2.;
                        typer.start("ESTÁ HECHO.");
                    } else if typer.is_finished() {
                        if galpha > -1. {
                            galpha -= delta * (if is_skip_pressed { 2. } else { 1. });
                        } else {
                            typer.stop();
                            state = PatcherState::EndUninstalledTalkB;
                        }
                    }
                }
                #[cfg(feature = "uninstaller")]
                PatcherState::EndUninstalledTalkB => {
                    if !typer.playing {
                        galpha = 2.;
                        typer.start("ESPERO^^10\nVOLVERTE A VER PRONTO.");
                    } else if typer.is_finished() {
                        if galpha > -1. {
                            galpha -= delta * (if is_skip_pressed { 2. } else { 1. });
                        } else {
                            typer.stop();
                            galpha = 3.;
                            state = PatcherState::CloseTimed;
                        }
                    }
                }
                #[cfg(feature = "uninstaller")]
                PatcherState::EndKeptTalkA => {
                    if !typer.playing {
                        galpha = 2.;
                        typer.start("ENTONCES^^10\nLA CONEXIÓN SE MANTENDRÁ.");
                    } else if typer.is_finished() {
                        if galpha > -1. {
                            galpha -= delta * (if is_skip_pressed { 2. } else { 1. });
                        } else {
                            typer.stop();
                            state = PatcherState::EndKeptTalkB;
                        }
                    }
                }
                #[cfg(feature = "uninstaller")]
                PatcherState::EndKeptTalkB => {
                    if !typer.playing {
                        galpha = 2.;
                        typer.start("NOS VEMOS.");
                    } else if typer.is_finished() {
                        if galpha > -1. {
                            galpha -= delta * (if is_skip_pressed { 2. } else { 1. });
                        } else {
                            typer.stop();
                            galpha = 3.;
                            state = PatcherState::CloseTimed;
                        }
                    }
                }
                PatcherState::CloseTimed => {
                    if idr.is_empty() {
                        if galpha > 0. {
                            galpha -= d.get_frame_time();
                        } else {
                            #[cfg(feature = "uninstaller")]
                            if show_friend && !shown_friend {
                                shown_friend = true;
                                aud_face.play();
                            } else if !aud_face.is_playing() {
                                close = true;
                            }
                            #[cfg(not(feature = "uninstaller"))]
                            {
                                close = true;
                            }
                        }
                    }
                }
            }
            if play_another_him {
                aud_another_him.set_pitch(pitch_another_him);
                if !aud_another_him.is_playing() {
                    aud_another_him.play();
                }
            } else if aud_another_him.is_playing() {
                aud_another_him.stop();
            }
            let mult  =
            {
                let cond;
                #[cfg(not(feature = "uninstaller"))]
                {
                    cond = state >= PatcherState::StartTalkD && (state != PatcherState::PatchFailed || typer.is_finished()) && state < PatcherState::EndTalkA;
                }
                #[cfg(feature = "uninstaller")]
                {
                    cond = state <= PatcherState::Uninstalling;
                    if !cond {
                        if pitch_another_him > 0.02 {
                            pitch_another_him = (pitch_another_him - (delta / 3.)).max(0.02);
                        } else {
                            play_another_him = false;
                        }
                    } else {
                        pitch_another_him = (pitch_another_him + (delta / 3.)).min(0.2);
                    }
                }
                if cond {
                    if idr_time.elapsed().as_secs_f32() >= 40. / 30. {
                        idr_time = Instant::now();
                        idr.push(ImageDepthRenderer {
                            speed: 0.0025,
                            ..Default::default()
                        });
                    }
                    1.
                } else {
                    7.
                }
            };
            for idr_item in (0..idr.len()).rev() {
                if idr[idr_item].draw(&img_depth_tex, &mut d, mult) {
                    idr.remove(idr_item);
                }
            }
            #[cfg(not(feature = "uninstaller"))]
            if state >= PatcherState::SoulEntrance {
                soul.draw(&mut d);
            }
            let alpha = (timer.elapsed().as_secs_f32() * 2.).sin() * 0.2 + 0.6;
            let fake_galpha = if skipping { 1. } else { galpha };
            //println!("galpha={fake_galpha:.2} falpha={falpha:.2} final={:.2}", fake_galpha.clamp(0., 1.) * falpha.clamp(0., 1.));
            typer.draw(&fnt, &mut d, alpha, fake_galpha.clamp(0., 1.) * falpha.clamp(0., 1.));
            {
                let cond;
                #[cfg(not(feature = "uninstaller"))]
                {
                    cond = state == PatcherState::Patch;
                }
                #[cfg(feature = "uninstaller")]
                {
                    cond = state == PatcherState::Uninstalling;
                }
                if cond && galpha > 0. {
                    Typer::draw_text_centered_half_part(
                        &mut d,
                        &fnt,
                        Vector2::new(
                            320.,
                            typer.pos.y + fnt.measure_text(typer.get_text(), 32., 8.).y + 20.,
                        ),
                        &progress.lock(),
                        usize::MAX,
                        alpha,
                        galpha.clamp(0., 1.),
                    );
                }
            }
            {
                let cond;
                #[cfg(not(feature = "uninstaller"))]
                {
                    cond = state == PatcherState::StartPatchQuestion;
                }
                #[cfg(feature = "uninstaller")]
                {
                    cond = state == PatcherState::UninstallPrompt;
                }
                if cond {
                    let qs = ["SÍ", "NO"];
                    for (i, &q) in qs.iter().enumerate() {
                        Typer::draw_text_centered_part(
                            &mut d,
                            &fnt,
                            Vector2::new(320., 260. + (i as f32 * 40.)),
                            q,
                            usize::MAX,
                            alpha,
                            galpha.clamp(0., 1.),
                            if i == qoption { Color::YELLOW } else { Color::WHITE }.alpha(qalpha)
                        );
                    }
                    if !qanswered {
                        if d.is_key_pressed(KeyboardKey::KEY_DOWN) {
                            qoption += 1;
                            if qoption >= qs.len() {
                                qoption = 0;
                            }
                        } else if d.is_key_pressed(KeyboardKey::KEY_UP) {
                            if qoption > 0 {
                                qoption -= 1;
                            } else {
                                qoption = qs.len() - 1;
                            }
                        }
                    }
                }
            }
            #[cfg(not(feature = "uninstaller"))]
            if inst_alpha > 0. {
                Typer::draw_text_half(&mut d, &fnt, Vector2::new(10., 10.), "Presiona [X/SHIFT] para ir más rápido.\nPresiona [C/CTRL] para saltar la intro.", alpha, inst_alpha);
            }
            if let Some(goner) = goner.as_ref() {
                let mut scale = 1.;
                while (goner.height as f32 * (scale + 0.5)) < 200. {
                    scale += 0.5;
                }
                let tp = *typer_progress.lock();
                let src = Rectangle::new(
                    0.,
                    0.,
                    goner.width as _,
                    (goner.height as f32 * tp).floor()
                );
                let dest = Rectangle::new(
                    (320. - (goner.width as f32 * 0.5 * scale)).floor(),
                    (480. * (7. / 10.) - (goner.height as f32 * (0.5 * scale))).floor(),
                    (goner.width as f32 * scale).floor(),
                    (goner.height as f32 * tp * scale).floor()
                );
                //println!("{scale}, {}, {}, {src:?}, {dest:?}", goner.width, goner.height);
                d.draw_texture_pro(
                    goner,
                    src,
                    dest,
                    Vector2::zero(),
                    0.,
                    Color::WHITE.alpha(goner_alpha.clamp(0., 1.))
                );
            }
            #[cfg(feature = "uninstaller")]
            if shown_friend {
                d.draw_texture_pro(
                    &img_friend,
                    Rectangle::new(
                        0.,
                        0.,
                        img_friend.width as _,
                        img_friend.height as _
                    ),
                    Rectangle::new(
                        320.,
                        240.,
                        img_friend.width as f32 * 2. * friend_scale,
                        img_friend.height as f32 * 2. * friend_scale
                    ),
                    Vector2::new(
                        img_friend.width as f32 * friend_scale,
                        img_friend.height as f32 * friend_scale),
                        0.,
                        Color::WHITE
                    );
                friend_scale += delta;
            }
        });
        d.clear_background(Color::BLACK);
        d.draw_texture_pro(
            &render,
            Rectangle::new(
                0.,
                0.,
                render.texture.width as _,
                -render.texture.height as _,
            ),
            Rectangle::new(0., 0., 640. * scale as f32, 480. * scale as f32),
            Vector2::zero(),
            0.,
            Color::WHITE,
        );
        //d.draw_fps(20, 20);
    }
}
