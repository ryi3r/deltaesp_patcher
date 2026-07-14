use sola_raylib::{
    color::Color,
    drawing::{RaylibDraw, RaylibDrawHandle},
    math::{Rectangle, Vector2},
    texture::Texture2D,
};

pub struct ImageSoulBlurRenderer {
    pub spr: Option<Texture2D>,
    pub pos: Vector2,
    pub time: f32,
    pub momentum: f32,
}

impl ImageSoulBlurRenderer {
    pub fn new(spr: Option<Texture2D>) -> Self {
        Self {
            spr,
            pos: Vector2::new(320., 240.),
            time: -10.,
            momentum: 0.5,
        }
    }

    pub fn get_max(&self) -> f32 {
        (self.spr.as_ref().map(|x| x.height).unwrap_or(0) as f32) / 2.
    }

    pub fn get_max_time(&self) -> f32 {
        (self.spr.as_ref().map(|x| x.height).unwrap_or(0) as f32) / 2.
    }

    pub fn draw(&mut self, d: &mut RaylibDrawHandle) {
        let Some(spr) = &self.spr else {
            return;
        };
        if self.time <= 0. {
            let x = (1. + (self.time / 10.)).max(0.);
            d.draw_texture_pro(
                spr,
                Rectangle::new(0., self.get_max(), spr.width as _, 1.),
                Rectangle::new(
                    self.pos.x - (spr.width as f32 * x),
                    self.pos.y + self.get_max() - 400.,
                    spr.width as f32 * x * 2.,
                    spr.height as f32 * 800.,
                ),
                Vector2::zero(),
                0.,
                Color::WHITE,
            );
        } else if self.time < self.get_max() {
            let offset = Vector2::new(spr.width as _, spr.height as _);
            d.draw_texture_pro(
                spr,
                Rectangle::new(
                    0.,
                    self.get_max() - self.time,
                    spr.width as _,
                    1. + (self.time * 2.),
                ),
                Rectangle::new(
                    self.pos.x,
                    self.pos.y - (self.time * 2.) + self.get_max() + (spr.height as f32 / 2.),
                    spr.width as f32 * 2.,
                    2. * (self.time * 2.),
                ),
                offset,
                0.,
                Color::WHITE,
            );
            d.draw_texture_pro(
                spr,
                Rectangle::new(0., self.get_max() - self.time - 1., spr.width as _, 1.),
                Rectangle::new(
                    self.pos.x,
                    self.pos.y - 400. - self.time + self.get_max(),
                    spr.width as f32 * 2.,
                    spr.height as f32 * 800.,
                ),
                offset,
                0.,
                Color::WHITE,
            );
            d.draw_texture_pro(
                spr,
                Rectangle::new(0., self.get_max() + self.time, spr.width as _, 1.),
                Rectangle::new(
                    self.pos.x,
                    self.pos.y + self.time + self.get_max(),
                    spr.width as f32 * 2.,
                    spr.height as f32 * 800.,
                ),
                offset,
                0.,
                Color::WHITE,
            );
        } else {
            d.draw_texture_ex(
                spr,
                Vector2::new(
                    self.pos.x - spr.width as f32,
                    self.pos.y - spr.height as f32,
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
