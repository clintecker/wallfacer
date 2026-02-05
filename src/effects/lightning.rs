//! Lightning Storm — Electric arcs crackling between framed regions
//!
//! Fractal lightning bolts bridge the gaps between frames with bright
//! white-blue flashes and purple afterglow. Chain lightning cascades
//! through all frames in sequence.

use super::Effect;
use crate::display::PixelBuffer;
use crate::regions::Scene;
use crate::util::Rng;
use std::f32::consts::TAU;

const MAX_BOLTS: usize = 16;
const BOLT_MIN_INTERVAL: f32 = 0.3;
const BOLT_MAX_INTERVAL: f32 = 1.5;
const BOLT_LIFETIME: f32 = 0.45;
const SUBDIVISION_LEVELS: u32 = 5; // 2^5 = 32 segments per bolt
const BRANCH_PROBABILITY: f32 = 0.3;
const OFFSET_SCALE: f32 = 0.25;
const CHAIN_STAGGER: f32 = 0.06; // seconds between chain segments

const STAIN_FADE: f32 = 0.955; // fast fade for snappy afterglow
const STAIN_DEPOSIT: u8 = 50;

const AMBIENT_SPARK_RATE: f32 = 8.0; // sparks/sec per frame

struct BoltPath {
    points: Vec<(f32, f32)>,
    brightness: f32, // 1.0 for main bolt, 0.3-0.7 for branches
}

struct Bolt {
    paths: Vec<BoltPath>,
    age: f32, // negative = delayed (chain stagger)
    lifetime: f32,
}

struct FrameInfo {
    cx: f32,
    cy: f32,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

pub struct LightningStorm {
    bolts: Vec<Bolt>,
    frames: Vec<FrameInfo>,

    stain: Vec<u8>,
    stain_w: u32,
    stain_h: u32,

    rng: Rng,
    time: f32,
    next_bolt: f32,
    spark_accum: f32,
    scene_fingerprint: u64,
    screen_w: u32,
    screen_h: u32,
}

impl LightningStorm {
    pub fn new() -> Self {
        Self {
            bolts: Vec::new(),
            frames: Vec::new(),
            stain: Vec::new(),
            stain_w: 0,
            stain_h: 0,
            rng: Rng::new(0xB017),
            time: 0.0,
            next_bolt: 0.5,
            spark_accum: 0.0,
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

        self.bolts.clear();
    }

    /// Pick a point on the frame edge facing toward another frame
    fn facing_edge_point(frame: &FrameInfo, toward: &FrameInfo, rng: &mut Rng) -> (f32, f32) {
        let t = rng.next_f32();
        let dx = toward.cx - frame.cx;
        let dy = toward.cy - frame.cy;

        if dx.abs() > dy.abs() {
            if dx > 0.0 {
                (frame.max_x, frame.min_y + t * (frame.max_y - frame.min_y))
            } else {
                (frame.min_x, frame.min_y + t * (frame.max_y - frame.min_y))
            }
        } else if dy > 0.0 {
            (frame.min_x + t * (frame.max_x - frame.min_x), frame.max_y)
        } else {
            (frame.min_x + t * (frame.max_x - frame.min_x), frame.min_y)
        }
    }

    /// Random point on any edge of a frame
    fn random_edge_point(frame: &FrameInfo, rng: &mut Rng) -> (f32, f32) {
        let t = rng.next_f32();
        match rng.next_u32() % 4 {
            0 => (frame.min_x + t * (frame.max_x - frame.min_x), frame.min_y),
            1 => (frame.min_x + t * (frame.max_x - frame.min_x), frame.max_y),
            2 => (frame.min_x, frame.min_y + t * (frame.max_y - frame.min_y)),
            _ => (frame.max_x, frame.min_y + t * (frame.max_y - frame.min_y)),
        }
    }

    /// Generate a fractal bolt path via iterative midpoint displacement
    fn generate_path(
        start: (f32, f32),
        end: (f32, f32),
        levels: u32,
        rng: &mut Rng,
    ) -> Vec<(f32, f32)> {
        let mut points = vec![start, end];

        for level in 0..levels {
            let scale = OFFSET_SCALE / (1.4_f32).powi(level as i32);
            let mut new_points = Vec::with_capacity(points.len() * 2);

            for i in 0..points.len() - 1 {
                let (ax, ay) = points[i];
                let (bx, by) = points[i + 1];

                let mx = (ax + bx) * 0.5;
                let my = (ay + by) * 0.5;
                let dx = bx - ax;
                let dy = by - ay;
                let seg_len = (dx * dx + dy * dy).sqrt().max(0.001);

                let nx = -dy / seg_len;
                let ny = dx / seg_len;
                let offset = rng.range_f32(-1.0, 1.0) * seg_len * scale;

                new_points.push(points[i]);
                new_points.push((mx + nx * offset, my + ny * offset));
            }
            new_points.push(*points.last().unwrap());
            points = new_points;
        }

        points
    }

    /// Generate branches off the main bolt path
    fn generate_branches(main_path: &[(f32, f32)], rng: &mut Rng) -> Vec<BoltPath> {
        let mut branches = Vec::new();
        let len = main_path.len();
        if len < 6 {
            return branches;
        }

        for i in (2..len - 2).step_by(4) {
            if rng.next_f32() > BRANCH_PROBABILITY {
                continue;
            }

            let (px, py) = main_path[i];
            // Direction from previous to next point on main bolt
            let (prev_x, prev_y) = main_path[i.saturating_sub(1)];
            let dx = px - prev_x;
            let dy = py - prev_y;
            let base_angle = dy.atan2(dx);

            // Branch off at 20-60 degrees, random side
            let side = if rng.next_f32() < 0.5 { 1.0 } else { -1.0 };
            let angle = base_angle + side * rng.range_f32(0.35, 1.0);
            let branch_len = rng.range_f32(30.0, 100.0);

            let end = (px + angle.cos() * branch_len, py + angle.sin() * branch_len);
            let points = Self::generate_path((px, py), end, 3, rng);

            branches.push(BoltPath {
                points,
                brightness: rng.range_f32(0.3, 0.6),
            });
        }

        branches
    }

    /// Fire chain lightning through all frames
    fn fire_chain(&mut self) {
        if self.frames.len() < 2 {
            return;
        }

        let num_segments = self.frames.len() - 1;
        for seg in 0..num_segments {
            if self.bolts.len() >= MAX_BOLTS {
                break;
            }

            let start =
                Self::facing_edge_point(&self.frames[seg], &self.frames[seg + 1], &mut self.rng);
            let end =
                Self::facing_edge_point(&self.frames[seg + 1], &self.frames[seg], &mut self.rng);

            let main_path = Self::generate_path(start, end, SUBDIVISION_LEVELS, &mut self.rng);
            let mut paths = Vec::new();

            // Generate branches
            let branches = Self::generate_branches(&main_path, &mut self.rng);

            paths.push(BoltPath {
                points: main_path,
                brightness: 1.0,
            });
            paths.extend(branches);

            self.bolts.push(Bolt {
                paths,
                age: -(seg as f32 * CHAIN_STAGGER), // stagger for cascade effect
                lifetime: BOLT_LIFETIME + self.rng.range_f32(-0.05, 0.1),
            });
        }
    }

    /// Spawn a small ambient spark near a random frame edge
    fn spawn_spark(&mut self) {
        if self.frames.is_empty() || self.bolts.len() >= MAX_BOLTS {
            return;
        }
        let fi = self.rng.next_u32() as usize % self.frames.len();
        let frame = &self.frames[fi];
        let start = Self::random_edge_point(frame, &mut self.rng);

        // Short random direction
        let angle = self.rng.range_f32(0.0, TAU);
        let spark_len = self.rng.range_f32(15.0, 50.0);
        let end = (
            start.0 + angle.cos() * spark_len,
            start.1 + angle.sin() * spark_len,
        );

        let points = Self::generate_path(start, end, 3, &mut self.rng);
        self.bolts.push(Bolt {
            paths: vec![BoltPath {
                points,
                brightness: self.rng.range_f32(0.3, 0.6),
            }],
            age: 0.0,
            lifetime: self.rng.range_f32(0.1, 0.2),
        });
    }

    fn deposit_stain(&mut self, x: f32, y: f32, amount: u8) {
        let px = x as i32;
        let py = y as i32;
        let w = self.stain_w as i32;
        let h = self.stain_h as i32;
        if px >= 0 && px < w && py >= 0 && py < h {
            let idx = (py as u32 * self.stain_w + px as u32) as usize;
            self.stain[idx] = self.stain[idx].saturating_add(amount);
        }
    }

    fn fade_stain(&mut self) {
        let factor = (STAIN_FADE * 256.0) as u16;
        for pixel in &mut self.stain {
            *pixel = ((*pixel as u16 * factor) >> 8) as u8;
        }
    }
}

impl Default for LightningStorm {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for LightningStorm {
    fn update(&mut self, dt: f32, width: u32, height: u32, scene: &Scene) {
        let fp = Self::scene_fingerprint(scene);
        if width != self.screen_w || height != self.screen_h || fp != self.scene_fingerprint {
            self.rebuild_scene(width, height, scene, fp);
        }

        self.time += dt;

        // Fire chain lightning on timer
        self.next_bolt -= dt;
        if self.next_bolt <= 0.0 && self.frames.len() >= 2 {
            self.fire_chain();
            self.next_bolt = self.rng.range_f32(BOLT_MIN_INTERVAL, BOLT_MAX_INTERVAL);
        }

        // Ambient sparks
        if !self.frames.is_empty() {
            self.spark_accum += AMBIENT_SPARK_RATE * self.frames.len() as f32 * dt;
            while self.spark_accum >= 1.0 {
                self.spawn_spark();
                self.spark_accum -= 1.0;
            }
        }

        // Age bolts and deposit stain
        let mut i = 0;
        while i < self.bolts.len() {
            self.bolts[i].age += dt;

            let age = self.bolts[i].age;
            let lifetime = self.bolts[i].lifetime;

            if age > lifetime {
                self.bolts.swap_remove(i);
                continue;
            }

            // Deposit to stain while bolt is visible
            if age >= 0.0 {
                let flash = bolt_brightness(age, lifetime);
                let deposit = (STAIN_DEPOSIT as f32 * flash) as u8;
                if deposit > 0 {
                    // Deposit along main path only (paths[0])
                    let path_len = self.bolts[i].paths[0].points.len();
                    for pi in (0..path_len).step_by(2) {
                        let (px, py) = self.bolts[i].paths[0].points[pi];
                        self.deposit_stain(px, py, deposit);
                    }
                }
            }

            i += 1;
        }

        self.fade_stain();
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        let w = buffer.width() as i32;
        let h = buffer.height() as i32;

        // Dark stormy background
        for row in 0..h {
            let t = row as f32 / h as f32;
            let r = (3.0 + t * 5.0) as u8;
            let g = (3.0 + t * 4.0) as u8;
            let b = (8.0 + t * 10.0) as u8;
            buffer.hline(0, w - 1, row, r, g, b);
        }

        // Stain buffer — blue-purple afterglow
        let buf_w = buffer.width();
        if self.stain_w == buf_w && self.stain_h == buffer.height() {
            let pixels = buffer.as_bytes_mut();
            for y in 0..h {
                for x in 0..w {
                    let si = (y as u32 * self.stain_w + x as u32) as usize;
                    let brightness = self.stain[si];
                    if brightness > 2 {
                        let pi = ((y as u32 * buf_w + x as u32) * 4) as usize;
                        // ABGR: blue-purple afterglow
                        pixels[pi + 1] =
                            pixels[pi + 1].saturating_add((brightness as u16 * 3 / 4) as u8); // B
                        pixels[pi + 2] = pixels[pi + 2].saturating_add(brightness / 6); // G
                        pixels[pi + 3] =
                            pixels[pi + 3].saturating_add((brightness as u16 * 2 / 5) as u8);
                        // R
                    }
                }
            }
        }

        // Frame edge glow (subtle ambient)
        for frame in &self.frames {
            let pulse = (self.time * 0.8).sin() * 0.1 + 0.15;
            let size = ((frame.max_x - frame.min_x).max(frame.max_y - frame.min_y) * 0.5) as i32;
            let r = (30.0 * pulse) as u8;
            let g = (20.0 * pulse) as u8;
            let b = (80.0 * pulse) as u8;
            buffer.fill_circle_gradient(frame.cx as i32, frame.cy as i32, size, r, g, b, 2.0);
        }

        // Render active bolts
        for bolt in &self.bolts {
            if bolt.age < 0.0 {
                continue; // delayed by chain stagger
            }
            let flash = bolt_brightness(bolt.age, bolt.lifetime);
            if flash < 0.01 {
                continue;
            }

            for path in &bolt.paths {
                let b = flash * path.brightness;
                // Color: white core fading to blue
                let core_mix = (flash * 2.0).min(1.0); // white when fresh, blue when fading
                let r = (180.0 * b + 75.0 * core_mix * b) as u8;
                let g = (190.0 * b + 65.0 * core_mix * b) as u8;
                let bl = (255.0 * b) as u8;

                // Draw polyline
                for i in 0..path.points.len() - 1 {
                    let (x0, y0) = path.points[i];
                    let (x1, y1) = path.points[i + 1];

                    if path.brightness >= 0.9 {
                        // Main bolt: thick (3 parallel lines)
                        let dx = x1 - x0;
                        let dy = y1 - y0;
                        let len = (dx * dx + dy * dy).sqrt().max(0.001);
                        let nx = -dy / len * 0.8;
                        let ny = dx / len * 0.8;

                        buffer.line_aa_additive(x0, y0, x1, y1, r, g, bl);
                        let r2 = r / 2;
                        let g2 = g / 2;
                        let bl2 = bl / 2;
                        buffer.line_aa_additive(x0 + nx, y0 + ny, x1 + nx, y1 + ny, r2, g2, bl2);
                        buffer.line_aa_additive(x0 - nx, y0 - ny, x1 - nx, y1 - ny, r2, g2, bl2);
                    } else {
                        // Branch: single thinner line
                        buffer.line_aa_additive(x0, y0, x1, y1, r, g, bl);
                    }
                }
            }

            // Bright flash on frame edges for fresh bolts
            if bolt.age < 0.08 {
                for frame in &self.frames {
                    // Check if bolt starts/ends near this frame
                    let main = &bolt.paths[0].points;
                    let (sx, sy) = main[0];
                    let (ex, ey) = *main.last().unwrap();

                    let flash_i = ((1.0 - bolt.age / 0.08) * 200.0) as u8;
                    let size =
                        ((frame.max_x - frame.min_x).max(frame.max_y - frame.min_y) * 0.4) as i32;

                    if sx >= frame.min_x - 5.0
                        && sx <= frame.max_x + 5.0
                        && sy >= frame.min_y - 5.0
                        && sy <= frame.max_y + 5.0
                    {
                        buffer.fill_circle_gradient(
                            frame.cx as i32,
                            frame.cy as i32,
                            size,
                            flash_i / 2,
                            flash_i / 3,
                            flash_i,
                            2.5,
                        );
                    }
                    if ex >= frame.min_x - 5.0
                        && ex <= frame.max_x + 5.0
                        && ey >= frame.min_y - 5.0
                        && ey <= frame.max_y + 5.0
                    {
                        buffer.fill_circle_gradient(
                            frame.cx as i32,
                            frame.cy as i32,
                            size,
                            flash_i / 2,
                            flash_i / 3,
                            flash_i,
                            2.5,
                        );
                    }
                }
            }
        }

        buffer.bloom(40, 3, 0.6);
    }

    fn name(&self) -> &str {
        "Lightning Storm"
    }

    fn region_color(&self) -> (u8, u8, u8) {
        (4, 4, 10)
    }
}

/// Bolt brightness curve: bright flash then quick fade
fn bolt_brightness(age: f32, lifetime: f32) -> f32 {
    let t = age / lifetime;
    if t < 0.1 {
        // Ramp up
        t / 0.1
    } else {
        // Fade out (quadratic)
        let fade = (t - 0.1) / 0.9;
        (1.0 - fade * fade).max(0.0)
    }
}
