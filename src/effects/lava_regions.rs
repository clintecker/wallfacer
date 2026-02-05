//! Lava Regions Effect
//!
//! Regions glow with animated molten lava and emit rising embers.
//! The lava churns and pulses while sparks float upward.

use super::Effect;
use crate::display::PixelBuffer;
use crate::regions::{Scene, Shape};
use crate::util::Rng;

const MAX_EMBERS: usize = 200;

/// A rising ember particle
struct Ember {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
    max_life: f32,
    size: f32,
}

/// Lava-filled regions with rising embers
pub struct LavaRegions {
    embers: Vec<Ember>,
    rng: Rng,
    time: f32,
    width: u32,
    height: u32,
    // Precomputed noise for lava animation
    noise: Vec<f32>,
}

impl LavaRegions {
    pub fn new() -> Self {
        // Generate simplex-like noise table
        let mut rng = Rng::new(0xA1A_1234);
        let noise: Vec<f32> = (0..256).map(|_| rng.next_f32()).collect();

        Self {
            embers: Vec::with_capacity(MAX_EMBERS),
            rng: Rng::new(0xE8B5_5678),
            time: 0.0,
            width: 0,
            height: 0,
            noise,
        }
    }

    fn noise_at(&self, x: f32, y: f32, t: f32) -> f32 {
        // Simple animated noise using the precomputed table
        let ix = ((x * 0.05 + t * 0.5).sin() * 127.0 + 128.0) as usize % 256;
        let iy = ((y * 0.05 + t * 0.3).cos() * 127.0 + 128.0) as usize % 256;
        let it = ((t * 2.0).sin() * 127.0 + 128.0) as usize % 256;

        let n1 = self.noise[ix];
        let n2 = self.noise[iy];
        let n3 = self.noise[it];
        let n4 = self.noise[(ix + iy) % 256];

        (n1 + n2 + n3 + n4) / 4.0
    }

    fn lava_color(&self, intensity: f32) -> (u8, u8, u8) {
        // Lava color gradient: dark red -> orange -> yellow -> white
        let t = intensity.clamp(0.0, 1.0);

        if t < 0.3 {
            // Dark red to red
            let t2 = t / 0.3;
            ((80.0 + t2 * 175.0) as u8, (t2 * 30.0) as u8, 0)
        } else if t < 0.6 {
            // Red to orange
            let t2 = (t - 0.3) / 0.3;
            (255, (30.0 + t2 * 130.0) as u8, 0)
        } else if t < 0.85 {
            // Orange to yellow
            let t2 = (t - 0.6) / 0.25;
            (255, (160.0 + t2 * 95.0) as u8, (t2 * 100.0) as u8)
        } else {
            // Yellow to white hot
            let t2 = (t - 0.85) / 0.15;
            (255, 255, (100.0 + t2 * 155.0) as u8)
        }
    }

    fn spawn_ember(&mut self, x: f32, y: f32) {
        if self.embers.len() >= MAX_EMBERS {
            // Remove oldest
            self.embers.remove(0);
        }

        let max_life = 1.5 + self.rng.next_f32() * 2.0;
        self.embers.push(Ember {
            x,
            y,
            vx: (self.rng.next_f32() - 0.5) * 30.0,
            vy: -50.0 - self.rng.next_f32() * 80.0, // Rise upward
            life: max_life,
            max_life,
            size: 1.0 + self.rng.next_f32() * 2.0,
        });
    }
}

impl Default for LavaRegions {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for LavaRegions {
    fn update(&mut self, dt: f32, width: u32, height: u32, scene: &Scene) {
        self.width = width;
        self.height = height;
        self.time += dt;

        // Update embers
        self.embers.retain_mut(|ember| {
            ember.life -= dt;
            if ember.life <= 0.0 {
                return false;
            }

            // Apply slight wind and deceleration
            ember.vx *= 0.99;
            ember.vy *= 0.98;
            ember.vy -= 20.0 * dt; // Buoyancy

            ember.x += ember.vx * dt;
            ember.y += ember.vy * dt;

            true
        });

        // Spawn embers from region surfaces
        for region in &scene.regions {
            // Spawn rate based on region size
            let spawn_chance = dt * 30.0; // ~30 per second per region
            if self.rng.next_f32() < spawn_chance {
                match region.get_shape() {
                    Shape::Polygon(poly) => {
                        if let Some((min_x, min_y, max_x, _max_y)) = poly.bounds() {
                            // Spawn near top edge
                            let x = min_x + self.rng.next_f32() * (max_x - min_x);
                            self.spawn_ember(x, min_y);
                        }
                    }
                    Shape::Circle(circle) => {
                        // Spawn from top of circle
                        let angle = -std::f32::consts::PI * 0.5
                            + (self.rng.next_f32() - 0.5) * std::f32::consts::PI * 0.6;
                        let x = circle.center.x + angle.cos() * circle.radius;
                        let y = circle.center.y + angle.sin() * circle.radius;
                        self.spawn_ember(x, y);
                    }
                }
            }
        }
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        // Dark background
        buffer.clear(15, 5, 5);

        let width = buffer.width();
        let height = buffer.height();

        // Lava inside regions is handled by region_color() which returns animated lava color

        // Draw glowing lava inside regions
        // We need to iterate over pixels that are inside regions
        // For efficiency, iterate over region bounding boxes

        // Note: Since mask_regions will overwrite with region_color(),
        // we return a special color and handle the lava drawing here before masking

        // For each region, fill with animated lava
        // (We'll read back the scene in a moment - for now just draw based on stored bounds)

        // Draw embers
        for ember in &self.embers {
            let life_ratio = ember.life / ember.max_life;
            let (r, g, b) = self.lava_color(0.7 + life_ratio * 0.3);

            // Fade out as life decreases
            let alpha = life_ratio;
            let r = (r as f32 * alpha) as u8;
            let g = (g as f32 * alpha) as u8;
            let b = (b as f32 * alpha) as u8;

            let size = (ember.size * life_ratio) as i32;
            let cx = ember.x as i32;
            let cy = ember.y as i32;

            // Draw ember as small glowing dot
            for dy in -size..=size {
                for dx in -size..=size {
                    if dx * dx + dy * dy <= size * size {
                        let px = cx + dx;
                        let py = cy + dy;
                        if px >= 0 && px < width as i32 && py >= 0 && py < height as i32 {
                            // Additive blend
                            let idx = ((py as u32 * width + px as u32) * 4) as usize;
                            let pixels = buffer.as_bytes_mut();
                            if idx + 2 < pixels.len() {
                                pixels[idx] = pixels[idx].saturating_add(r);
                                pixels[idx + 1] = pixels[idx + 1].saturating_add(g);
                                pixels[idx + 2] = pixels[idx + 2].saturating_add(b);
                            }
                        }
                    }
                }
            }
        }

        // Draw ambient glow at bottom
        for y in (height * 3 / 4)..height {
            let glow = ((y - height * 3 / 4) as f32 / (height / 4) as f32) * 0.3;
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;
                let pixels = buffer.as_bytes_mut();
                if idx + 2 < pixels.len() {
                    pixels[idx] = (pixels[idx] as f32 + 60.0 * glow).min(255.0) as u8;
                    pixels[idx + 1] = (pixels[idx + 1] as f32 + 20.0 * glow).min(255.0) as u8;
                }
            }
        }
    }

    fn name(&self) -> &str {
        "Lava Regions"
    }

    fn region_color(&self) -> (u8, u8, u8) {
        // Regions will be filled with animated lava color
        // Use time-based color cycling
        let t = (self.time * 0.5).sin() * 0.5 + 0.5;
        self.lava_color(0.4 + t * 0.3)
    }
}
