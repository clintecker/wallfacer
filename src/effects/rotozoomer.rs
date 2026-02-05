//! Rotozoomer Effect
//!
//! Rotating and zooming a tiled texture across the screen.
//! Invented by Chaos/Sanity on Amiga 500 in 1989.
//!
//! Uses incremental scanline stepping to avoid per-pixel multiplies.
//! MipTexture eliminates aliasing when zoomed out.

use super::Effect;
use crate::display::PixelBuffer;
use crate::regions::Scene;
use crate::texture::{MipTexture, Texture};

/// Classic rotozoomer with procedural texture
pub struct Rotozoomer {
    time: f32,
    mip_texture: MipTexture,
    rotation_speed: f32,
    zoom_speed: f32,
    zoom_base: f32,
    zoom_range: f32,
}

impl Rotozoomer {
    pub fn new() -> Self {
        let texture = Texture::xor_pattern(256);
        let mip_texture = MipTexture::from_texture(&texture);

        Self {
            time: 0.0,
            mip_texture,
            rotation_speed: 0.5,
            zoom_speed: 0.3,
            zoom_base: 1.5,
            zoom_range: 1.0,
        }
    }

    /// Create with a checkerboard texture
    pub fn checkerboard() -> Self {
        let texture = Texture::checkerboard(256, 32, (255, 100, 50), (50, 100, 255));
        let mip_texture = MipTexture::from_texture(&texture);
        Self {
            time: 0.0,
            mip_texture,
            rotation_speed: 0.4,
            zoom_speed: 0.25,
            zoom_base: 2.0,
            zoom_range: 1.5,
        }
    }

    /// Create with a plasma texture
    pub fn plasma() -> Self {
        let palette: Vec<(u8, u8, u8)> = (0..256)
            .map(|i| {
                let t = i as f32 / 255.0;
                let r = ((t * std::f32::consts::TAU * 2.0).sin() * 0.5 + 0.5) * 255.0;
                let g = ((t * std::f32::consts::TAU * 3.0 + 1.0).sin() * 0.5 + 0.5) * 255.0;
                let b = ((t * std::f32::consts::TAU * 5.0 + 2.0).sin() * 0.5 + 0.5) * 255.0;
                (r as u8, g as u8, b as u8)
            })
            .collect();
        let texture = Texture::plasma(256, &palette);
        let mip_texture = MipTexture::from_texture(&texture);

        Self {
            time: 0.0,
            mip_texture,
            rotation_speed: 0.3,
            zoom_speed: 0.2,
            zoom_base: 1.0,
            zoom_range: 0.5,
        }
    }
}

impl Default for Rotozoomer {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Rotozoomer {
    fn update(&mut self, dt: f32, _width: u32, _height: u32, _scene: &Scene) {
        self.time += dt;
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        let width = buffer.width() as i32;
        let height = buffer.height() as i32;
        let cx = width as f32 / 2.0;
        let cy = height as f32 / 2.0;

        let angle = self.time * self.rotation_speed;
        let zoom = self.zoom_base + (self.time * self.zoom_speed).sin() * self.zoom_range;

        // Mip level from zoom: when zoomed out (zoom < 1), use higher mip to reduce aliasing
        let max_mip = (self.mip_texture.level_count() - 1) as f32;
        let mip = if zoom < 1.0 {
            (-zoom.log2()).min(max_mip) as u32
        } else {
            0
        };

        // Work in texel space (0..tex_width) instead of UV space (0..1)
        // Use level 0 dimensions for coordinate computation — sample_mipped shifts internally
        let tex_scale = self.mip_texture.level(0).width() as f32 * 0.01;
        let cos_a = angle.cos() / zoom * tex_scale;
        let sin_a = angle.sin() / zoom * tex_scale;

        // Texel step per screen pixel
        let du_dx = cos_a;
        let dv_dx = sin_a;
        let du_dy = -sin_a;
        let dv_dy = cos_a;

        // Texel coords at top-left corner
        let mut u_row = -cx * du_dx - cy * du_dy;
        let mut v_row = -cx * dv_dx - cy * dv_dy;

        let pixels = buffer.as_bytes_mut();
        let mut idx = 0;

        for _y in 0..height {
            let mut u = u_row;
            let mut v = v_row;

            for _x in 0..width {
                let (r, g, b) = self.mip_texture.sample_mipped(u as i32, v as i32, mip);

                pixels[idx] = 255;
                pixels[idx + 1] = b;
                pixels[idx + 2] = g;
                pixels[idx + 3] = r;
                idx += 4;

                u += du_dx;
                v += dv_dx;
            }

            u_row += du_dy;
            v_row += dv_dy;
        }
    }

    fn name(&self) -> &str {
        "Rotozoomer"
    }
}
