//! Ethereal Ink — Energy flows between framed regions
//!
//! Tendrils of living light crawl across the wall between frames, chaining
//! left-to-right. With 2 frames you get source→sink; with 3+ frames,
//! energy cascades through each pair in sequence.

use super::Effect;
use crate::display::PixelBuffer;
use crate::regions::Scene;
use crate::util::{hsv_to_rgb, Rng};
use std::f32::consts::TAU;

// --- Flow field ---
const FIELD_W: usize = 80;
const FIELD_H: usize = 60;

// --- Tendrils ---
const MAX_TENDRILS: usize = 600;
const SPAWN_RATE: f32 = 80.0; // total tendrils/sec across all segments
const TENDRIL_SPEED: f32 = 120.0; // base px/s
const TENDRIL_MAX_AGE: f32 = 8.0; // seconds
const TENDRIL_STEER: f32 = 3.0; // how strongly the flow field steers velocity
const TARGET_PULL: f32 = 80.0; // base attraction toward next frame (px/s²)
const TARGET_DIST_SCALE: f32 = 0.4; // extra pull per pixel of distance

// --- Stain ---
const STAIN_FADE: f32 = 0.988;
const STAIN_DEPOSIT: u8 = 25;

// ============================================================================
// Noise (value noise + FBM)
// ============================================================================

fn noise_hash(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let mut h = seed.wrapping_add(x as u32).wrapping_mul(374761393);
    h = h.wrapping_add(y as u32).wrapping_mul(668265263);
    h = h.wrapping_add(z as u32).wrapping_mul(1274126177);
    h ^= h >> 13;
    h = h.wrapping_mul(1103515245);
    h ^= h >> 16;
    (h & 0x7fff) as f32 / 0x7fff as f32
}

fn value_noise(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let iz = z.floor() as i32;
    let fx = x - ix as f32;
    let fy = y - iy as f32;
    let fz = z - iz as f32;
    let sx = fx * fx * (3.0 - 2.0 * fx);
    let sy = fy * fy * (3.0 - 2.0 * fy);
    let sz = fz * fz * (3.0 - 2.0 * fz);

    let mut result = 0.0;
    for dz in 0..2 {
        for dy in 0..2 {
            for dx in 0..2 {
                let weight = match (dx, dy, dz) {
                    (0, 0, 0) => (1.0 - sx) * (1.0 - sy) * (1.0 - sz),
                    (1, 0, 0) => sx * (1.0 - sy) * (1.0 - sz),
                    (0, 1, 0) => (1.0 - sx) * sy * (1.0 - sz),
                    (1, 1, 0) => sx * sy * (1.0 - sz),
                    (0, 0, 1) => (1.0 - sx) * (1.0 - sy) * sz,
                    (1, 0, 1) => sx * (1.0 - sy) * sz,
                    (0, 1, 1) => (1.0 - sx) * sy * sz,
                    _ => sx * sy * sz,
                };
                result += weight * noise_hash(ix + dx, iy + dy, iz + dz, seed);
            }
        }
    }
    result
}

fn fbm(x: f32, y: f32, z: f32, octaves: u32, seed: u32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 0.5;
    let mut frequency = 1.0;
    for i in 0..octaves {
        value += amplitude * value_noise(x * frequency, y * frequency, z * frequency, seed + i);
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    value
}

// ============================================================================
// Data structures
// ============================================================================

struct Tendril {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    age: f32,
    prev_x: f32,
    prev_y: f32,
    hue_offset: f32,
    segment: u8, // which frame pair: frames[seg] → frames[seg+1]
}

struct FrameInfo {
    cx: f32,
    cy: f32,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

pub struct EtherealInk {
    tendrils: Vec<Tendril>,
    active: usize,

    field_vx: Vec<f32>,
    field_vy: Vec<f32>,

    stain: Vec<u8>,
    stain_w: u32,
    stain_h: u32,

    /// All detected frames sorted left-to-right by centroid X
    frames: Vec<FrameInfo>,

    rng: Rng,
    time: f32,
    spawn_accum: f32,
    scene_fingerprint: u64,
    screen_w: u32,
    screen_h: u32,
}

impl EtherealInk {
    pub fn new() -> Self {
        let mut tendrils = Vec::with_capacity(MAX_TENDRILS);
        for _ in 0..MAX_TENDRILS {
            tendrils.push(Tendril {
                x: 0.0,
                y: 0.0,
                vx: 0.0,
                vy: 0.0,
                age: 0.0,
                prev_x: 0.0,
                prev_y: 0.0,
                hue_offset: 0.0,
                segment: 0,
            });
        }
        Self {
            tendrils,
            active: 0,
            field_vx: vec![0.0; FIELD_W * FIELD_H],
            field_vy: vec![0.0; FIELD_W * FIELD_H],
            stain: Vec::new(),
            stain_w: 0,
            stain_h: 0,
            frames: Vec::new(),
            rng: Rng::new(0xE1F0),
            time: 0.0,
            spawn_accum: 0.0,
            scene_fingerprint: u64::MAX,
            screen_w: 0,
            screen_h: 0,
        }
    }

    fn scene_fingerprint(scene: &Scene) -> u64 {
        let mut h: u64 = scene.regions.len() as u64;
        for region in &scene.regions {
            for v in &region.polygon.vertices {
                h = h.wrapping_mul(31).wrapping_add(v.x.to_bits() as u64);
                h = h.wrapping_mul(31).wrapping_add(v.y.to_bits() as u64);
            }
        }
        h
    }

    fn rebuild_scene(&mut self, width: u32, height: u32, scene: &Scene, fingerprint: u64) {
        self.screen_w = width;
        self.screen_h = height;
        self.scene_fingerprint = fingerprint;

        let pixels = (width * height) as usize;
        if self.stain_w != width || self.stain_h != height {
            self.stain = vec![0u8; pixels];
            self.stain_w = width;
            self.stain_h = height;
        }

        // Collect all frames sorted left-to-right by centroid X
        self.frames.clear();
        for region in &scene.regions {
            if let Some((min_x, min_y, max_x, max_y)) = region.polygon.bounds() {
                if let Some(c) = region.polygon.centroid() {
                    self.frames.push(FrameInfo {
                        cx: c.x,
                        cy: c.y,
                        min_x,
                        min_y,
                        max_x,
                        max_y,
                    });
                }
            }
        }
        self.frames.sort_by(|a, b| a.cx.partial_cmp(&b.cx).unwrap());

        self.active = 0;
    }

    /// Pure noise-based flow field (no source/sink warping — per-tendril steering handles direction)
    fn update_field(&mut self) {
        let t = self.time * 0.15;

        for gy in 0..FIELD_H {
            for gx in 0..FIELD_W {
                let idx = gy * FIELD_W + gx;

                let nx = gx as f32 * 0.08;
                let ny = gy as f32 * 0.08;
                let angle = fbm(nx, ny, t, 3, 42) * TAU * 2.0;
                let speed = 0.3 + fbm(nx + 100.0, ny + 100.0, t, 2, 99) * 0.7;
                let vx = angle.cos() * speed;
                let vy = angle.sin() * speed;

                let len = (vx * vx + vy * vy).sqrt().max(0.001);
                self.field_vx[idx] = vx / len;
                self.field_vy[idx] = vy / len;
            }
        }
    }

    fn sample_field(&self, wx: f32, wy: f32) -> (f32, f32) {
        let w = self.screen_w as f32;
        let h = self.screen_h as f32;
        let gx = (wx / w * FIELD_W as f32) - 0.5;
        let gy = (wy / h * FIELD_H as f32) - 0.5;

        let ix = (gx.floor() as i32).clamp(0, FIELD_W as i32 - 2) as usize;
        let iy = (gy.floor() as i32).clamp(0, FIELD_H as i32 - 2) as usize;
        let fx = gx - ix as f32;
        let fy = gy - iy as f32;

        let i00 = iy * FIELD_W + ix;
        let i10 = i00 + 1;
        let i01 = i00 + FIELD_W;
        let i11 = i01 + 1;

        let vx = self.field_vx[i00] * (1.0 - fx) * (1.0 - fy)
            + self.field_vx[i10] * fx * (1.0 - fy)
            + self.field_vx[i01] * (1.0 - fx) * fy
            + self.field_vx[i11] * fx * fy;
        let vy = self.field_vy[i00] * (1.0 - fx) * (1.0 - fy)
            + self.field_vy[i10] * fx * (1.0 - fy)
            + self.field_vy[i01] * (1.0 - fx) * fy
            + self.field_vy[i11] * fx * fy;

        (vx, vy)
    }

    fn spawn_tendril(&mut self, segment: usize) {
        if self.active >= MAX_TENDRILS || segment >= self.frames.len() {
            return;
        }
        let src = &self.frames[segment];
        let src_cx = src.cx;
        let src_cy = src.cy;
        let src_min_x = src.min_x;
        let src_min_y = src.min_y;
        let src_max_x = src.max_x;
        let src_max_y = src.max_y;

        // Spawn along a random edge of the source frame
        let t = self.rng.next_f32();
        let side = self.rng.next_u32() % 4;
        let (x, y) = match side {
            0 => (src_min_x + t * (src_max_x - src_min_x), src_min_y),
            1 => (src_min_x + t * (src_max_x - src_min_x), src_max_y),
            2 => (src_min_x, src_min_y + t * (src_max_y - src_min_y)),
            _ => (src_max_x, src_min_y + t * (src_max_y - src_min_y)),
        };

        // Initial velocity: toward next frame if it exists, else outward from source
        let (dir_x, dir_y) = if segment + 1 < self.frames.len() {
            let tgt = &self.frames[segment + 1];
            (tgt.cx - x, tgt.cy - y)
        } else {
            (x - src_cx, y - src_cy)
        };
        let dist = (dir_x * dir_x + dir_y * dir_y).sqrt().max(1.0);
        let speed = self.rng.range_f32(TENDRIL_SPEED * 0.7, TENDRIL_SPEED * 1.3);

        let f = &mut self.tendrils[self.active];
        f.x = x;
        f.y = y;
        f.prev_x = x;
        f.prev_y = y;
        f.vx = dir_x / dist * speed;
        f.vy = dir_y / dist * speed;
        f.age = 0.0;
        f.hue_offset = self.rng.range_f32(0.0, 1.0);
        f.segment = segment as u8;
        self.active += 1;
    }

    fn inside_frame(frame: &FrameInfo, x: f32, y: f32) -> bool {
        x >= frame.min_x && x <= frame.max_x && y >= frame.min_y && y <= frame.max_y
    }

    fn deposit_stain(&mut self, x: f32, y: f32, brightness: u8) {
        let px = x as i32;
        let py = y as i32;
        let w = self.stain_w as i32;
        let h = self.stain_h as i32;
        for dy in -1..=1 {
            for dx in -1..=1 {
                let sx = px + dx;
                let sy = py + dy;
                if sx >= 0 && sx < w && sy >= 0 && sy < h {
                    let idx = (sy as u32 * self.stain_w + sx as u32) as usize;
                    self.stain[idx] = self.stain[idx].saturating_add(brightness);
                }
            }
        }
    }

    fn fade_stain(&mut self) {
        let factor = (STAIN_FADE * 256.0) as u16;
        for pixel in &mut self.stain {
            *pixel = ((*pixel as u16 * factor) >> 8) as u8;
        }
    }
}

impl Default for EtherealInk {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for EtherealInk {
    fn update(&mut self, dt: f32, width: u32, height: u32, scene: &Scene) {
        let fp = Self::scene_fingerprint(scene);
        if width != self.screen_w || height != self.screen_h || fp != self.scene_fingerprint {
            self.rebuild_scene(width, height, scene, fp);
        }

        self.time += dt;
        let w = width as f32;
        let h = height as f32;

        self.update_field();

        // Number of chain segments (pairs of consecutive frames)
        let num_segments = if self.frames.len() >= 2 {
            self.frames.len() - 1
        } else {
            usize::from(self.frames.len() == 1)
        };

        // Move tendrils
        let mut i = 0;
        while i < self.active {
            // Sample before taking mutable borrow
            let (fvx, fvy) = self.sample_field(self.tendrils[i].x, self.tendrils[i].y);
            let seg = self.tendrils[i].segment as usize;

            let t = &mut self.tendrils[i];
            t.prev_x = t.x;
            t.prev_y = t.y;
            t.age += dt;

            // Steer velocity toward flow field
            t.vx += (fvx * TENDRIL_SPEED - t.vx) * TENDRIL_STEER * dt;
            t.vy += (fvy * TENDRIL_SPEED - t.vy) * TENDRIL_STEER * dt;

            // Per-tendril attraction toward target frame
            // Scales with distance: gentle up close (organic), strong when far (ensures arrival)
            if seg + 1 < self.frames.len() {
                let dx = self.frames[seg + 1].cx - t.x;
                let dy = self.frames[seg + 1].cy - t.y;
                let dist = (dx * dx + dy * dy).sqrt().max(1.0);
                let pull = TARGET_PULL + dist * TARGET_DIST_SCALE;
                t.vx += dx / dist * pull * dt;
                t.vy += dy / dist * pull * dt;
            }

            t.x += t.vx * dt;
            t.y += t.vy * dt;

            // Kill conditions: too old, off screen, or consumed by target frame
            let mut dead = t.age > TENDRIL_MAX_AGE
                || t.x < -20.0
                || t.x > w + 20.0
                || t.y < -20.0
                || t.y > h + 20.0;

            if !dead
                && seg + 1 < self.frames.len()
                && Self::inside_frame(&self.frames[seg + 1], t.x, t.y)
            {
                dead = true;
            }

            let tx = t.x;
            let ty = t.y;
            let t_age = t.age;

            if !dead {
                let fade = 1.0 - (t_age / TENDRIL_MAX_AGE);
                let brightness = (STAIN_DEPOSIT as f32 * fade) as u8;
                self.deposit_stain(tx, ty, brightness);
            }

            if dead {
                self.active -= 1;
                self.tendrils.swap(i, self.active);
            } else {
                i += 1;
            }
        }

        self.fade_stain();

        // Spawn new tendrils distributed across all segments
        if num_segments > 0 {
            self.spawn_accum += SPAWN_RATE * dt;
            while self.spawn_accum >= 1.0 && self.active < MAX_TENDRILS {
                let seg = self.rng.next_u32() as usize % num_segments;
                self.spawn_tendril(seg);
                self.spawn_accum -= 1.0;
            }
        }
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        let w = buffer.width() as i32;
        let h = buffer.height() as i32;

        buffer.clear(5, 5, 12);

        // Render stain buffer (blue-purple ghostly trails)
        let buf_w = buffer.width();
        if self.stain_w == buf_w && self.stain_h == buffer.height() {
            let pixels = buffer.as_bytes_mut();
            for y in 0..h {
                for x in 0..w {
                    let si = (y as u32 * self.stain_w + x as u32) as usize;
                    let brightness = self.stain[si];
                    if brightness > 2 {
                        let pi = ((y as u32 * buf_w + x as u32) * 4) as usize;
                        // ABGR layout: vibrant purple-magenta-blue glow
                        pixels[pi + 1] = pixels[pi + 1].saturating_add(brightness); // B
                        pixels[pi + 2] = pixels[pi + 2].saturating_add(brightness / 3); // G
                        pixels[pi + 3] =
                            pixels[pi + 3].saturating_add((brightness as u16 * 2 / 3) as u8);
                        // R
                    }
                }
            }
        }

        // Glow halos around ALL frames with per-frame hue variation
        let num_frames = self.frames.len();
        for (fi, frame) in self.frames.iter().enumerate() {
            let hue_base = if num_frames > 1 {
                fi as f32 / (num_frames - 1) as f32 * 180.0
            } else {
                0.0
            };
            let pulse = (self.time * (1.5 + fi as f32 * 0.3) + fi as f32 * 1.2).sin() * 0.25 + 0.65;
            let size = ((frame.max_x - frame.min_x).max(frame.max_y - frame.min_y) * 0.7) as i32;
            let (r, g, b) = hsv_to_rgb(220.0 + hue_base, 0.6, pulse);
            buffer.fill_circle_gradient(frame.cx as i32, frame.cy as i32, size, r, g, b, 2.0);
        }

        // Render live tendrils as anti-aliased line segments
        for i in 0..self.active {
            let t = &self.tendrils[i];
            let age_frac = t.age / TENDRIL_MAX_AGE;
            let brightness = (1.0 - age_frac * 0.6).min(1.0);
            if brightness < 0.02 {
                continue;
            }

            // Per-segment hue shift so each chain link has a color theme
            let seg_hue = if num_frames > 1 {
                t.segment as f32 / (num_frames - 1).max(1) as f32 * 120.0
            } else {
                0.0
            };
            let hue =
                (t.hue_offset * 360.0 + seg_hue + self.time * 25.0 + age_frac * 120.0) % 360.0;
            let (r, g, b) = hsv_to_rgb(hue, 0.7, brightness);

            // 3 parallel AA lines for thickness
            let dx = t.x - t.prev_x;
            let dy = t.y - t.prev_y;
            let len = (dx * dx + dy * dy).sqrt().max(0.001);
            let nx = -dy / len * 1.2;
            let ny = dx / len * 1.2;

            buffer.line_aa_additive(t.prev_x, t.prev_y, t.x, t.y, r, g, b);
            let r2 = (r as u16 * 3 / 4) as u8;
            let g2 = (g as u16 * 3 / 4) as u8;
            let b2 = (b as u16 * 3 / 4) as u8;
            buffer.line_aa_additive(t.prev_x + nx, t.prev_y + ny, t.x + nx, t.y + ny, r2, g2, b2);
            buffer.line_aa_additive(t.prev_x - nx, t.prev_y - ny, t.x - nx, t.y - ny, r2, g2, b2);

            // Bright head dot on young tendrils
            if age_frac < 0.3 {
                let head_r = r.saturating_add(80);
                let head_g = g.saturating_add(80);
                let head_b = b.saturating_add(80);
                buffer.blend_pixel_additive(t.x as i32, t.y as i32, head_r, head_g, head_b);
                buffer.blend_pixel_additive(
                    t.x as i32 + 1,
                    t.y as i32,
                    head_r / 2,
                    head_g / 2,
                    head_b / 2,
                );
                buffer.blend_pixel_additive(
                    t.x as i32 - 1,
                    t.y as i32,
                    head_r / 2,
                    head_g / 2,
                    head_b / 2,
                );
                buffer.blend_pixel_additive(
                    t.x as i32,
                    t.y as i32 + 1,
                    head_r / 2,
                    head_g / 2,
                    head_b / 2,
                );
                buffer.blend_pixel_additive(
                    t.x as i32,
                    t.y as i32 - 1,
                    head_r / 2,
                    head_g / 2,
                    head_b / 2,
                );
            }
        }

        buffer.bloom(60, 4, 0.8);
    }

    fn name(&self) -> &str {
        "Ethereal Ink"
    }

    fn region_color(&self) -> (u8, u8, u8) {
        (5, 5, 12)
    }
}
