use std::{collections::HashMap, time::Instant};

use sola_raylib::{
    color::Color,
    drawing::{RaylibDraw, RaylibDrawHandle},
    ffi::KeyboardKey,
    math::Vector2,
    text::{Font, RaylibFont},
};

const TARGET_CHAR_TIME: f32 = 4. / 30.;
const NEW_LINE: char = '\n';

pub struct Typer {
    timer: Instant,
    pub text: String,
    pub pos: Vector2,
    pub playing: bool,
    chars: usize,
    delay: HashMap<usize, usize>,
    pub instant_show: bool,
    pub increase_typer: bool,
}

impl Default for Typer {
    fn default() -> Self {
        Self {
            timer: Instant::now(),
            text: String::new(),
            pos: Vector2::zero(),
            playing: false,
            chars: 0,
            delay: HashMap::new(),
            instant_show: false,
            increase_typer: true,
        }
    }
}

impl Typer {
    pub fn get_text(&self) -> &String {
        &self.text
    }

    pub fn draw_text_half(
        d: &mut RaylibDrawHandle,
        fnt: &Font,
        pos: Vector2,
        text: &str,
        shadow_alpha: f32,
        global_alpha: f32,
    ) {
        d.draw_text_ex(
            fnt,
            text,
            Vector2::new(pos.x - 1., pos.y),
            16.,
            4.,
            Color::WHITE.alpha(shadow_alpha * global_alpha),
        );
        d.draw_text_ex(
            fnt,
            text,
            Vector2::new(pos.x + 1., pos.y),
            16.,
            4.,
            Color::WHITE.alpha(shadow_alpha * global_alpha),
        );
        d.draw_text_ex(
            fnt,
            text,
            Vector2::new(pos.x, pos.y - 1.),
            16.,
            4.,
            Color::WHITE.alpha(shadow_alpha * global_alpha),
        );
        d.draw_text_ex(
            fnt,
            text,
            Vector2::new(pos.x, pos.y + 1.),
            16.,
            4.,
            Color::WHITE.alpha(shadow_alpha * global_alpha),
        );

        d.draw_text_ex(
            fnt,
            text,
            Vector2::new(pos.x - 1., pos.y - 1.),
            16.,
            4.,
            Color::WHITE.alpha(shadow_alpha * global_alpha / 2.),
        );
        d.draw_text_ex(
            fnt,
            text,
            Vector2::new(pos.x + 1., pos.y - 1.),
            16.,
            4.,
            Color::WHITE.alpha(shadow_alpha * global_alpha / 2.),
        );
        d.draw_text_ex(
            fnt,
            text,
            Vector2::new(pos.x + 1., pos.y + 1.),
            16.,
            4.,
            Color::WHITE.alpha(shadow_alpha * global_alpha / 2.),
        );
        d.draw_text_ex(
            fnt,
            text,
            Vector2::new(pos.x - 1., pos.y + 1.),
            16.,
            4.,
            Color::WHITE.alpha(shadow_alpha * global_alpha / 2.),
        );

        d.draw_text_ex(fnt, text, pos, 16., 4., Color::WHITE.alpha(global_alpha));
    }

    pub fn draw_text(
        d: &mut RaylibDrawHandle,
        fnt: &Font,
        pos: Vector2,
        text: &str,
        shadow_alpha: f32,
        global_alpha: f32,
        tint: Color,
    ) {
        d.draw_text_ex(
            fnt,
            text,
            Vector2::new(pos.x - 2., pos.y),
            32.,
            8.,
            tint.alpha((tint.a as f32 / 255.) * shadow_alpha * global_alpha),
        );
        d.draw_text_ex(
            fnt,
            text,
            Vector2::new(pos.x + 2., pos.y),
            32.,
            8.,
            tint.alpha((tint.a as f32 / 255.) * shadow_alpha * global_alpha),
        );
        d.draw_text_ex(
            fnt,
            text,
            Vector2::new(pos.x, pos.y - 2.),
            32.,
            8.,
            tint.alpha((tint.a as f32 / 255.) * shadow_alpha * global_alpha),
        );
        d.draw_text_ex(
            fnt,
            text,
            Vector2::new(pos.x, pos.y + 2.),
            32.,
            8.,
            tint.alpha((tint.a as f32 / 255.) * shadow_alpha * global_alpha),
        );

        d.draw_text_ex(
            fnt,
            text,
            Vector2::new(pos.x - 2., pos.y - 2.),
            32.,
            8.,
            tint.alpha((tint.a as f32 / 255.) * shadow_alpha * global_alpha / 2.),
        );
        d.draw_text_ex(
            fnt,
            text,
            Vector2::new(pos.x + 2., pos.y - 2.),
            32.,
            8.,
            tint.alpha((tint.a as f32 / 255.) * shadow_alpha * global_alpha / 2.),
        );
        d.draw_text_ex(
            fnt,
            text,
            Vector2::new(pos.x + 2., pos.y + 2.),
            32.,
            8.,
            tint.alpha((tint.a as f32 / 255.) * shadow_alpha * global_alpha / 2.),
        );
        d.draw_text_ex(
            fnt,
            text,
            Vector2::new(pos.x - 2., pos.y + 2.),
            32.,
            8.,
            tint.alpha((tint.a as f32 / 255.) * shadow_alpha * global_alpha / 2.),
        );

        d.draw_text_ex(
            fnt,
            text,
            pos,
            32.,
            8.,
            tint.alpha((tint.a as f32 / 255.) * global_alpha),
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw_text_centered_part(
        d: &mut RaylibDrawHandle,
        fnt: &Font,
        pos: Vector2,
        text: &str,
        offset: usize,
        shadow_alpha: f32,
        global_alpha: f32,
        tint: Color,
    ) {
        let nl_y = fnt.measure_text(&NEW_LINE.to_string(), 32., 8.).y / 2.;
        let mut c = 0;
        for (i, s) in text.split(NEW_LINE).enumerate() {
            let s_x = fnt.measure_text(s, 32., 8.).x / 2.;
            if c + s.len() >= offset {
                Self::draw_text(
                    d,
                    fnt,
                    Vector2::new(
                        (pos.x - s_x).floor(),
                        (pos.y + (i as f32 * 8.)).floor() + (nl_y * i as f32),
                    ),
                    &s.chars().take(offset - c).collect::<String>(),
                    shadow_alpha,
                    global_alpha,
                    tint,
                );
                break;
            }
            Self::draw_text(
                d,
                fnt,
                Vector2::new(
                    (pos.x - s_x).floor(),
                    (pos.y + (i as f32 * 8.)).floor() + (nl_y * i as f32),
                ),
                s,
                shadow_alpha,
                global_alpha,
                tint,
            );
            c += s.len() + 1; // include newline
        }
    }

    pub fn draw_text_centered_half_part(
        d: &mut RaylibDrawHandle,
        fnt: &Font,
        pos: Vector2,
        text: &str,
        offset: usize,
        shadow_alpha: f32,
        global_alpha: f32,
    ) {
        let nl_y = fnt.measure_text(&NEW_LINE.to_string(), 16., 4.).y / 2.;
        let mut c = 0;
        for (i, s) in text.split(NEW_LINE).enumerate() {
            let s_x = fnt.measure_text(s, 16., 4.).x / 2.;
            if c + s.len() >= offset {
                Self::draw_text_half(
                    d,
                    fnt,
                    Vector2::new(
                        (pos.x - s_x).floor(),
                        (pos.y + (i as f32 * 8.)).floor() + (nl_y * i as f32),
                    ),
                    &s[..(offset - c)],
                    shadow_alpha,
                    global_alpha,
                );
                break;
            }
            Self::draw_text_half(
                d,
                fnt,
                Vector2::new(
                    (pos.x - s_x).floor(),
                    (pos.y + (i as f32 * 8.)).floor() + (nl_y * i as f32),
                ),
                s,
                shadow_alpha,
                global_alpha,
            );
            c += s.len() + 1; // include newline
        }
    }

    pub fn start(&mut self, text: &str) {
        self.delay.clear();
        self.chars = 0;
        let mut ntext = String::new();
        let mut skip = 0;
        for (i, char) in text.chars().enumerate() {
            if skip > 0 {
                skip -= 1;
                continue;
            }
            let last_char = text.chars().nth(i.saturating_sub(1)).unwrap();
            if char == '^' && last_char != '\\' && text.len() > i + 1 {
                let next_char = text.chars().nth(i + 1).unwrap();
                if next_char == '^' && text.len() > i + 3 {
                    self.delay.insert(
                        ntext.chars().count() + 1,
                        text.chars()
                            .skip(i + 2)
                            .take(2)
                            .collect::<String>()
                            .parse()
                            .unwrap(),
                    );
                    skip = 3;
                } else {
                    self.delay.insert(
                        ntext.chars().count() + 1,
                        next_char.to_string().parse().unwrap(),
                    );
                    skip = 1;
                }
            } else {
                ntext += &char.to_string();
            }
        }
        self.increase_typer = true;
        self.text = ntext;
        self.playing = true;
        self.timer = Instant::now();
    }

    pub fn stop(&mut self) {
        self.chars = 0;
        self.text = "".into();
        self.playing = false;
        self.timer = Instant::now();
        self.instant_show = false;
        self.increase_typer = true;
    }

    pub fn is_finished(&self) -> bool {
        self.playing && self.chars > self.text.len()
    }

    pub fn draw(
        &mut self,
        fnt: &Font,
        d: &mut RaylibDrawHandle,
        shadow_alpha: f32,
        global_alpha: f32,
    ) {
        let is_skip_pressed = d.is_key_down(KeyboardKey::KEY_X)
            || d.is_key_down(KeyboardKey::KEY_LEFT_SHIFT)
            || d.is_key_down(KeyboardKey::KEY_RIGHT_SHIFT);
        if self.instant_show {
            self.chars = self.text.len() + 1;
        }
        if self.chars < (self.text.len() + 1)
            && self.timer.elapsed().as_secs_f32()
                > TARGET_CHAR_TIME
                    / (if is_skip_pressed { 2. } else { 1. })
                    / (if cfg!(feature = "uninstaller") {
                        2.
                    } else {
                        1.
                    })
        {
            self.timer = Instant::now();
            if self.delay.contains_key(&self.chars) && !is_skip_pressed {
                let e = self.delay.entry(self.chars).or_default();
                if *e > 0 {
                    *e -= 1;
                } else {
                    self.chars += 1;
                }
            } else {
                self.chars += 1;
            }
        }
        Self::draw_text_centered_part(
            d,
            fnt,
            self.pos,
            &self.text,
            self.chars,
            shadow_alpha,
            global_alpha,
            Color::WHITE,
        );
    }
}
