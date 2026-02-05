//! Julia Morph Effect
//!
//! Julia set where the c parameter orbits near the Mandelbrot boundary,
//! causing the fractal to morph continuously between connected and
//! disconnected shapes. Uses smooth coloring with palette cycling.

use super::Effect;
use crate::display::PixelBuffer;
use crate::regions::Scene;
use crate::util::hsv_to_rgb;

const MAX_ITER: u32 = 80;
const PALETTE_SIZE: usize = 256;

/// Morphing Julia set with animated c parameter
pub struct Julia {
    time: f32,
    palette: Vec<(u8, u8, u8)>,
}

impl Julia {
    pub fn new() -> Self {
        // Different hue offset from Mandelbrot for visual distinction
        let palette: Vec<(u8, u8, u8)> = (0..PALETTE_SIZE)
            .map(|i| {
                let t = i as f32 / PALETTE_SIZE as f32;
                hsv_to_rgb((t * 360.0 + 120.0) % 360.0, 0.9, 0.9)
            })
            .collect();

        Self { time: 0.0, palette }
    }
}

impl Default for Julia {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Julia {
    fn update(&mut self, dt: f32, _width: u32, _height: u32, _scene: &Scene) {
        self.time += dt;
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        let width = buffer.width() as i32;
        let height = buffer.height() as i32;
        let pixels = buffer.as_bytes_mut();

        // Animated c orbiting near the Mandelbrot boundary
        let c_re = 0.355 + 0.3 * (self.time * 0.15).cos();
        let c_im = 0.355 + 0.3 * (self.time * 0.2).sin();

        let pixel_size = width.min(height) as f32;
        let scale = 3.0 / pixel_size;
        let cx_offset = width as f32 * 0.5;
        let cy_offset = height as f32 * 0.5;

        let palette_shift = (self.time * 25.0) as usize;
        let ln2 = 2.0_f32.ln();

        let mut idx = 0;
        for py in 0..height {
            let zi0 = (py as f32 - cy_offset) * scale;
            for px in 0..width {
                let zr0 = (px as f32 - cx_offset) * scale;

                let mut zr = zr0;
                let mut zi = zi0;
                let mut iter = 0u32;
                let mut zr2 = zr * zr;
                let mut zi2 = zi * zi;

                // Periodicity checking (Brent's cycle detection)
                let mut saved_zr = zr;
                let mut saved_zi = zi;
                let mut period = 0u32;
                let mut check_period = 8u32;

                while zr2 + zi2 <= 4.0 && iter < MAX_ITER {
                    zi = 2.0 * zr * zi + c_im;
                    zr = zr2 - zi2 + c_re;
                    zr2 = zr * zr;
                    zi2 = zi * zi;
                    iter += 1;

                    // Detect periodic orbit → point is inside the set
                    if (zr - saved_zr).abs() < 1e-7 && (zi - saved_zi).abs() < 1e-7 {
                        iter = MAX_ITER;
                        break;
                    }
                    period += 1;
                    if period >= check_period {
                        saved_zr = zr;
                        saved_zi = zi;
                        period = 0;
                        check_period = (check_period * 2).min(256);
                    }
                }

                if iter == MAX_ITER {
                    pixels[idx] = 255;
                    pixels[idx + 1] = 0;
                    pixels[idx + 2] = 0;
                    pixels[idx + 3] = 0;
                } else {
                    let modulus = (zr2 + zi2).sqrt();
                    let smooth = iter as f32 + 1.0 - modulus.ln().ln() / ln2;
                    let color_idx = ((smooth * 4.0) as usize + palette_shift) % PALETTE_SIZE;
                    let (r, g, b) = self.palette[color_idx];

                    pixels[idx] = 255;
                    pixels[idx + 1] = b;
                    pixels[idx + 2] = g;
                    pixels[idx + 3] = r;
                }

                idx += 4;
            }
        }
    }

    fn name(&self) -> &str {
        "Julia Morph"
    }
}
