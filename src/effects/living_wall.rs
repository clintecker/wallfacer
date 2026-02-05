//! Living Wall — Reaction-Diffusion Turing patterns around framed regions
//!
//! Gray-Scott model produces organic, coral-like patterns that grow and
//! evolve on the wall. Frames act as obstacles — patterns flow around them.

use super::Effect;
use crate::display::PixelBuffer;
use crate::regions::Scene;
use crate::util::Rng;

/// Simulation grid resolution (1 cell = 2x2 pixels for decent detail)
const GRID_SCALE: u32 = 2;
/// Diffusion rates
const DU: f32 = 0.16;
const DV: f32 = 0.08;
/// Feed and kill rates (controls pattern type)
/// These values produce mitosis/cell-division patterns
const FEED: f32 = 0.035;
const KILL: f32 = 0.065;
/// Simulation steps per frame (more = faster evolution)
const STEPS_PER_FRAME: u32 = 8;

pub struct LivingWall {
    /// Chemical U concentration (substrate)
    u: Vec<f32>,
    /// Chemical V concentration (activator)
    v: Vec<f32>,
    /// Scratch buffers for double-buffering
    u_next: Vec<f32>,
    v_next: Vec<f32>,
    /// Obstacle mask: true where a frame exists (no diffusion)
    obstacle: Vec<bool>,

    grid_w: u32,
    grid_h: u32,
    rng: Rng,
    time: f32,
    scene_fingerprint: u64,
    screen_w: u32,
    screen_h: u32,
    initialized: bool,
}

impl LivingWall {
    pub fn new() -> Self {
        Self {
            u: Vec::new(),
            v: Vec::new(),
            u_next: Vec::new(),
            v_next: Vec::new(),
            obstacle: Vec::new(),
            grid_w: 0,
            grid_h: 0,
            rng: Rng::new(0xAC1D),
            time: 0.0,
            scene_fingerprint: u64::MAX,
            screen_w: 0,
            screen_h: 0,
            initialized: false,
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

        self.grid_w = width.div_ceil(GRID_SCALE);
        self.grid_h = height.div_ceil(GRID_SCALE);
        let cells = (self.grid_w * self.grid_h) as usize;

        // Initialize U=1 everywhere, V=0 everywhere
        self.u = vec![1.0; cells];
        self.v = vec![0.0; cells];
        self.u_next = vec![0.0; cells];
        self.v_next = vec![0.0; cells];
        self.obstacle = vec![false; cells];

        // Mark obstacles where frame polygons exist
        let gw = self.grid_w;
        for region in &scene.regions {
            if let Some((min_x, min_y, max_x, max_y)) = region.polygon.bounds() {
                let gx0 = ((min_x as i32).max(0) as u32 / GRID_SCALE).min(gw - 1);
                let gx1 = (((max_x as i32) + 1).max(0) as u32 / GRID_SCALE + 1).min(gw);
                let gy0 = ((min_y as i32).max(0) as u32 / GRID_SCALE).min(self.grid_h - 1);
                let gy1 = (((max_y as i32) + 1).max(0) as u32 / GRID_SCALE + 1).min(self.grid_h);

                for gy in gy0..gy1 {
                    for gx in gx0..gx1 {
                        let wx = (gx * GRID_SCALE + GRID_SCALE / 2) as f32;
                        let wy = (gy * GRID_SCALE + GRID_SCALE / 2) as f32;
                        if region.polygon.contains(wx, wy) {
                            self.obstacle[(gy * gw + gx) as usize] = true;
                        }
                    }
                }
            }
        }

        // Seed V near frame edges to start growth from the frames
        for region in &scene.regions {
            if let Some((min_x, min_y, max_x, max_y)) = region.polygon.bounds() {
                let margin = 8i32; // grid cells around frames to seed
                let gx0 = (((min_x as i32 - margin * GRID_SCALE as i32).max(0)) as u32
                    / GRID_SCALE)
                    .min(gw - 1);
                let gx1 = ((((max_x as i32) + margin * GRID_SCALE as i32 + 1).max(0)) as u32
                    / GRID_SCALE
                    + 1)
                .min(gw);
                let gy0 = (((min_y as i32 - margin * GRID_SCALE as i32).max(0)) as u32
                    / GRID_SCALE)
                    .min(self.grid_h - 1);
                let gy1 = ((((max_y as i32) + margin * GRID_SCALE as i32 + 1).max(0)) as u32
                    / GRID_SCALE
                    + 1)
                .min(self.grid_h);

                for gy in gy0..gy1 {
                    for gx in gx0..gx1 {
                        let idx = (gy * gw + gx) as usize;
                        if !self.obstacle[idx] && self.rng.next_f32() < 0.15 {
                            self.v[idx] = self.rng.range_f32(0.15, 0.3);
                            self.u[idx] = 0.5;
                        }
                    }
                }
            }
        }

        self.initialized = true;
    }

    fn step(&mut self) {
        let gw = self.grid_w as i32;
        let gh = self.grid_h as i32;

        for y in 1..gh - 1 {
            for x in 1..gw - 1 {
                let idx = (y * gw + x) as usize;
                if self.obstacle[idx] {
                    self.u_next[idx] = 1.0;
                    self.v_next[idx] = 0.0;
                    continue;
                }

                let u_val = self.u[idx];
                let v_val = self.v[idx];

                // Laplacian with obstacle-aware boundary
                let get = |i: usize| -> (f32, f32) {
                    if self.obstacle[i] {
                        (u_val, v_val) // reflect at obstacle boundary
                    } else {
                        (self.u[i], self.v[i])
                    }
                };

                let (ul, vl) = get((y * gw + x - 1) as usize);
                let (ur, vr) = get((y * gw + x + 1) as usize);
                let (uu, vu) = get(((y - 1) * gw + x) as usize);
                let (ud, vd) = get(((y + 1) * gw + x) as usize);

                let lap_u = (ul + ur + uu + ud) - 4.0 * u_val;
                let lap_v = (vl + vr + vu + vd) - 4.0 * v_val;

                let uvv = u_val * v_val * v_val;
                self.u_next[idx] = u_val + DU * lap_u - uvv + FEED * (1.0 - u_val);
                self.v_next[idx] = v_val + DV * lap_v + uvv - (FEED + KILL) * v_val;

                // Clamp
                self.u_next[idx] = self.u_next[idx].clamp(0.0, 1.0);
                self.v_next[idx] = self.v_next[idx].clamp(0.0, 1.0);
            }
        }

        // Swap buffers
        std::mem::swap(&mut self.u, &mut self.u_next);
        std::mem::swap(&mut self.v, &mut self.v_next);
    }
}

impl Default for LivingWall {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for LivingWall {
    fn update(&mut self, dt: f32, width: u32, height: u32, scene: &Scene) {
        let fp = Self::scene_fingerprint(scene);
        if width != self.screen_w || height != self.screen_h || fp != self.scene_fingerprint {
            self.rebuild_scene(width, height, scene, fp);
        }
        self.time += dt;

        if !self.initialized {
            return;
        }

        for _ in 0..STEPS_PER_FRAME {
            self.step();
        }
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        let w = buffer.width() as i32;
        let h = buffer.height() as i32;
        let gw = self.grid_w;

        buffer.clear(5, 5, 8);

        if !self.initialized {
            return;
        }

        let buf_w = buffer.width();
        let pixels = buffer.as_bytes_mut();

        for py in 0..h {
            let gy = (py as u32 / GRID_SCALE).min(self.grid_h - 1);
            for px in 0..w {
                let gx = (px as u32 / GRID_SCALE).min(gw - 1);
                let gi = (gy * gw + gx) as usize;

                if self.obstacle[gi] {
                    continue; // region masking handles this
                }

                let v_val = self.v[gi];
                let u_val = self.u[gi];

                // Color based on chemical concentrations
                // V high = organic growth (teal/cyan), U depleted = darker
                let pi = ((py as u32 * buf_w + px as u32) * 4) as usize;

                if v_val > 0.01 {
                    // Growth pattern: teal → purple gradient based on V concentration
                    let t = v_val.min(0.5) * 2.0; // normalize to 0..1

                    // Organic color: dark teal base with purple highlights
                    let r = (5.0 + t * 120.0 + (1.0 - u_val) * 40.0) as u8;
                    let g = (5.0 + t * 180.0 * u_val) as u8;
                    let b = (8.0 + t * 140.0 + (1.0 - u_val) * 60.0) as u8;

                    pixels[pi] = 255;
                    pixels[pi + 1] = b;
                    pixels[pi + 2] = g;
                    pixels[pi + 3] = r;
                }
            }
        }

        buffer.bloom(25, 2, 0.3);
    }

    fn name(&self) -> &str {
        "Living Wall"
    }

    fn region_color(&self) -> (u8, u8, u8) {
        (5, 5, 8)
    }
}
