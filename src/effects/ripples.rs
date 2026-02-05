//! Shockwave Ripples — Concentric waves emanate from framed regions
//!
//! Each frame periodically drops a "stone" impulse into a 2D wave simulation.
//! Waves from different frames interfere — constructive and destructive patterns
//! create a liquid mercury look on the wall surface.

use super::Effect;
use crate::display::PixelBuffer;
use crate::regions::Scene;
use crate::util::{hsv_to_rgb, Rng};

/// Wave simulation grid resolution (lower = faster, chunkier)
const GRID_SCALE: u32 = 3; // 1 grid cell = 3x3 pixels
const DAMPING: f32 = 0.995;
const WAVE_SPEED: f32 = 0.4; // propagation per step (< 0.5 for stability)
const IMPULSE_MIN_INTERVAL: f32 = 0.8;
const IMPULSE_MAX_INTERVAL: f32 = 2.5;
const IMPULSE_STRENGTH: f32 = 80.0;
const IMPULSE_RADIUS: i32 = 3; // grid cells

struct FrameInfo {
    cx: f32,
    cy: f32,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
    next_impulse: f32,
}

pub struct Ripples {
    /// Wave height at each grid cell
    height: Vec<f32>,
    /// Wave velocity at each grid cell
    velocity: Vec<f32>,
    grid_w: u32,
    grid_h: u32,

    frames: Vec<FrameInfo>,

    rng: Rng,
    time: f32,
    scene_fingerprint: u64,
    screen_w: u32,
    screen_h: u32,
}

impl Ripples {
    pub fn new() -> Self {
        Self {
            height: Vec::new(),
            velocity: Vec::new(),
            grid_w: 0,
            grid_h: 0,
            frames: Vec::new(),
            rng: Rng::new(0xD20F),
            time: 0.0,
            scene_fingerprint: u64::MAX,
            screen_w: 0,
            screen_h: 0,
        }
    }

    fn scene_fingerprint(scene: &Scene) -> u64 {
        let mut h: u64 = scene.regions.len() as u64;
        for region in &scene.regions {
            // Use bounds for fingerprinting - works for both polygons and circles
            if let Some((min_x, min_y, max_x, max_y)) = region.get_shape().bounds() {
                h = h.wrapping_mul(31).wrapping_add(min_x.to_bits() as u64);
                h = h.wrapping_mul(31).wrapping_add(min_y.to_bits() as u64);
                h = h.wrapping_mul(31).wrapping_add(max_x.to_bits() as u64);
                h = h.wrapping_mul(31).wrapping_add(max_y.to_bits() as u64);
            }
        }
        h
    }

    fn rebuild_scene(&mut self, width: u32, height: u32, scene: &Scene, fingerprint: u64) {
        self.screen_w = width;
        self.screen_h = height;
        self.scene_fingerprint = fingerprint;

        self.grid_w = width.div_ceil(GRID_SCALE);
        self.grid_h = height.div_ceil(GRID_SCALE);
        let cells = (self.grid_w * self.grid_h) as usize;
        self.height = vec![0.0; cells];
        self.velocity = vec![0.0; cells];

        self.frames.clear();
        for region in &scene.regions {
            let shape = region.get_shape();
            if let Some((min_x, min_y, max_x, max_y)) = shape.bounds() {
                if let Some(c) = shape.centroid() {
                    self.frames.push(FrameInfo {
                        cx: c.x,
                        cy: c.y,
                        min_x,
                        min_y,
                        max_x,
                        max_y,
                        next_impulse: self.rng.range_f32(0.0, 0.5),
                    });
                }
            }
        }
        self.frames.sort_by(|a, b| a.cx.partial_cmp(&b.cx).unwrap());
    }

    /// Drop a circular impulse into the wave grid
    fn drop_impulse(&mut self, world_x: f32, world_y: f32, strength: f32) {
        let gx = (world_x / GRID_SCALE as f32) as i32;
        let gy = (world_y / GRID_SCALE as f32) as i32;
        let gw = self.grid_w as i32;
        let gh = self.grid_h as i32;

        for dy in -IMPULSE_RADIUS..=IMPULSE_RADIUS {
            for dx in -IMPULSE_RADIUS..=IMPULSE_RADIUS {
                let px = gx + dx;
                let py = gy + dy;
                if px >= 0 && px < gw && py >= 0 && py < gh {
                    let dist = ((dx * dx + dy * dy) as f32).sqrt();
                    if dist <= IMPULSE_RADIUS as f32 {
                        let falloff = 1.0 - dist / IMPULSE_RADIUS as f32;
                        let idx = (py as u32 * self.grid_w + px as u32) as usize;
                        self.height[idx] += strength * falloff;
                    }
                }
            }
        }
    }

    /// Step the wave equation (Laplacian + velocity Verlet)
    fn step_wave(&mut self) {
        let gw = self.grid_w as i32;
        let gh = self.grid_h as i32;

        for y in 1..gh - 1 {
            for x in 1..gw - 1 {
                let idx = (y * gw + x) as usize;
                // Discrete Laplacian (4-neighbor average minus center)
                let neighbors = self.height[idx - 1]
                    + self.height[idx + 1]
                    + self.height[(idx as i32 - gw) as usize]
                    + self.height[(idx as i32 + gw) as usize];
                let laplacian = neighbors * 0.25 - self.height[idx];

                self.velocity[idx] += laplacian * WAVE_SPEED;
                self.velocity[idx] *= DAMPING;
            }
        }

        // Apply velocity to height
        let cells = (self.grid_w * self.grid_h) as usize;
        for i in 0..cells {
            self.height[i] += self.velocity[i];
        }
    }
}

impl Default for Ripples {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Ripples {
    fn update(&mut self, dt: f32, width: u32, height: u32, scene: &Scene) {
        let fp = Self::scene_fingerprint(scene);
        if width != self.screen_w || height != self.screen_h || fp != self.scene_fingerprint {
            self.rebuild_scene(width, height, scene, fp);
        }

        self.time += dt;

        // Drop impulses from frames on timer
        for fi in 0..self.frames.len() {
            self.frames[fi].next_impulse -= dt;
            if self.frames[fi].next_impulse <= 0.0 {
                // Random point along frame edge
                let t = self.rng.next_f32();
                let frame = &self.frames[fi];
                let (ix, iy) = match self.rng.next_u32() % 4 {
                    0 => (frame.min_x + t * (frame.max_x - frame.min_x), frame.min_y),
                    1 => (frame.min_x + t * (frame.max_x - frame.min_x), frame.max_y),
                    2 => (frame.min_x, frame.min_y + t * (frame.max_y - frame.min_y)),
                    _ => (frame.max_x, frame.min_y + t * (frame.max_y - frame.min_y)),
                };

                // Alternate positive/negative impulses for variety
                let sign = if self.rng.next_f32() < 0.5 { 1.0 } else { -1.0 };
                self.drop_impulse(ix, iy, IMPULSE_STRENGTH * sign);

                self.frames[fi].next_impulse = self
                    .rng
                    .range_f32(IMPULSE_MIN_INTERVAL, IMPULSE_MAX_INTERVAL);
            }
        }

        // Run multiple wave steps per frame for faster propagation
        for _ in 0..3 {
            self.step_wave();
        }
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        let w = buffer.width() as i32;
        let h = buffer.height() as i32;
        let gw = self.grid_w;

        // Dark background
        buffer.clear(4, 4, 12);

        let buf_w = buffer.width();
        let pixels = buffer.as_bytes_mut();

        for py in 0..h {
            let gy = (py as u32 / GRID_SCALE).min(self.grid_h - 1);
            for px in 0..w {
                let gx = (px as u32 / GRID_SCALE).min(gw - 1);
                let gi = (gy * gw + gx) as usize;
                let val = self.height[gi];

                // Map wave height to color
                // Positive = bright blue-white, negative = dark blue-purple
                let pi = ((py as u32 * buf_w + px as u32) * 4) as usize;

                if val > 0.5 {
                    // Positive wave: blue → cyan → white
                    let t = (val / 40.0).min(1.0);
                    let r = (t * 180.0) as u8;
                    let g = (t * 200.0) as u8;
                    let b = (80.0 + t * 175.0) as u8;
                    pixels[pi] = 255; // A
                    pixels[pi + 1] = pixels[pi + 1].saturating_add(b);
                    pixels[pi + 2] = pixels[pi + 2].saturating_add(g);
                    pixels[pi + 3] = pixels[pi + 3].saturating_add(r);
                } else if val < -0.5 {
                    // Negative wave: dark purple-blue
                    let t = (-val / 40.0).min(1.0);
                    let r = (t * 60.0) as u8;
                    let g = (t * 20.0) as u8;
                    let b = (t * 100.0) as u8;
                    pixels[pi] = 255;
                    pixels[pi + 1] = pixels[pi + 1].saturating_add(b);
                    pixels[pi + 2] = pixels[pi + 2].saturating_add(g);
                    pixels[pi + 3] = pixels[pi + 3].saturating_add(r);
                }
            }
        }

        // Subtle glow halos around frames
        for (fi, frame) in self.frames.iter().enumerate() {
            let hue = if self.frames.len() > 1 {
                fi as f32 / (self.frames.len() - 1) as f32 * 120.0 + 200.0
            } else {
                240.0
            };
            let pulse = (self.time * 1.2 + fi as f32).sin() * 0.15 + 0.3;
            let size = ((frame.max_x - frame.min_x).max(frame.max_y - frame.min_y) * 0.4) as i32;
            let (r, g, b) = hsv_to_rgb(hue, 0.5, pulse);
            buffer.fill_circle_gradient(frame.cx as i32, frame.cy as i32, size, r, g, b, 2.0);
        }

        buffer.bloom(30, 3, 0.4);
    }

    fn name(&self) -> &str {
        "Shockwave Ripples"
    }

    fn region_color(&self) -> (u8, u8, u8) {
        (4, 4, 12)
    }
}
