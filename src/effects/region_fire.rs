//! Region Fire Effect
//!
//! Fire that burns inside the defined regions instead of filling the background.
//! Each region becomes a window into a shared fire simulation.

use super::color::fire_palette;
use super::Effect;
use crate::display::PixelBuffer;
use crate::regions::{Scene, Shape};
use crate::util::Rng;

/// Target fire pixel size (keeps the chunky retro look)
const FIRE_SCALE: u32 = 4;
/// Fixed simulation rate
const SIM_STEP: f32 = 1.0 / 60.0;

/// Fire effect that renders inside regions
pub struct RegionFire {
    buffer: Vec<u8>,
    palette: Vec<(u8, u8, u8)>,
    time: f32,
    sim_accum: f32,
    rng: Rng,
    fire_w: usize,
    fire_h: usize,
    scale: u32,
    // Cache the scene for rendering
    cached_scene: Option<Vec<CachedRegion>>,
    scene_hash: u64,
}

#[derive(Clone)]
struct CachedRegion {
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
    is_circle: bool,
    // For circles
    cx: f32,
    cy: f32,
    radius: f32,
    // For polygons - precomputed for fast point-in-polygon
    vertices: Vec<(f32, f32)>,
}

impl RegionFire {
    pub fn new() -> Self {
        let fire_w = 160;
        let fire_h = 120;
        Self {
            buffer: vec![0; fire_w * fire_h],
            palette: fire_palette(),
            time: 0.0,
            sim_accum: 0.0,
            rng: Rng::new(0xF1E3_ABCD),
            fire_w,
            fire_h,
            scale: FIRE_SCALE,
            cached_scene: None,
            scene_hash: 0,
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        let new_w = (width / self.scale) as usize;
        let new_h = (height / self.scale) as usize;

        if new_w != self.fire_w || new_h != self.fire_h {
            self.fire_w = new_w.max(1);
            self.fire_h = new_h.max(1);
            self.buffer = vec![0; self.fire_w * self.fire_h];
        }
    }

    fn hash_scene(scene: &Scene) -> u64 {
        let mut h: u64 = scene.regions.len() as u64;
        for region in &scene.regions {
            if let Some(bounds) = region.get_shape().bounds() {
                h = h.wrapping_mul(31).wrapping_add(bounds.0.to_bits() as u64);
                h = h.wrapping_mul(31).wrapping_add(bounds.1.to_bits() as u64);
                h = h.wrapping_mul(31).wrapping_add(bounds.2.to_bits() as u64);
                h = h.wrapping_mul(31).wrapping_add(bounds.3.to_bits() as u64);
            }
        }
        h
    }

    fn cache_scene(&mut self, scene: &Scene) {
        let hash = Self::hash_scene(scene);
        if self.scene_hash == hash && self.cached_scene.is_some() {
            return;
        }

        let mut cached = Vec::with_capacity(scene.regions.len());
        for region in &scene.regions {
            let shape = region.get_shape();
            if let Some((min_x, min_y, max_x, max_y)) = shape.bounds() {
                match shape {
                    Shape::Circle(c) => {
                        cached.push(CachedRegion {
                            min_x: min_x as i32,
                            min_y: min_y as i32,
                            max_x: max_x as i32,
                            max_y: max_y as i32,
                            is_circle: true,
                            cx: c.center.x,
                            cy: c.center.y,
                            radius: c.radius,
                            vertices: Vec::new(),
                        });
                    }
                    Shape::Polygon(p) => {
                        cached.push(CachedRegion {
                            min_x: min_x as i32,
                            min_y: min_y as i32,
                            max_x: max_x as i32,
                            max_y: max_y as i32,
                            is_circle: false,
                            cx: 0.0,
                            cy: 0.0,
                            radius: 0.0,
                            vertices: p.as_tuples(),
                        });
                    }
                }
            }
        }

        self.cached_scene = Some(cached);
        self.scene_hash = hash;
    }

    /// Check if a point is inside a cached region
    fn point_in_region(region: &CachedRegion, x: f32, y: f32) -> bool {
        if region.is_circle {
            let dx = x - region.cx;
            let dy = y - region.cy;
            dx * dx + dy * dy <= region.radius * region.radius
        } else {
            // Ray casting algorithm
            let verts = &region.vertices;
            let n = verts.len();
            if n < 3 {
                return false;
            }

            let mut inside = false;
            let mut j = n - 1;
            for i in 0..n {
                let (xi, yi) = verts[i];
                let (xj, yj) = verts[j];
                if ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / (yj - yi) + xi) {
                    inside = !inside;
                }
                j = i;
            }
            inside
        }
    }
}

impl Default for RegionFire {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for RegionFire {
    fn update(&mut self, dt: f32, width: u32, height: u32, scene: &Scene) {
        self.resize(width, height);
        self.cache_scene(scene);

        self.time += dt;
        self.sim_accum += dt;

        let fire_w = self.fire_w;
        let fire_h = self.fire_h;

        let max_steps = 4;
        let mut steps = 0;

        while self.sim_accum >= SIM_STEP && steps < max_steps {
            self.sim_accum -= SIM_STEP;
            steps += 1;

            // Seed the bottom row with heat
            for x in 0..fire_w {
                let flicker = self.rng.next_u8() % 80;
                let wave = ((self.time * 6.0 + x as f32 * 0.08).sin() * 40.0) as i32;
                let base = (180 + wave).clamp(120, 255) as u8;
                self.buffer[(fire_h - 1) * fire_w + x] = base.saturating_add(flicker);
            }

            // Propagate heat upwards
            let wind = (self.time * 2.5).sin() * 2.0;
            for y in 1..fire_h {
                for x in 0..fire_w {
                    let wind_x = (x as i32 + wind as i32 + fire_w as i32) % fire_w as i32;
                    let x0 = wind_x as usize;
                    let x1 = (x0 + fire_w - 1) % fire_w;
                    let x2 = (x0 + 1) % fire_w;

                    let below = self.buffer[y * fire_w + x0] as u16;
                    let below_left = self.buffer[y * fire_w + x1] as u16;
                    let below_right = self.buffer[y * fire_w + x2] as u16;
                    let below2 = if y + 1 < fire_h {
                        self.buffer[(y + 1) * fire_w + x0] as u16
                    } else {
                        below
                    };

                    let mut heat = (below + below_left + below_right + below2) / 4;
                    let cooling = 1 + (self.rng.next_u8() % 3) as u16;
                    heat = heat.saturating_sub(cooling);
                    self.buffer[(y - 1) * fire_w + x] = heat as u8;
                }
            }
        }

        if steps >= max_steps {
            self.sim_accum = 0.0;
        }
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        // Dark background
        buffer.clear(5, 2, 0);

        let Some(regions) = &self.cached_scene else {
            return;
        };

        if regions.is_empty() {
            return;
        }

        let fire_w = self.fire_w;
        let fire_h = self.fire_h;
        let scale = self.scale as i32;

        // For each region, render fire pixels that fall within it
        for region in regions {
            // Iterate over the region's bounding box
            for py in region.min_y..=region.max_y {
                for px in region.min_x..=region.max_x {
                    // Check if this pixel is inside the region
                    if !Self::point_in_region(region, px as f32 + 0.5, py as f32 + 0.5) {
                        continue;
                    }

                    // Map screen pixel to fire buffer coordinate
                    let fx = (px / scale) as usize;
                    let fy = (py / scale) as usize;

                    if fx < fire_w && fy < fire_h {
                        let heat = self.buffer[fy * fire_w + fx] as usize;
                        let (r, g, b) = self.palette[heat.min(255)];
                        buffer.set_pixel(px, py, r, g, b);
                    }
                }
            }
        }
    }

    fn name(&self) -> &str {
        "Region Fire"
    }

    fn region_color(&self) -> (u8, u8, u8) {
        // Regions ARE the fire, background is dark
        (5, 2, 0)
    }
}
