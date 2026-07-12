use sola_raylib::{
    color::Color,
    drawing::{RaylibDraw, RaylibDrawHandle},
    math::{Rectangle, Vector2},
    texture::Texture2D,
};

pub struct ImageSoulBlurRenderer {
    pub spr: Texture2D,
    pub pos: Vector2,
    pub time: f32,
    pub momentum: f32,
}

impl ImageSoulBlurRenderer {
    pub fn new(spr: Texture2D) -> Self {
        Self {
            spr,
            pos: Vector2::new(320., 240.),
            time: -10.,
            momentum: 0.5,
        }
    }

    pub fn get_max(&self) -> f32 {
        (self.spr.height as f32) / 2.
    }

    pub fn get_max_time(&self) -> f32 {
        (self.spr.height as f32) / 2.
    }

    pub fn draw(&mut self, d: &mut RaylibDrawHandle) {
        if self.time <= 0. {
            let x = (1. + (self.time / 10.)).max(0.);
            d.draw_texture_pro(
                &self.spr,
                Rectangle::new(0., self.get_max(), self.spr.width as _, 1.),
                Rectangle::new(
                    self.pos.x - (self.spr.width as f32 * x),
                    self.pos.y + self.get_max() - 400.,
                    self.spr.width as f32 * x * 2.,
                    self.spr.height as f32 * 800.,
                ),
                Vector2::zero(),
                0.,
                Color::WHITE,
            );
        } else if self.time < self.get_max() {
            let offset = Vector2::new(self.spr.width as _, self.spr.height as _);
            d.draw_texture_pro(
                &self.spr,
                Rectangle::new(
                    0.,
                    self.get_max() - self.time,
                    self.spr.width as _,
                    1. + (self.time * 2.),
                ),
                Rectangle::new(
                    self.pos.x,
                    self.pos.y - (self.time * 2.) + self.get_max() + (self.spr.height as f32 / 2.),
                    self.spr.width as f32 * 2.,
                    2. * (self.time * 2.),
                ),
                offset,
                0.,
                Color::WHITE,
            );
            d.draw_texture_pro(
                &self.spr,
                Rectangle::new(0., self.get_max() - self.time - 1., self.spr.width as _, 1.),
                Rectangle::new(
                    self.pos.x,
                    self.pos.y - 400. - self.time + self.get_max(),
                    self.spr.width as f32 * 2.,
                    self.spr.height as f32 * 800.,
                ),
                offset,
                0.,
                Color::WHITE,
            );
            d.draw_texture_pro(
                &self.spr,
                Rectangle::new(0., self.get_max() + self.time, self.spr.width as _, 1.),
                Rectangle::new(
                    self.pos.x,
                    self.pos.y + self.time + self.get_max(),
                    self.spr.width as f32 * 2.,
                    self.spr.height as f32 * 800.,
                ),
                offset,
                0.,
                Color::WHITE,
            );
        } else {
            d.draw_texture_ex(
                &self.spr,
                Vector2::new(
                    self.pos.x - self.spr.width as f32,
                    self.pos.y - self.spr.height as f32,
                ),
                0.,
                2.,
                Color::WHITE,
            );
        }

        if (self.momentum > 0. && self.time < self.get_max_time() + 2.) || self.momentum < 0. {
            self.time += self.momentum * (d.get_frame_time() / (1. / 30.));
        }
    }
}
