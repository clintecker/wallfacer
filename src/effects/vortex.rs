//! Vortex Effect
//!
//! Particles swirl in a mesmerizing spiral pattern,
//! getting pulled toward the center.

use super::Effect;
use crate::display::PixelBuffer;
use crate::regions::Scene;
use crate::util::Rng;

const NUM_PARTICLES: usize = 3000;

struct Particle {
    x: f32,
    y: f32,
    angle: f32,
    radius: f32,
    speed: f32,
    hue: f32,
    brightness: f32,
}

/// Swirling particle vortex
pub struct Vortex {
    particles: Vec<Particle>,
    rng: Rng,
    time: f32,
    center_x: f32,
    center_y: f32,
    width: u32,
    height: u32,
}

impl Vortex {
    pub fn new() -> Self {
        Self {
            particles: Vec::with_capacity(NUM_PARTICLES),
            rng: Rng::new(0x0123_4567),
            time: 0.0,
            center_x: 0.0,
            center_y: 0.0,
            width: 0,
            height: 0,
        }
    }

    fn init_particles(&mut self) {
        self.particles.clear();
        let max_radius = (self.width.max(self.height) as f32) * 0.6;

        for _ in 0..NUM_PARTICLES {
            let angle = self.rng.next_f32() * std::f32::consts::TAU;
            let radius = self.rng.next_f32().sqrt() * max_radius; // sqrt for uniform distribution
            let speed = 0.3 + self.rng.next_f32() * 0.7;

            self.particles.push(Particle {
                x: self.center_x + angle.cos() * radius,
                y: self.center_y + angle.sin() * radius,
                angle,
                radius,
                speed,
                hue: self.rng.next_f32() * 360.0,
                brightness: 0.5 + self.rng.next_f32() * 0.5,
            });
        }
    }

    fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
        let h = h % 360.0;
        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;

        let (r, g, b) = match (h / 60.0) as i32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };

        (
            ((r + m) * 255.0) as u8,
            ((g + m) * 255.0) as u8,
            ((b + m) * 255.0) as u8,
        )
    }
}

impl Default for Vortex {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Vortex {
    fn update(&mut self, dt: f32, width: u32, height: u32, _scene: &Scene) {
        if width != self.width || height != self.height {
            self.width = width;
            self.height = height;
            self.center_x = width as f32 / 2.0;
            self.center_y = height as f32 / 2.0;
            self.init_particles();
        }

        self.time += dt;
        let max_radius = (width.max(height) as f32) * 0.6;

        // Animate center position slightly
        let wobble = 20.0;
        self.center_x = width as f32 / 2.0 + (self.time * 0.3).sin() * wobble;
        self.center_y = height as f32 / 2.0 + (self.time * 0.4).cos() * wobble;

        for particle in &mut self.particles {
            // Angular velocity increases as particles get closer to center
            let angular_speed = particle.speed * (1.0 + 3.0 / (particle.radius / 50.0 + 1.0));
            particle.angle += angular_speed * dt;

            // Slowly spiral inward
            particle.radius -= dt * 15.0 * particle.speed;

            // Reset particles that reach center
            if particle.radius < 5.0 {
                particle.radius = max_radius;
                particle.angle = self.rng.next_f32() * std::f32::consts::TAU;
                particle.hue = (particle.hue + 30.0) % 360.0;
            }

            // Update position
            particle.x = self.center_x + particle.angle.cos() * particle.radius;
            particle.y = self.center_y + particle.angle.sin() * particle.radius;

            // Shift hue slowly
            particle.hue = (particle.hue + dt * 20.0) % 360.0;
        }
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        // Fade existing buffer for trail effect
        let pixels = buffer.as_bytes_mut();
        for chunk in pixels.chunks_exact_mut(4) {
            chunk[0] = (chunk[0] as u16 * 92 / 100) as u8;
            chunk[1] = (chunk[1] as u16 * 92 / 100) as u8;
            chunk[2] = (chunk[2] as u16 * 92 / 100) as u8;
        }

        let width = buffer.width();
        let height = buffer.height();

        // Draw particles
        for particle in &self.particles {
            let (r, g, b) = Self::hsv_to_rgb(particle.hue, 0.9, particle.brightness);

            let px = particle.x as i32;
            let py = particle.y as i32;

            if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                // Brighter particles closer to center
                let center_dist = particle.radius / (width.max(height) as f32 * 0.6);
                let glow = 1.0 - center_dist.min(1.0);

                let r = (r as f32 * (0.5 + glow * 0.5)) as u8;
                let g = (g as f32 * (0.5 + glow * 0.5)) as u8;
                let b = (b as f32 * (0.5 + glow * 0.5)) as u8;

                buffer.set_pixel(px, py, r, g, b);

                // Draw slightly larger for particles near center
                if glow > 0.5 {
                    buffer.set_pixel(px + 1, py, r / 2, g / 2, b / 2);
                    buffer.set_pixel(px, py + 1, r / 2, g / 2, b / 2);
                }
            }
        }

        // Draw bright center
        let cx = self.center_x as i32;
        let cy = self.center_y as i32;
        for r in 0..8 {
            let intensity = 255 - r * 25;
            for dy in -(r as i32)..=(r as i32) {
                for dx in -(r as i32)..=(r as i32) {
                    if dx * dx + dy * dy <= r as i32 * r as i32 {
                        buffer.set_pixel(cx + dx, cy + dy, intensity as u8, intensity as u8, 255);
                    }
                }
            }
        }
    }

    fn name(&self) -> &str {
        "Vortex"
    }

    fn region_color(&self) -> (u8, u8, u8) {
        (20, 0, 40)
    }
}
