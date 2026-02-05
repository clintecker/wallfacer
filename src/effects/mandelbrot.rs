//! Mandelbrot Zoom Effect
//!
//! Animated deep zoom into the Mandelbrot set with smooth color cycling.
//! Uses f64 precision for center/zoom to handle deep zoom levels.
//! Per-pixel rendering directly to the buffer for maximum speed.

use super::Effect;
use crate::display::PixelBuffer;
use crate::regions::Scene;
use crate::util::hsv_to_rgb;

const PALETTE_SIZE: usize = 256;

/// Animated Mandelbrot zoom with smooth coloring
pub struct Mandelbrot {
    time: f32,
    palette: Vec<(u8, u8, u8)>,
    zoom: f64,
    center_x: f64,
    center_y: f64,
    max_iter: u32,
}

impl Mandelbrot {
    pub fn new() -> Self {
        // Build HSV palette with varying hue, high saturation
        let palette: Vec<(u8, u8, u8)> = (0..PALETTE_SIZE)
            .map(|i| {
                let t = i as f32 / PALETTE_SIZE as f32;
                hsv_to_rgb(t * 360.0, 0.85, 0.95)
            })
            .collect();

        Self {
            time: 0.0,
            palette,
            zoom: 1.0,
            // Seahorse Valley — a visually rich zoom target
            center_x: -0.745,
            center_y: 0.186,
            max_iter: 64,
        }
    }
}

impl Default for Mandelbrot {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Mandelbrot {
    fn update(&mut self, dt: f32, _width: u32, _height: u32, _scene: &Scene) {
        self.time += dt;
        // Slow ping-pong zoom: in for 20s, out for 20s — stays within f32 precision
        let cycle_period = 20.0;
        let t = (self.time % (cycle_period * 2.0)) / cycle_period;
        // Triangle wave: 0→1→0
        let t_pingpong = if t < 1.0 { t } else { 2.0 - t };
        // Smooth ease-in-out
        let eased = t_pingpong * t_pingpong * (3.0 - 2.0 * t_pingpong);
        // Zoom range: 1× to ~1,800× (stays crisp within f32 iteration precision)
        self.zoom = (eased as f64 * 7.5).exp();
        // Scale iterations with zoom — periodicity checking keeps interior cheap
        self.max_iter = 64 + ((self.zoom.ln().max(0.0) * 8.0) as u32).min(160);
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        let width = buffer.width() as i32;
        let height = buffer.height() as i32;
        let pixels = buffer.as_bytes_mut();

        let pixel_size = width.min(height) as f64;
        let inv_zoom = 3.0 / (pixel_size * self.zoom);
        let cx_offset = width as f64 * 0.5;
        let cy_offset = height as f64 * 0.5;

        let palette_shift = (self.time * 30.0) as usize;
        let ln2 = 2.0_f32.ln();
        let max_iter = self.max_iter;

        let mut idx = 0;
        for py in 0..height {
            // f64 for coordinate mapping (precision at deep zoom)
            let ci_f64 = (py as f64 - cy_offset) * inv_zoom + self.center_y;
            let ci = ci_f64 as f32;

            for px in 0..width {
                let cr_f64 = (px as f64 - cx_offset) * inv_zoom + self.center_x;
                let cr = cr_f64 as f32;

                // Cardioid check: skip points inside the main cardioid
                let q = (cr - 0.25) * (cr - 0.25) + ci * ci;
                if q * (q + (cr - 0.25)) <= 0.25 * ci * ci {
                    pixels[idx] = 255;
                    pixels[idx + 1] = 0;
                    pixels[idx + 2] = 0;
                    pixels[idx + 3] = 0;
                    idx += 4;
                    continue;
                }
                // Period-2 bulb check
                if (cr + 1.0) * (cr + 1.0) + ci * ci <= 0.0625 {
                    pixels[idx] = 255;
                    pixels[idx + 1] = 0;
                    pixels[idx + 2] = 0;
                    pixels[idx + 3] = 0;
                    idx += 4;
                    continue;
                }

                let mut zr = 0.0_f32;
                let mut zi = 0.0_f32;
                let mut iter = 0u32;
                let mut zr2 = 0.0_f32;
                let mut zi2 = 0.0_f32;

                // Periodicity checking (Brent's cycle detection)
                let mut saved_zr = 0.0_f32;
                let mut saved_zi = 0.0_f32;
                let mut period = 0u32;
                let mut check_period = 8u32;

                while zr2 + zi2 <= 4.0 && iter < max_iter {
                    zi = 2.0 * zr * zi + ci;
                    zr = zr2 - zi2 + cr;
                    zr2 = zr * zr;
                    zi2 = zi * zi;
                    iter += 1;

                    // Detect periodic orbit → point is inside the set
                    if (zr - saved_zr).abs() < 1e-7 && (zi - saved_zi).abs() < 1e-7 {
                        iter = max_iter;
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

                if iter == max_iter {
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
        "Mandelbrot Zoom"
    }
}
