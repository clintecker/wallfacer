//! Bobs (Blitter Objects) Effect
//!
//! Classic bouncing sprites with trails - the bread and butter of oldschool demos.
//! Shows as many bobs as possible, all moving independently.

use super::Effect;
use crate::display::PixelBuffer;
use crate::regions::Scene;
use crate::util::hsv_to_rgb;
use std::f32::consts::TAU;

/// Number of bobs in the effect
const NUM_BOBS: usize = 16;

/// Base bob radius in pixels (designed for 480p)
const BASE_BOB_RADIUS: f32 = 20.0;

/// A single bouncing bob
struct Bob {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    hue: f32,
    phase: f32,
}

impl Bob {
    fn new(x: f32, y: f32, angle: f32, speed: f32, hue: f32, phase: f32) -> Self {
        Self {
            x,
            y,
            vx: angle.cos() * speed,
            vy: angle.sin() * speed,
            hue,
            phase,
        }
    }
}

/// Classic bouncing bobs with trails
pub struct Bobs {
    time: f32,
    bobs: Vec<Bob>,
    trail_buffer: Vec<u8>, // Separate buffer for trail fade
    trail_w: u32,
    trail_h: u32,
    screen_scale: f32, // min(w,h) / 480.0
}

impl Bobs {
    pub fn new() -> Self {
        let bobs = (0..NUM_BOBS)
            .map(|i| {
                let t = i as f32 / NUM_BOBS as f32;
                let angle = t * TAU;
                let speed = 150.0 + t * 100.0;
                Bob::new(
                    320.0, // Will be adjusted on first update
                    240.0,
                    angle,
                    speed,
                    t * 360.0,
                    t * TAU,
                )
            })
            .collect();

        Self {
            time: 0.0,
            bobs,
            trail_buffer: Vec::new(),
            trail_w: 0,
            trail_h: 0,
            screen_scale: 1.0,
        }
    }

    /// Initialize or resize trail buffer
    fn ensure_trail_buffer(&mut self, width: u32, height: u32) {
        if self.trail_w != width || self.trail_h != height {
            self.trail_w = width;
            self.trail_h = height;
            self.trail_buffer = vec![0u8; (width * height * 4) as usize];
        }
    }

    /// Fade the trail buffer
    fn fade_trails(&mut self) {
        for chunk in self.trail_buffer.chunks_exact_mut(4) {
            // Fade RGB, keep alpha
            chunk[1] = (chunk[1] as u16 * 245 / 256) as u8;
            chunk[2] = (chunk[2] as u16 * 245 / 256) as u8;
            chunk[3] = (chunk[3] as u16 * 245 / 256) as u8;
        }
    }

    /// Draw a bob to the trail buffer
    fn draw_bob_to_trail(&mut self, x: i32, y: i32, r: u8, g: u8, b: u8, radius: i32) {
        let w = self.trail_w as i32;
        let h = self.trail_h as i32;

        // Draw filled circle with gradient
        for dy in -radius..=radius {
            let py = y + dy;
            if py < 0 || py >= h {
                continue;
            }

            let row_width = ((radius * radius - dy * dy) as f32).sqrt() as i32;

            for dx in -row_width..=row_width {
                let px = x + dx;
                if px < 0 || px >= w {
                    continue;
                }

                // Distance from center for gradient
                let dist = ((dx * dx + dy * dy) as f32).sqrt();
                let intensity = 1.0 - (dist / radius as f32);
                let intensity = intensity * intensity; // Quadratic falloff

                let idx = ((py as u32 * self.trail_w + px as u32) * 4) as usize;

                // Additive blending to trail
                let add_r = (r as f32 * intensity) as u16;
                let add_g = (g as f32 * intensity) as u16;
                let add_b = (b as f32 * intensity) as u16;

                self.trail_buffer[idx] = 255;
                self.trail_buffer[idx + 1] =
                    (self.trail_buffer[idx + 1] as u16 + add_b).min(255) as u8;
                self.trail_buffer[idx + 2] =
                    (self.trail_buffer[idx + 2] as u16 + add_g).min(255) as u8;
                self.trail_buffer[idx + 3] =
                    (self.trail_buffer[idx + 3] as u16 + add_r).min(255) as u8;
            }
        }
    }
}

impl Default for Bobs {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Bobs {
    fn update(&mut self, dt: f32, width: u32, height: u32, _scene: &Scene) {
        self.time += dt;
        self.ensure_trail_buffer(width, height);
        self.screen_scale = width.min(height) as f32 / 480.0;

        let w = width as f32;
        let h = height as f32;
        let bob_radius = (BASE_BOB_RADIUS * self.screen_scale).round() as i32;
        let margin = bob_radius as f32;

        // Update bob positions with bouncing (scale speed proportionally)
        let sx = width as f32 / 640.0;
        let sy = height as f32 / 480.0;
        for bob in &mut self.bobs {
            bob.x += bob.vx * dt * sx;
            bob.y += bob.vy * dt * sy;

            // Bounce off edges
            if bob.x < margin {
                bob.x = margin;
                bob.vx = bob.vx.abs();
                bob.hue = (bob.hue + 30.0) % 360.0;
            } else if bob.x > w - margin {
                bob.x = w - margin;
                bob.vx = -bob.vx.abs();
                bob.hue = (bob.hue + 30.0) % 360.0;
            }

            if bob.y < margin {
                bob.y = margin;
                bob.vy = bob.vy.abs();
                bob.hue = (bob.hue + 30.0) % 360.0;
            } else if bob.y > h - margin {
                bob.y = h - margin;
                bob.vy = -bob.vy.abs();
                bob.hue = (bob.hue + 30.0) % 360.0;
            }

            // Slight wobble in velocity
            let wobble = (self.time * 2.0 + bob.phase).sin() * 0.02;
            let speed = (bob.vx * bob.vx + bob.vy * bob.vy).sqrt();
            let angle = bob.vy.atan2(bob.vx) + wobble;
            bob.vx = angle.cos() * speed;
            bob.vy = angle.sin() * speed;
        }

        // Fade existing trails
        self.fade_trails();

        // Collect bob draw data first to satisfy borrow checker
        let draw_data: Vec<(i32, i32, u8, u8, u8)> = self
            .bobs
            .iter()
            .map(|bob| {
                let (r, g, b) = hsv_to_rgb(bob.hue, 0.9, 1.0);
                (bob.x as i32, bob.y as i32, r, g, b)
            })
            .collect();

        // Draw bobs to trail buffer
        let bob_radius = (BASE_BOB_RADIUS * self.screen_scale).round() as i32;
        for (x, y, r, g, b) in draw_data {
            self.draw_bob_to_trail(x, y, r, g, b, bob_radius);
        }
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        // Copy trail buffer to output
        let pixels = buffer.as_bytes_mut();
        let len = pixels.len().min(self.trail_buffer.len());
        pixels[..len].copy_from_slice(&self.trail_buffer[..len]);
    }

    fn name(&self) -> &str {
        "Bobs"
    }
}
