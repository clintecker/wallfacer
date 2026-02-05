use super::{color, Effect};
use crate::display::PixelBuffer;
use crate::regions::Scene;

/// Classic demoscene plasma effect
pub struct Plasma {
    time: f32,
    palette: Vec<(u8, u8, u8)>,
    sin_table: Vec<f32>,
}

impl Plasma {
    pub fn new() -> Self {
        // Pre-compute sine table for speed
        let sin_table: Vec<f32> = (0..256)
            .map(|i| (i as f32 * std::f32::consts::TAU / 256.0).sin())
            .collect();

        Self {
            time: 0.0,
            palette: color::make_palette(256),
            sin_table,
        }
    }

    #[inline]
    fn fast_sin(&self, x: f32) -> f32 {
        let idx = ((x * 40.74) as i32 & 255) as usize;
        self.sin_table[idx]
    }
}

impl Default for Plasma {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Plasma {
    fn update(&mut self, dt: f32, _width: u32, _height: u32, _scene: &Scene) {
        self.time += dt;
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        let width = buffer.width();
        let height = buffer.height();
        let t = self.time;

        for y in 0..height {
            for x in 0..width {
                let fx = x as f32;
                let fy = y as f32;

                // Classic plasma formula - sum of sines at different frequencies
                let v1 = self.fast_sin(fx * 0.02 + t);
                let v2 = self.fast_sin(fy * 0.03 + t * 0.5);
                let v3 = self.fast_sin((fx + fy) * 0.02 + t * 0.7);
                let v4 = self.fast_sin(((fx * fx + fy * fy).sqrt() * 0.03) + t);

                let v = (v1 + v2 + v3 + v4 + 4.0) / 8.0; // Normalize to 0-1
                let idx = (v * 255.0) as usize;

                let (r, g, b) = self.palette[idx.min(255)];

                // SAFETY: We're iterating within bounds
                unsafe {
                    buffer.set_pixel_unchecked(x, y, r, g, b);
                }
            }
        }
    }

    fn name(&self) -> &str {
        "Plasma"
    }

    fn region_color(&self) -> (u8, u8, u8) {
        // Deep purple glow matching plasma vibe
        (20, 5, 30)
    }
}
