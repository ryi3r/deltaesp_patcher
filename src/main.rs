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

use clap::Parser;
use ftail::Ftail;
use log::{LevelFilter, debug, info, warn};
#[allow(unused_imports)]
use oxidelta::compress::{CompressOptions, decoder, encoder};
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
use sola_raylib::{camera::Camera2D, drawing::RaylibMode2DExt};

use crate::{
    image_depth_renderer::ImageDepthRenderer, image_soul_blur_renderer::ImageSoulBlurRenderer,
    typer::Typer,
};

pub mod image_depth_renderer;
pub mod image_soul_blur_renderer;
pub mod typer;

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum DeltaruneOsKind {
    #[default]
    Windows,
    Mac,
    Switch,
}

#[derive(Debug, Clone)]
pub struct DeltarunePath {
    pub path: PathBuf,
    pub os: DeltaruneOsKind,
}

fn transform_deltarune_path(pb: &Path) -> Option<DeltarunePath> {
    let mut drp = DeltarunePath {
        path: pb.to_path_buf(),
        os: DeltaruneOsKind::default(),
    };
    if drp.path.is_file() {
        drp.path.pop(); // DELTARUNE.exe (and other) -> folder path
    }
    if !drp.path.is_dir() {
        warn!("{:?} isn't a directory?", drp.path);
    }
    {
        let dir = drp.path.file_name().unwrap().to_string_lossy();
        if dir == "mus"
            || dir == "English.lproj" || dir == "__MACOSX" // mac only
            // likely a chapter path, will work for fake paths like: chapter1AAAAA_mac, this will get rectified later on
            || (dir.starts_with("chapter")
                && (dir.ends_with("_mac") || dir.ends_with("_windows") || dir.ends_with("_switch"))
                && dir
                    .chars()
                    .nth("chapter".chars().count())
                    .map(|x| x.is_ascii_digit())
                    .unwrap_or(false))
        {
            if dir.ends_with("_mac") {
                drp.os = DeltaruneOsKind::Mac;
            } else if dir.ends_with("_switch") {
                drp.os = DeltaruneOsKind::Switch;
            }
            drp.path.pop();
        }
    }
    // switch check!
    {
        let mut p = drp.path.clone();
        p.push("game.win");
        if p.exists() && p.is_file() {
            drp.os = DeltaruneOsKind::Switch;
        }
        p.pop();
        p.push("chapter1_switch");
        if p.exists() && p.is_dir() {
            drp.os = DeltaruneOsKind::Switch;
        }
        p.pop();
        p.push("preselected_user");
        if p.exists() && p.is_dir() {
            drp.os = DeltaruneOsKind::Switch;
        }
        p.pop();
    }
    // mac check!
    {
        let mut p = drp.path.clone();
        p.push("__MACOSX");
        if p.exists() && p.is_dir() {
            drp.os = DeltaruneOsKind::Mac;
        }
        p.pop();
        p.push("DELTARUNE.app");
        if p.exists() && p.is_dir() {
            drp.os = DeltaruneOsKind::Mac;
            drp.path = p.clone();
        } else {
            p.pop();
        }
        p.push("Contents");
        if p.exists() && p.is_dir() {
            drp.os = DeltaruneOsKind::Mac;
            drp.path = p.clone();
        } else {
            p.pop();
        }
        p.push("MacOS");
        if p.exists() && p.is_dir() {
            drp.os = DeltaruneOsKind::Mac;
        }
        p.pop();
        p.push("Resources");
        if p.exists() && p.is_dir() {
            drp.os = DeltaruneOsKind::Mac;
            drp.path = p.clone();
        } else {
            p.pop();
        }
        p.push("English.lproj");
        if p.exists() && p.is_dir() {
            drp.os = DeltaruneOsKind::Mac;
        }
        p.pop();
        p.push("game.ios");
        if p.exists() && p.is_file() {
            drp.os = DeltaruneOsKind::Mac;
        }
    }
    // target folder ACTUALLY has deltarune?
    {
        let folder_check = match drp.os {
            DeltaruneOsKind::Windows => "windows",
            DeltaruneOsKind::Mac => "mac",
            DeltaruneOsKind::Switch => "switch",
        };
        let mut p = drp.path.clone();
        for ch in 0..=5 {
            if ch != 0 {
                p.push(format!("chapter{ch}_{folder_check}"));
            }
            if p.exists() && p.is_dir() {
                p.push(match drp.os {
                    DeltaruneOsKind::Windows => "data.win",
                    DeltaruneOsKind::Mac => "game.ios",
                    DeltaruneOsKind::Switch => "game.win",
                });
                if cfg!(feature = "uninstaller") && !p.add_extension("bak") {
                    warn!("failed to add extension to file??");
                    return None; // what?
                }
                if p.exists() && p.is_file() {
                    p.pop();
                } else {
                    return None;
                }
                if ch != 0 {
                    p.pop();
                }
                continue;
            }
            return None;
        }
        p.push("mus");
        if p.exists() && p.is_dir() {
            p.pop(); // maybe do other checks?
        } else {
            return None;
        }
    }
    Some(drp)
}

#[test]
fn test_win_path() {
    use std::str::FromStr;

    let mut p = PathBuf::from_str("/home/ryi3r/Desktop/DELTARUNE WIN BAK/").unwrap();
    let drp = transform_deltarune_path(&p).unwrap();
    assert_eq!(drp.os, DeltaruneOsKind::Windows);
    assert_eq!(drp.path, p);

    p.push("chapter5_windows");
    let drp = transform_deltarune_path(&p.clone()).unwrap();
    p.pop();
    assert_eq!(drp.os, DeltaruneOsKind::Windows);
    assert_eq!(drp.path, p);

    p.push("mus");
    let drp = transform_deltarune_path(&p.clone()).unwrap();
    p.pop();
    assert_eq!(drp.os, DeltaruneOsKind::Windows);
    assert_eq!(drp.path, p);

    p.push("DELTARUNE.exe");
    let drp = transform_deltarune_path(&p.clone()).unwrap();
    p.pop();
    assert_eq!(drp.os, DeltaruneOsKind::Windows);
    assert_eq!(drp.path, p);
}

#[test]
fn test_mac_path() {
    use std::str::FromStr;

    let mut p = PathBuf::from_str("/home/ryi3r/Desktop/DELTARUNE MAC BAK/").unwrap();
    let base_p = p.clone();
    let drp = transform_deltarune_path(&p.clone()).unwrap(); // triggers all path changes
    assert_eq!(drp.os, DeltaruneOsKind::Mac);
    p.push("DELTARUNE.app/Contents/Resources/");
    assert_eq!(drp.path, p);

    p = base_p;
    p.push("__MACOSX");
    let drp = transform_deltarune_path(&p.clone()).unwrap();
    assert_eq!(drp.os, DeltaruneOsKind::Mac);
    p.pop();
    p.push("DELTARUNE.app/Contents/Resources/");
    assert_eq!(drp.path, p);

    p.push("mus");
    let drp = transform_deltarune_path(&p.clone()).unwrap();
    p.pop();
    assert_eq!(drp.os, DeltaruneOsKind::Mac);
    assert_eq!(drp.path, p);

    p.push("game.ios");
    let drp = transform_deltarune_path(&p.clone()).unwrap();
    p.pop();
    assert_eq!(drp.os, DeltaruneOsKind::Mac);
    assert_eq!(drp.path, p);
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

#[derive(clap::Parser, Debug, Clone)]
#[command(version, about = "DeltaESP's Deltarune patcher, developed by ryi3r.", long_about = None)]
pub struct Args {
    /// This disables (or not) the GUI.
    #[arg(short, long, default_value = "false")]
    pub cli: bool,

    /// The folder/directory of the patch, which will be used to patch the game itself.
    #[arg(short, long, default_value = None)]
    pub patch_path: Option<PathBuf>,
    /// The folder/directory of the game.
    #[arg(short, long, default_value = None)]
    pub game_path: Option<PathBuf>,

    #[arg(short, long, default_value = "false")]
    pub no_question_gui: bool,
    #[arg(short, long, default_value = "false")]
    pub skip_outro_gui: bool,
}

fn main() {
    Ftail::new()
        .console(LevelFilter::Debug)
        .single_file(&PathBuf::from("log.txt"), false, LevelFilter::Debug)
        .init()
        .unwrap();
    #[cfg(target_os = "windows")]
    unsafe {
        use windows::Win32::System::Console::{
            ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_ERROR_HANDLE,
            STD_OUTPUT_HANDLE, SetStdHandle,
        };

        if AttachConsole(ATTACH_PARENT_PROCESS).is_ok() {
            let handle = GetStdHandle(STD_OUTPUT_HANDLE).unwrap();
            SetStdHandle(STD_OUTPUT_HANDLE, handle).unwrap();
            let err_handle = GetStdHandle(STD_ERROR_HANDLE).unwrap();
            SetStdHandle(STD_ERROR_HANDLE, err_handle).unwrap();
        }
    }

    let args = Args::parse();
    let files = if args.cli {
        HashMap::default()
    } else {
        postcard::from_bytes::<HashMap<String, Vec<u8>>>(&fs::read("data.bin").unwrap()).unwrap()
    };
    let (mut rl, thread) = sola_raylib::init()
        .hidden()
        .size(640, 480)
        .title("DeltaESP Patcher")
        .vsync()
        .build();
    rl.set_trace_log(TraceLogLevel::LOG_WARNING);
    rl.set_exit_key(None);
    let mut scale = 1;
    if !args.cli {
        rl.clear_window_state(WindowState::default().set_window_hidden(true));
        let cm = window::get_current_monitor();
        let w = window::get_monitor_width(cm);
        let h = window::get_monitor_height(cm);
        while w > 640 * (scale + 1) && h > 480 * (scale + 1) {
            scale += 1;
        }
        rl.set_window_size(640 * scale, 480 * scale);
        //debug!("{:?}, {:?}, {:?}", ((w - rl.get_screen_width()) / 2, (h - rl.get_screen_height()) / 2), (w, h), (rl.get_screen_width(), rl.get_screen_height()));
        rl.set_window_position(
            (w - rl.get_screen_width()) / 2,
            (h - rl.get_screen_height()) / 2,
        );

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
    }
    let audio = RaylibAudio::init_audio_device().unwrap();
    let camera = Camera2D {
        zoom: scale as _,
        ..Default::default()
    };
    let fnt = if files.is_empty() {
        None
    } else {
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
        .into()
    };
    let img_depth_tex = if files.is_empty() {
        None
    } else {
        rl.load_texture_from_image(
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
        .unwrap()
        .into()
    };
    let timer = Instant::now();
    let mut idr = Vec::new();
    let mut idr_time = Instant::now();

    let typer_progress = Arc::new(Mutex::new(0f32));
    let progress = Arc::new(Mutex::new(String::new()));
    let deltarune_path = Arc::new(Mutex::new(None));
    let mut folder = PathBuf::new();

    if let Ok(sl) = steamlocate::locate_all() {
        for lib in sl {
            if let Ok(Some(a)) = lib.find_app(1_671_210) {
                // deltarune's steam id
                folder = a.1.resolve_app_dir(&a.0);
                debug!("UPDATE FOLDER");
            }
        }
    }
    if let Some(p) = &args.game_path {
        folder = p.clone();
    }

    if args.cli {
        if let Some(mp) = transform_deltarune_path(&folder) {
            *deltarune_path.lock() = Some(mp);
        } else {
            panic!("DELTARUNE was not found.");
        }
    }
    let mut state = if args.cli {
        #[cfg(not(feature = "uninstaller"))]
        {
            PatcherState::Patch
        }
        #[cfg(feature = "uninstaller")]
        {
            PatcherState::Uninstalling
        }
    } else {
        PatcherState::Init
    };
    let img_soul_blur_tex = if files.is_empty() {
        None
    } else {
        rl.load_texture_from_image(
            &thread,
            &Image::load_image_from_mem(".png", &files["image_soul_blur.png"]).unwrap(),
        )
        .unwrap()
        .into()
    };
    #[allow(unused)]
    let mut soul = ImageSoulBlurRenderer::new(img_soul_blur_tex);
    #[allow(unused)]
    let aud_appearance = if files.is_empty() {
        None
    } else {
        audio
            .new_sound_from_wave(
                &audio
                    .new_wave_from_memory(".wav", &files["audio_appearance.wav"])
                    .unwrap(),
            )
            .unwrap()
            .into()
    };
    let aud_another_him = if files.is_empty() {
        None
    } else {
        audio
            .new_sound_from_wave(
                &audio
                    .new_wave_from_memory(".ogg", &files["audio_anotherhim.ogg"])
                    .unwrap(),
            )
            .unwrap()
            .into()
    };
    #[cfg(feature = "uninstaller")]
    let aud_face = if files.is_empty() {
        None
    } else {
        audio
            .new_sound_from_wave(
                &audio
                    .new_wave_from_memory(".ogg", &files["face.ogg"])
                    .unwrap(),
            )
            .unwrap()
            .into()
    };
    #[allow(unused)]
    let mut play_another_him = cfg!(feature = "uninstaller");
    #[allow(unused)]
    let mut pitch_another_him = if cfg!(feature = "uninstaller") {
        0.001
    } else {
        0.2
    };
    let mut typer = if let Some(fnt) = fnt {
        Typer {
            timer: Instant::now(),
            text: String::new(),
            pos: Vector2::new(320., 220.),
            playing: false,
            chars: 0,
            delay: HashMap::new(),
            instant_show: false,
            increase_typer: true,
            fnt,
        }
        .into()
    } else {
        None
    };
    #[cfg(feature = "uninstaller")]
    if let Some(typer) = &mut typer {
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
    let img_friend = if files.is_empty() {
        None
    } else {
        rl.load_texture_from_image(
            &thread,
            &Image::load_image_from_mem(".png", &files["friend.png"]).unwrap(),
        )
        .unwrap()
        .into()
    };
    #[cfg(feature = "uninstaller")]
    let mut friend_scale = 1f32;

    //state = PatcherState::CloseTimed;
    //galpha = 2.;

    //let mut cl = 0;
    while !rl.window_should_close() && !close {
        //debug!("FRAME: {state:?} {galpha:.2} {inst_alpha:.2}");
        let mut d = rl.begin_drawing(&thread);
        let delta = d.get_frame_time();
        d.clear_background(Color::BLACK);
        {
            let mut d = d.begin_mode2D(camera);
            if let Some(typer) = &typer
                && typer.playing
            {
                if skipping {
                    if falpha > 0. {
                        falpha = (falpha - delta).max(0.);
                    }
                } else if falpha < 1. {
                    falpha = (falpha + delta).min(1.);
                }
            }
            #[cfg(not(feature = "uninstaller"))]
            if (d.is_key_pressed(KeyboardKey::KEY_C)
                || d.is_key_pressed(KeyboardKey::KEY_LEFT_CONTROL)
                || d.is_key_pressed(KeyboardKey::KEY_RIGHT_CONTROL))
                && state < PatcherState::SoulExit
            {
                skipped = true;
                state = PatcherState::SoulExit;
                idr_time = Instant::now();
                play_another_him = true;
                if let Some(typer) = &mut typer {
                    typer.increase_typer = false;
                }
                galpha = 0.;
                skipping = true;
                if soul.time >= soul.get_max_time() {
                    soul.momentum = -0.5;
                    soul.time = soul.get_max_time();
                    if let Some(aud) = &aud_appearance {
                        aud.play();
                    }
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
                        inst_alpha = (inst_alpha
                            + (delta * (if is_skip_pressed { 2. } else { 1. })))
                        .min(1.);
                    }
                    if galpha <= -1. && inst_alpha >= 1. {
                        state = PatcherState::StartTalkA;
                    }
                }
                PatcherState::StartTalkA => {
                    if let Some(typer) = &mut typer {
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
                }
                PatcherState::StartTalkB => {
                    if let Some(typer) = &mut typer {
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
                                    if let Some(aud) = &aud_appearance {
                                        aud.play();
                                    }
                                }
                                #[cfg(feature = "uninstaller")]
                                {
                                    state = if let Some(drp) = transform_deltarune_path(&folder) {
                                        *deltarune_path.lock() = Some(drp);
                                        PatcherState::UninstallPrompt
                                    } else {
                                        folder.clear();
                                        deltarune_path.lock().take();
                                        debug!("CLEAR FOLDER (2)");
                                        PatcherState::FindFolder
                                    }
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
                    if let Some(typer) = &mut typer {
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
                }
                #[cfg(not(feature = "uninstaller"))]
                PatcherState::StartTalkD => {
                    if let Some(typer) = &mut typer {
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
                                if let Some(aud) = &aud_appearance {
                                    aud.play();
                                }
                            }
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
                        pitch_another_him =
                            (pitch_another_him + 0.02 * (delta / (1. / 30.))).min(0.96);
                    }
                    if let Some(typer) = &mut typer
                        && galpha >= 1.
                        && pitch_another_him >= 0.96
                        && inst_alpha <= 0.
                    {
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
                    if let Some(typer) = &mut typer {
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
                }
                #[cfg(not(feature = "uninstaller"))]
                PatcherState::StartPatchQuestion => {
                    if let Some(typer) = &mut typer {
                        if !args.no_question_gui && !typer.playing {
                            typer.start(
                                "¿DESEAS INICIAR^^10\nLA CONVERSIÓN^^10\nDE FORMA AUTOMÁTICA?",
                            );
                            galpha = 2.;
                            qoption = 0;
                            qalpha = 0.;
                            qanswered = false;
                        } else {
                            if !args.no_question_gui && !qanswered {
                                qalpha = (qalpha + delta).min(1.);
                                if d.is_key_down(KeyboardKey::KEY_Z)
                                    || d.is_key_down(KeyboardKey::KEY_ENTER)
                                    || d.is_key_down(KeyboardKey::KEY_KP_ENTER)
                                {
                                    qanswered = true;
                                }
                            } else {
                                qalpha = (qalpha - delta).max(0.);
                            }
                            if args.no_question_gui || (typer.is_finished() && qanswered) {
                                if !args.no_question_gui && galpha > -1. {
                                    galpha -= delta * (if is_skip_pressed { 2. } else { 1. });
                                } else {
                                    typer.stop();
                                    state = if qoption == 0
                                        && let Some(mp) = transform_deltarune_path(&folder)
                                    {
                                        *deltarune_path.lock() = Some(mp);
                                        PatcherState::Patch
                                    } else {
                                        debug!("CLEAR FOLDER (1)");
                                        PatcherState::RefuseStartPatch
                                    };
                                }
                            }
                        }
                    }
                }
                #[cfg(not(feature = "uninstaller"))]
                PatcherState::RefuseStartPatch => {
                    if let Some(typer) = &mut typer {
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
                }
                PatcherState::FindFolder => {
                    let mut lock = deltarune_path.lock();
                    if d.is_file_dropped() {
                        for f in d.load_dropped_files().paths() {
                            if f.is_empty() {
                                continue;
                            }
                            let f = PathBuf::from(f);
                            if f.exists() {
                                *lock = transform_deltarune_path(&f)
                                    .unwrap_or(DeltarunePath {
                                        path: PathBuf::new(),
                                        os: DeltaruneOsKind::default(),
                                    })
                                    .into();
                                debug!("UPDATE FOLDER");
                            }
                        }
                    }
                    if let Some(typer) = &mut typer {
                        if !typer.playing {
                            galpha = 2.;
                            typer.pos.y = 100.;
                            typer.start("ARRASTRA LA CARPETA\nDE DELTARUNE^^10\nA ESTA VENTANA.");
                        } else if typer.is_finished()
                            && let Some(data) = lock.as_ref()
                        {
                            if galpha > -1. {
                                galpha -= delta * (if is_skip_pressed { 2. } else { 1. });
                            } else {
                                typer.stop();
                                state = if !data.path.is_empty() {
                                    PatcherState::DeltaruneFound
                                } else {
                                    PatcherState::DeltaruneNotFound
                                };
                            }
                        }
                    }
                }
                PatcherState::DeltaruneNotFound => {
                    deltarune_path.lock().take();
                    if let Some(typer) = &mut typer {
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
                }
                PatcherState::DeltaruneFound => {
                    if let Some(typer) = &mut typer {
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
                }
                #[cfg(feature = "uninstaller")]
                PatcherState::UninstallPrompt => {
                    if let Some(typer) = &mut typer {
                        if !args.no_question_gui && !typer.playing {
                            typer.start("¿COMENZAR^^10\nLA ESCISIÓN?");
                            galpha = 2.;
                            qoption = 0;
                            qalpha = 0.;
                            qanswered = false;
                        } else {
                            if !args.no_question_gui && !qanswered {
                                qalpha = (qalpha + delta).min(1.);
                                if d.is_key_down(KeyboardKey::KEY_Z)
                                    || d.is_key_down(KeyboardKey::KEY_ENTER)
                                    || d.is_key_down(KeyboardKey::KEY_KP_ENTER)
                                {
                                    qanswered = true;
                                }
                            } else {
                                qalpha = (qalpha - delta).max(0.);
                            }
                            if args.no_question_gui || (typer.is_finished() && qanswered) {
                                if !args.no_question_gui && galpha > -1. {
                                    galpha -= delta * (if is_skip_pressed { 2. } else { 1. });
                                } else {
                                    typer.stop();
                                    state = if qoption == 0 {
                                        PatcherState::Uninstalling
                                    } else {
                                        deltarune_path.lock().take();
                                        PatcherState::EndKeptTalkA
                                    };
                                }
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
                            let deltarune_path = deltarune_path.lock().clone();
                            let typer_progress = typer_progress.clone();
                            let args = args.clone();
                            patch_task = Some(std::thread::spawn(move || {
                                *progress.lock() = "Desinstalando parche...".into();
                                let deltarune_path = deltarune_path.unwrap(); // just in case do it here lol
                                let init_path = if let Some(p) = &args.patch_path {
                                    p.clone()
                                } else {
                                    let mut p = std::env::current_dir().unwrap();
                                    p.push("patch");
                                    p
                                };
                                let mut dirs = vec![init_path];
                                let comp_count = dirs[0].components().count();
                                let mut count = 0usize;
                                {
                                    let mut dirs = dirs.clone();
                                    while let Some(dir) = dirs.pop() {
                                        if let Ok(d) = fs::read_dir(dir) {
                                            for f in d.flatten() {
                                                let Ok(ft) = f.file_type() else {
                                                    continue;
                                                };
                                                if ft.is_dir() {
                                                    let dir = f.path();
                                                    let fname = dir
                                                        .file_name()
                                                        .unwrap_or_default()
                                                        .to_string_lossy();
                                                    if fname != "mac" && fname != "switch" {
                                                        dirs.push(dir);
                                                    }
                                                    continue;
                                                }
                                                count += 1;
                                            }
                                        }
                                    }
                                }
                                debug!("count! {count}");
                                let mut done = 0usize;
                                while let Some(dir) = dirs.pop() {
                                    if let Ok(d) = fs::read_dir(&dir) {
                                        for f in d.flatten() {
                                            let Ok(ft) = f.file_type() else {
                                                continue;
                                            };
                                            let p = f.path();
                                            let fname =
                                                p.file_name().unwrap_or_default().to_string_lossy();
                                            if ft.is_dir() && fname != "mac" && fname != "switch" {
                                                dirs.push(p);
                                            }
                                        }
                                    }
                                    // ensure that the folder exists in the output destination...
                                    let _ = fs::create_dir_all(&dir);
                                    if let Ok(d) = fs::read_dir(&dir) {
                                        for f in d.flatten() {
                                            let Ok(ft) = f.file_type() else {
                                                continue;
                                            };
                                            if !ft.is_file() {
                                                continue;
                                            }
                                            let mut show_path = f
                                                .path()
                                                .components()
                                                .skip(comp_count)
                                                .collect::<PathBuf>()
                                                .to_string_lossy()
                                                .to_string()
                                                .replace(".xdelta", "");
                                            info!("processing: {show_path}");
                                            if show_path.len() > 20 {
                                                show_path =
                                                    show_path[(show_path.len() - 20)..].to_string();
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

                                            let is_xdelta = f
                                                .path()
                                                .extension()
                                                .map(|ext| ext == "xdelta")
                                                .unwrap_or(false);
                                            let mut spath = deltarune_path.path.clone();
                                            {
                                                let mut p = f.path();
                                                let diff = p.parent().unwrap().components().count()
                                                    - comp_count;
                                                let os_suffix = match deltarune_path.os {
                                                    DeltaruneOsKind::Windows => "windows",
                                                    DeltaruneOsKind::Mac => "mac",
                                                    DeltaruneOsKind::Switch => "switch",
                                                };
                                                if diff == 0 && is_xdelta {
                                                    let fpr =
                                                        p.file_prefix().unwrap().to_str().unwrap();
                                                    let game_data_target = match deltarune_path.os {
                                                        DeltaruneOsKind::Windows => "data.win",
                                                        DeltaruneOsKind::Mac => "game.ios",
                                                        DeltaruneOsKind::Switch => "game.win",
                                                    };
                                                    if fpr == "launcher" {
                                                        spath.push(game_data_target);
                                                    } else if fpr.len() == ("chapter".len() + 1)
                                                        && fpr.starts_with("chapter")
                                                        && let Some(ch) =
                                                            fpr.chars().nth("chapter".len())
                                                        && ch.is_ascii_digit()
                                                    {
                                                        spath.push(format!("{fpr}_{os_suffix}"));
                                                        spath.push(game_data_target);
                                                    }
                                                } else {
                                                    if is_xdelta {
                                                        p.set_extension("");
                                                    }
                                                    let mut cmps = p
                                                        .components()
                                                        .skip(comp_count)
                                                        .collect::<Vec<_>>();
                                                    // folder + filename
                                                    if cmps.len() > 1 {
                                                        let fpr =
                                                            cmps[0].as_os_str().to_str().unwrap();
                                                        if fpr.len() == ("chapter".len() + 1)
                                                            && fpr.starts_with("chapter")
                                                            && let Some(ch) =
                                                                fpr.chars().nth("chapter".len())
                                                            && ch.is_ascii_digit()
                                                        {
                                                            cmps.remove(0);
                                                            spath
                                                                .push(format!("{fpr}_{os_suffix}"));
                                                        }
                                                    }
                                                    spath.extend(cmps);
                                                }
                                            }
                                            let bpath = spath.with_added_extension("bak");
                                            if bpath.exists() && bpath.is_file() {
                                                if fs::metadata(&bpath).unwrap().len() > 0 {
                                                    if let Ok(patched) = decoder::decode_all(
                                                        &fs::read(&spath).unwrap(),
                                                        &fs::read(&bpath).unwrap(),
                                                    ) {
                                                        fs::write(spath, &patched).unwrap();
                                                    } else {
                                                        warn!(
                                                            "unable to apply backup patch! was it an update?"
                                                        );
                                                    }
                                                    //fs::copy(&bpath, &spath).unwrap();
                                                } else {
                                                    fs::remove_file(&spath).unwrap();
                                                }
                                                fs::remove_file(&bpath).unwrap();
                                                //debug!("spath:{spath:?},bpath:{bpath:?}");
                                            }

                                            done += 1;
                                            let tp = *typer_progress.lock();
                                            *typer_progress.lock() =
                                                tp.max(done as f32 / count as f32);
                                        }
                                    }
                                }
                                *progress.lock() = "Finalizado.".into();
                                *typer_progress.lock() = 1.;
                            }));
                        }
                    }
                    if let Some(typer) = &mut typer
                        && !typer.playing
                    {
                        galpha = 2.;
                        goner_alpha = galpha;
                        typer.start("LA ESCISIÓN^^10\nESTÁ TOMANDO LUGAR.");
                    } else if patch_task
                        .as_ref()
                        .map(|x| x.is_finished())
                        .unwrap_or(false)
                        && (args.cli || typer.as_ref().map(|x| x.is_finished()).unwrap_or(false))
                    {
                        if galpha > -1. {
                            galpha -= delta * (if is_skip_pressed { 2. } else { 1. });
                        } else {
                            *progress.lock() = String::new();
                            if let Some(typer) = &mut typer {
                                typer.stop();
                            }
                            if let Some(pt) = patch_task.take()
                                && let Err(e) = pt.join()
                                && !args.cli
                            {
                                let mut f =
                                    BufWriter::new(File::create("uninstalling_error.txt").unwrap());
                                f.write_all(format!("{e:?}").as_bytes()).unwrap();
                                f.flush().unwrap();
                            }
                            state = PatcherState::EndUninstalledTalkA;
                            if args.cli {
                                break;
                            }
                        }
                    }
                }
                #[cfg(not(feature = "uninstaller"))]
                PatcherState::Patch => {
                    if patch_task.is_none() {
                        if !files.is_empty() {
                            let f = files
                                .keys()
                                .filter(|x| x.starts_with("goners/"))
                                .collect::<Vec<_>>();
                            goner = Some(
                                d.load_texture_from_image(
                                    &thread,
                                    &Image::load_image_from_mem(
                                        ".png",
                                        &files[f[rand::random_range(0..f.len())]],
                                    )
                                    .unwrap(),
                                )
                                .unwrap(),
                            );
                        }
                        *typer_progress.lock() = 0.;
                        *progress.lock() = String::new();
                        {
                            let progress = progress.clone();
                            let deltarune_path = deltarune_path.lock().clone();
                            let typer_progress = typer_progress.clone();
                            let args = args.clone();
                            patch_task = Some(std::thread::spawn(move || {
                                *progress.lock() = "Inicializando parche...".into();
                                let deltarune_path = deltarune_path.unwrap(); // just in case do it here lol
                                let init_path = if let Some(p) = &args.patch_path {
                                    p.clone()
                                } else {
                                    let mut p = std::env::current_dir().unwrap();
                                    p.push("patch");
                                    p
                                };
                                let mut dirs = vec![init_path.clone()];
                                let comp_count = dirs[0].components().count();
                                let mut count = 0usize;
                                {
                                    let mut dirs = dirs.clone();
                                    while let Some(dir) = dirs.pop() {
                                        if let Ok(d) = fs::read_dir(dir) {
                                            for f in d.flatten() {
                                                let Ok(ft) = f.file_type() else {
                                                    continue;
                                                };
                                                if ft.is_dir() {
                                                    let dir = f.path();
                                                    let fname = dir
                                                        .file_name()
                                                        .unwrap_or_default()
                                                        .to_string_lossy();
                                                    if fname != "mac" && fname != "switch" {
                                                        dirs.push(dir);
                                                    }
                                                    continue;
                                                }
                                                count += 1;
                                            }
                                        }
                                    }
                                }
                                info!("count! {count}");
                                let mut done = 0usize;
                                while let Some(dir) = dirs.pop() {
                                    if let Ok(d) = fs::read_dir(&dir) {
                                        for f in d.flatten() {
                                            let Ok(ft) = f.file_type() else {
                                                continue;
                                            };
                                            let p = f.path();
                                            let fname =
                                                p.file_name().unwrap_or_default().to_string_lossy();
                                            if ft.is_dir() && fname != "mac" && fname != "switch" {
                                                dirs.push(p);
                                            }
                                        }
                                    }
                                    // ensure that the folder exists in the output destination...
                                    let _ = fs::create_dir_all(&dir);
                                    if let Ok(d) = fs::read_dir(&dir) {
                                        for f in d.flatten() {
                                            let Ok(ft) = f.file_type() else {
                                                continue;
                                            };
                                            if !ft.is_file() {
                                                continue;
                                            }
                                            let mut show_path = f
                                                .path()
                                                .components()
                                                .skip(comp_count)
                                                .collect::<PathBuf>()
                                                .to_string_lossy()
                                                .to_string()
                                                .replace(".xdelta", "");
                                            info!("processing: {show_path}");
                                            if show_path.len() > 20 {
                                                show_path =
                                                    show_path[(show_path.len() - 20)..].to_string();
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

                                            let is_xdelta = f
                                                .path()
                                                .extension()
                                                .map(|ext| ext == "xdelta")
                                                .unwrap_or(false);
                                            let mut spath = deltarune_path.path.clone();
                                            let mut mac_patch = None;
                                            {
                                                let mut p = f.path();
                                                let diff = p.parent().unwrap().components().count()
                                                    - comp_count;
                                                let os_suffix = match deltarune_path.os {
                                                    DeltaruneOsKind::Windows => "windows",
                                                    DeltaruneOsKind::Mac => "mac",
                                                    DeltaruneOsKind::Switch => "switch",
                                                };
                                                if diff == 0 && is_xdelta {
                                                    let fpr =
                                                        p.file_prefix().unwrap().to_str().unwrap();
                                                    let game_data_target = match deltarune_path.os {
                                                        DeltaruneOsKind::Windows => "data.win",
                                                        DeltaruneOsKind::Mac => "game.ios",
                                                        DeltaruneOsKind::Switch => "game.win",
                                                    };
                                                    let mut valid = true;
                                                    if fpr == "launcher" {
                                                        spath.push(game_data_target);
                                                    } else if fpr.len() == ("chapter".len() + 1)
                                                        && fpr.starts_with("chapter")
                                                        && let Some(ch) =
                                                            fpr.chars().nth("chapter".len())
                                                        && ch.is_ascii_digit()
                                                    {
                                                        spath.push(format!("{fpr}_{os_suffix}"));
                                                        spath.push(game_data_target);
                                                    } else {
                                                        valid = false;
                                                    }
                                                    if valid
                                                        && (deltarune_path.os
                                                            == DeltaruneOsKind::Mac
                                                            || deltarune_path.os
                                                                == DeltaruneOsKind::Switch)
                                                    {
                                                        let mut np = p.clone();
                                                        let fln =
                                                            np.file_name().unwrap().to_os_string();
                                                        np.pop();
                                                        np.push(
                                                            if deltarune_path.os
                                                                == DeltaruneOsKind::Mac
                                                            {
                                                                "mac"
                                                            } else {
                                                                "switch"
                                                            },
                                                        );
                                                        np.push(fln);
                                                        if np.exists() && np.is_file() {
                                                            mac_patch = Some(np);
                                                        }
                                                    }
                                                } else {
                                                    if is_xdelta {
                                                        p.set_extension("");
                                                    }
                                                    let mut cmps = p
                                                        .components()
                                                        .skip(comp_count)
                                                        .collect::<Vec<_>>();
                                                    // folder + filename
                                                    if cmps.len() > 1 {
                                                        let fpr =
                                                            cmps[0].as_os_str().to_str().unwrap();
                                                        if fpr.len() == ("chapter".len() + 1)
                                                            && fpr.starts_with("chapter")
                                                            && let Some(ch) =
                                                                fpr.chars().nth("chapter".len())
                                                            && ch.is_ascii_digit()
                                                        {
                                                            cmps.remove(0);
                                                            spath
                                                                .push(format!("{fpr}_{os_suffix}"));
                                                        }
                                                    }
                                                    spath.extend(cmps);
                                                }
                                            }
                                            let bpath = spath.with_added_extension("bak");
                                            //debug!("spath:{spath:?},bpath:{bpath:?}");
                                            if !is_xdelta {
                                                if spath.exists()
                                                    && if bpath.exists() {
                                                        fs::metadata(&bpath).unwrap().len() > 0
                                                    } else {
                                                        true
                                                    }
                                                {
                                                    let mut data = fs::read(&spath).unwrap();
                                                    if bpath.exists() {
                                                        info!("decoding backup xdelta");
                                                        *progress.lock() = format!(
                                                            "Decodificando el backup de {show_path}..."
                                                        );
                                                        if let Ok(d) = decoder::decode_all(
                                                            &data,
                                                            &fs::read(&bpath).unwrap(),
                                                        ) {
                                                            data = d;
                                                        } else {
                                                            warn!(
                                                                "backup decode failed! was it an update?"
                                                            );
                                                        }
                                                    }
                                                    info!("making xdelta for backup");
                                                    *progress.lock() = format!(
                                                        "Haciendo backup de {show_path}..."
                                                    );
                                                    let mut fb = BufWriter::new(
                                                        File::create(&bpath).unwrap(),
                                                    );
                                                    encoder::encode_all(
                                                        &mut fb,
                                                        &fs::read(f.path()).unwrap(),
                                                        &data,
                                                        CompressOptions::default(),
                                                    )
                                                    .unwrap();
                                                    fb.flush().unwrap();
                                                    //fs::copy(&spath, &bpath).unwrap();
                                                } else {
                                                    fs::write(&bpath, []).unwrap();
                                                }

                                                *progress.lock() =
                                                    format!("Copiando archivo {show_path}");
                                                //debug!("{:?},{spath:?}", f.path());
                                                fs::copy(f.path(), &spath).unwrap();
                                                *progress.lock() = format!("{show_path} OK");
                                                info!("copied");
                                            } else {
                                                let mut read = fs::read(&spath).unwrap();
                                                if bpath.exists() && bpath.is_file() {
                                                    *progress.lock() = format!(
                                                        "Decodificando el backup de {show_path}..."
                                                    );
                                                    if let Ok(r) = decoder::decode_all(
                                                        &read,
                                                        &fs::read(&bpath).unwrap(),
                                                    ) {
                                                        read = r;
                                                    } else {
                                                        warn!(
                                                            "backup decode failed! was it an update?"
                                                        );
                                                    }
                                                }
                                                //info!("og:{spath:?},patch:{:?}", f.path());
                                                *progress.lock() = format!(
                                                    "Aplicando el parche para {show_path}..."
                                                );
                                                let mut dec = read.clone();
                                                if let Some(mac_patch) = mac_patch {
                                                    dec = decoder::decode_all(
                                                        &dec,
                                                        &fs::read(mac_patch).unwrap(),
                                                    )
                                                    .unwrap();
                                                }
                                                dec = decoder::decode_all(
                                                    &dec,
                                                    &fs::read(f.path()).unwrap(),
                                                )
                                                .unwrap();

                                                info!("making xdelta for backup");
                                                *progress.lock() =
                                                    format!("Realizando backup de {show_path}...");
                                                {
                                                    //fs::write(bpath.with_added_extension("source"), &dec).unwrap();
                                                    //fs::write(bpath.with_added_extension("target"), &read).unwrap();
                                                    let mut fb = BufWriter::new(
                                                        File::create(&bpath).unwrap(),
                                                    );
                                                    encoder::encode_all(
                                                        &mut fb,
                                                        &dec,
                                                        &read,
                                                        CompressOptions {
                                                            level: 3,
                                                            ..Default::default()
                                                        },
                                                    )
                                                    .unwrap();
                                                    fb.flush().unwrap();
                                                }

                                                {
                                                    let mut fw = BufWriter::new(
                                                        File::create(&spath).unwrap(),
                                                    );
                                                    fw.write_all(&dec).unwrap();
                                                    fw.flush().unwrap();
                                                }
                                                info!("wrote delta");

                                                *progress.lock() = format!("{show_path} OK");
                                            }

                                            done += 1;
                                            let tp = *typer_progress.lock();
                                            *typer_progress.lock() =
                                                tp.max(done as f32 / count as f32);
                                        }
                                    }
                                }
                                *progress.lock() = "Finalizado.".into();
                                *typer_progress.lock() = 1.;
                                info!("finished");
                            }));
                        }
                    }
                    if let Some(typer) = &mut typer
                        && !typer.playing
                    {
                        galpha = 2.;
                        goner_alpha = galpha;
                        typer.start("AHORA.^^10\nSE REALIZARÁ LA CONVERSIÓN.");
                    } else if patch_task
                        .as_ref()
                        .map(|x| x.is_finished())
                        .unwrap_or(false)
                        && (args.cli || typer.as_ref().map(|x| x.is_finished()).unwrap_or(false))
                    {
                        if !args.cli && galpha > -1. {
                            galpha -= delta * (if is_skip_pressed { 2. } else { 1. });
                        } else {
                            if let Some(typer) = &mut typer {
                                typer.stop();
                            }
                            let mut success = false;
                            if let Some(pt) = patch_task.take() {
                                if let Err(e) = pt.join()
                                    && !args.cli
                                {
                                    let mut f =
                                        BufWriter::new(File::create("patching_error.txt").unwrap());
                                    f.write_all(progress.lock().as_bytes()).unwrap();
                                    f.write_all("\n\n".as_bytes()).unwrap();
                                    f.write_all(format!("{e:?}").as_bytes()).unwrap();
                                    f.flush().unwrap();
                                } else {
                                    success = true;
                                }
                            }
                            *progress.lock() = String::new();
                            state = if success {
                                PatcherState::PatchSuccess
                            } else {
                                PatcherState::PatchFailed
                            };
                            if args.cli {
                                break;
                            }
                        }
                        if state != PatcherState::PatchFailed {
                            goner_alpha = galpha;
                        }
                    }
                }
                #[cfg(not(feature = "uninstaller"))]
                PatcherState::PatchFailed => {
                    let mut go_lower = true;
                    if let Some(typer) = &mut typer {
                        if !typer.playing {
                            goner = Some(
                                d.load_texture_from_image(
                                    &thread,
                                    &Image::load_image_from_mem(".png", &files["error.png"])
                                        .unwrap(),
                                )
                                .unwrap(),
                            );
                            *typer_progress.lock() = 1.;
                            galpha = 2.;
                            typer.start("LA CONVERSIÓN^5 HA FALLADO.^^10\nASEGÚRATE^5 DE TENER LOS^^10\nARCHIVOS ORIGINALES.");
                        } else if typer.is_finished() {
                            go_lower = false;
                            if galpha > -1. {
                                galpha -= delta * (if is_skip_pressed { 2. } else { 1. });
                            } else if pitch_another_him >= 0.96 {
                                typer.stop();
                                debug!("CLEAR FOLDER");
                                folder = PathBuf::new();
                                deltarune_path.lock().take();
                                state = PatcherState::FindFolder;
                            }
                            goner_alpha = galpha;
                        }
                    }

                    if go_lower {
                        if pitch_another_him > 0.02 {
                            pitch_another_him = (pitch_another_him
                                - (0.02 * (d.get_frame_time() / (1. / 30.))))
                                .max(0.02);
                        } else {
                            play_another_him = false;
                        }
                    } else {
                        if !play_another_him {
                            play_another_him = true;
                        }
                        if pitch_another_him < 0.96 {
                            pitch_another_him = (pitch_another_him
                                + (0.02 * (d.get_frame_time() / (1. / 30.))))
                                .min(0.96);
                        }
                    }
                }
                #[cfg(not(feature = "uninstaller"))]
                PatcherState::PatchSuccess => {
                    if let Some(typer) = &mut typer {
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
                }
                #[cfg(not(feature = "uninstaller"))]
                PatcherState::EndTalkA => {
                    if args.skip_outro_gui {
                        break;
                    } else if let Some(typer) = &mut typer {
                        if !typer.playing {
                            galpha = 2.;
                            typer.start(&format!(
                                "NOS VEMOS PRONTO^^10\n{}.",
                                whoami::username()
                                    .unwrap_or(String::from("JUGADOR"))
                                    .to_uppercase()
                            ));
                        } else if typer.is_finished() {
                            if galpha > -1. {
                                galpha -= delta * (if is_skip_pressed { 2. } else { 1. });
                            } else {
                                typer.stop();
                                state = PatcherState::CloseTimed;
                                galpha = 3.;
                            }
                        }
                    }

                    if pitch_another_him > 0.02 {
                        pitch_another_him = (pitch_another_him
                            - (0.02 * (d.get_frame_time() / (1. / 30.))))
                            .max(0.02);
                    } else {
                        play_another_him = false;
                    }
                }
                #[cfg(feature = "uninstaller")]
                PatcherState::EndUninstalledTalkA => {
                    if args.skip_outro_gui {
                        break;
                    } else if let Some(typer) = &mut typer {
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
                }
                #[cfg(feature = "uninstaller")]
                PatcherState::EndUninstalledTalkB => {
                    if let Some(typer) = &mut typer {
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
                }
                #[cfg(feature = "uninstaller")]
                PatcherState::EndKeptTalkA => {
                    if let Some(typer) = &mut typer {
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
                }
                #[cfg(feature = "uninstaller")]
                PatcherState::EndKeptTalkB => {
                    if let Some(typer) = &mut typer {
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
                }
                PatcherState::CloseTimed => {
                    if idr.is_empty() {
                        if galpha > 0. {
                            galpha -= d.get_frame_time();
                        } else {
                            #[cfg(feature = "uninstaller")]
                            if show_friend && !shown_friend {
                                shown_friend = true;
                                if let Some(aud) = &aud_face {
                                    aud.play();
                                }
                            } else if let Some(aud) = &aud_face
                                && !aud.is_playing()
                            {
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
            if let Some(aud) = &aud_another_him {
                if play_another_him {
                    aud.set_pitch(pitch_another_him);
                    if !aud.is_playing() {
                        aud.play();
                    }
                } else if aud.is_playing() {
                    aud.stop();
                }
            }
            let mult = {
                let cond;
                #[cfg(not(feature = "uninstaller"))]
                {
                    cond = state >= PatcherState::StartTalkE
                        && (state != PatcherState::PatchFailed
                            || typer.as_ref().map(|x| x.is_finished()).unwrap_or(false))
                        && state < PatcherState::EndTalkA;
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
            //info!("galpha={fake_galpha:.2} falpha={falpha:.2} final={:.2}", fake_galpha.clamp(0., 1.) * falpha.clamp(0., 1.));
            if let Some(typer) = &mut typer {
                typer.draw(
                    &mut d,
                    alpha,
                    fake_galpha.clamp(0., 1.) * falpha.clamp(0., 1.),
                );
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
                        typer.draw_text_centered_half_part(
                            &mut d,
                            Vector2::new(
                                320.,
                                typer.pos.y
                                    + typer.fnt.measure_text(typer.get_text(), 32., 8.).y
                                    + 20.,
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
                            typer.draw_text_centered_part(
                                &mut d,
                                Vector2::new(320., 260. + (i as f32 * 40.)),
                                q,
                                usize::MAX,
                                alpha,
                                galpha.clamp(0., 1.),
                                if i == qoption {
                                    Color::YELLOW
                                } else {
                                    Color::WHITE
                                }
                                .alpha(qalpha),
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
                    typer.draw_text_half(
                        &mut d,
                        Vector2::new(10., 10.),
                        "Presiona [X/SHIFT] para ir más rápido.\nPresiona [C/CTRL] para saltar la intro.",
                        alpha,
                        inst_alpha,
                    );
                }
            }
            if let Some(goner) = goner.as_ref() {
                let mut scale = 1.;
                while (goner.height as f32 * (scale + 0.5)) < 200. {
                    scale += 0.5;
                }
                let tp = *typer_progress.lock();
                let src =
                    Rectangle::new(0., 0., goner.width as _, (goner.height as f32 * tp).floor());
                let dest = Rectangle::new(
                    (320. - (goner.width as f32 * 0.5 * scale)).floor(),
                    (480. * (7. / 10.) - (goner.height as f32 * (0.5 * scale))).floor(),
                    (goner.width as f32 * scale).floor(),
                    (goner.height as f32 * tp * scale).floor(),
                );
                //info!("{scale}, {}, {}, {src:?}, {dest:?}", goner.width, goner.height);
                d.draw_texture_pro(
                    goner,
                    src,
                    dest,
                    Vector2::zero(),
                    0.,
                    Color::WHITE.alpha(goner_alpha.clamp(0., 1.)),
                );
            }
            #[cfg(feature = "uninstaller")]
            if shown_friend && let Some(img_friend) = &img_friend {
                d.draw_texture_pro(
                    img_friend,
                    Rectangle::new(0., 0., img_friend.width as _, img_friend.height as _),
                    Rectangle::new(
                        320.,
                        240.,
                        img_friend.width as f32 * 2. * friend_scale,
                        img_friend.height as f32 * 2. * friend_scale,
                    ),
                    Vector2::new(
                        img_friend.width as f32 * friend_scale,
                        img_friend.height as f32 * friend_scale,
                    ),
                    0.,
                    Color::WHITE,
                );
                friend_scale += delta;
            }
        }
        //d.draw_fps(20, 20);
    }
}
