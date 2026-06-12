use sola_raylib::{
    color::Color,
    drawing::{RaylibDraw, RaylibDrawHandle},
    math::{Rectangle, Vector2},
    texture::Texture2D,
};

pub struct ImageDepthRenderer {
    pub scale: f32,
    pub time: f32,
    pub speed: f32,

    pub alpha1: f32,
    pub alpha2: f32,
    pub alpha3: f32,
}

impl Default for ImageDepthRenderer {
    fn default() -> Self {
        Self {
            scale: 1.,
            time: 0.,
            speed: 0.02,
            alpha1: 1.,   // normal
            alpha2: 0.,   // o
            alpha3: -0.2, // b
        }
    }
}

impl ImageDepthRenderer {
    pub fn draw(&mut self, tex: &Texture2D, d: &mut RaylibDrawHandle, mult: f32) -> bool {
        let ft = d.get_frame_time() / (1. / 30.);
        self.time += ft;
        if self.speed > 0. {
            self.alpha1 = (self.time / 34.).sin() * 0.2;
        }
        self.scale += self.speed * ft;
        if self.alpha3 < 0. {
            self.alpha3 += 0.01 * ft * mult;
        }
        if self.scale > 2. {
            self.alpha2 += 0.01 * ft * mult;
        }
        if self.time > 2. {
            let a = (0.2 + self.alpha1 - self.alpha2 + self.alpha3).clamp(0., 1.);
            //println!("a:{a:.3}");
            if a <= 0.01 {
                return true;
            }
            let (w, h) = (tex.width * 3, tex.height * 3);
            let dest = Rectangle::new(320., 240., w as f32 * self.scale, h as f32 * self.scale);
            d.draw_texture_pro(
                tex,
                Rectangle::new(0., 0., tex.width as _, tex.height as _),
                dest,
                Vector2::zero(),
                0.,
                Color::WHITE.alpha(a),
            );
            d.draw_texture_pro(
                tex,
                Rectangle::new(0., 0., -tex.width as _, tex.height as _),
                dest,
                Vector2::new(w as f32 * self.scale, 0.),
                0.,
                Color::WHITE.alpha(a),
            );
            d.draw_texture_pro(
                tex,
                Rectangle::new(0., 0., -tex.width as _, -tex.height as _),
                dest,
                Vector2::new(w as f32 * self.scale, h as f32 * self.scale),
                0.,
                Color::WHITE.alpha(a),
            );
            d.draw_texture_pro(
                tex,
                Rectangle::new(0., 0., tex.width as _, -tex.height as _),
                dest,
                Vector2::new(0., h as f32 * self.scale),
                0.,
                Color::WHITE.alpha(a),
            );
        }

        false
    }
}
