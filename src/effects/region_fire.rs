//! Region Fire Effect
//!
//! Flames erupt upward from the top edges of defined regions,
//! as if each region is on fire. The fire simulation runs per-column
//! with heat sources at the region surfaces.

use super::color::fire_palette;
use super::Effect;
use crate::display::PixelBuffer;
use crate::regions::Scene;
use crate::util::Rng;

/// Fire pixel scale for chunky retro look
const FIRE_SCALE: u32 = 2;
/// How high flames can rise above regions (in fire-grid cells)
const FLAME_HEIGHT: usize = 150;
/// Fixed simulation rate
const SIM_STEP: f32 = 1.0 / 60.0;

/// Fire effect with flames erupting from region surfaces
pub struct RegionFire {
    /// Per-column heat values (screen width / scale)
    heat: Vec<Vec<u8>>,
    /// Per-column: Y of region surface (-1 if no region)
    surface_y: Vec<i32>,
    palette: Vec<(u8, u8, u8)>,
    time: f32,
    sim_accum: f32,
    rng: Rng,
    fire_w: usize,
    screen_w: u32,
    screen_h: u32,
    scene_hash: u64,
}

impl RegionFire {
    pub fn new() -> Self {
        Self {
            heat: Vec::new(),
            surface_y: Vec::new(),
            palette: fire_palette(),
            time: 0.0,
            sim_accum: 0.0,
            rng: Rng::new(0xF1E3_ABCD),
            fire_w: 0,
            screen_w: 0,
            screen_h: 0,
            scene_hash: 0,
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

    /// Rebuild the surface map - find topmost Y of each region per column
    fn rebuild_surface(&mut self, width: u32, height: u32, scene: &Scene) {
        self.screen_w = width;
        self.screen_h = height;
        self.fire_w = (width / FIRE_SCALE) as usize;

        // Initialize heat columns
        self.heat = vec![vec![0u8; FLAME_HEIGHT]; self.fire_w];

        // Initialize surface map (-1 = no surface)
        self.surface_y = vec![-1; self.fire_w];

        let h = height as i32;

        // Scan each column to find topmost region surface
        for region in &scene.regions {
            let shape = region.get_shape();
            if let Some((min_x, min_y, max_x, max_y)) = shape.bounds() {
                let x0 = ((min_x as i32).max(0) / FIRE_SCALE as i32) as usize;
                let x1 = (((max_x as i32) + 1).min(width as i32) / FIRE_SCALE as i32) as usize;
                let y_start = (min_y as i32).max(0);
                let y_end = ((max_y as i32) + 1).min(h);

                for fire_col in x0..x1.min(self.fire_w) {
                    let screen_x = (fire_col as i32 * FIRE_SCALE as i32) + (FIRE_SCALE as i32 / 2);

                    // Scan downward to find topmost point inside region
                    for y in y_start..y_end {
                        if shape.contains(screen_x as f32, y as f32 + 0.5) {
                            // Found the top surface at this column
                            if self.surface_y[fire_col] < 0 || y < self.surface_y[fire_col] {
                                self.surface_y[fire_col] = y;
                            }
                            break;
                        }
                    }
                }
            }
        }

        self.scene_hash = Self::hash_scene(scene);
    }
}

impl Default for RegionFire {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for RegionFire {
    fn update(&mut self, dt: f32, width: u32, height: u32, scene: &Scene) {
        let hash = Self::hash_scene(scene);
        if width != self.screen_w || height != self.screen_h || hash != self.scene_hash {
            self.rebuild_surface(width, height, scene);
        }

        self.time += dt;
        self.sim_accum += dt;

        let max_steps = 4;
        let mut steps = 0;

        while self.sim_accum >= SIM_STEP && steps < max_steps {
            self.sim_accum -= SIM_STEP;
            steps += 1;

            let fire_w = self.fire_w;

            // Process each column - classic fire algorithm
            for x in 0..fire_w {
                // Inject heat at surface (bottom of flame column)
                if self.surface_y[x] >= 0 {
                    // Strong, flickering heat source
                    let flicker = self.rng.next_u8() % 55;
                    let wave = ((self.time * 6.0 + x as f32 * 0.08).sin() * 25.0) as i32;
                    let base = (230 + wave).clamp(200, 255) as u8;
                    self.heat[x][FLAME_HEIGHT - 1] = base.saturating_add(flicker);
                    // Add extra heat in rows above for thicker base
                    if FLAME_HEIGHT > 2 {
                        self.heat[x][FLAME_HEIGHT - 2] = base.saturating_add(flicker / 2);
                    }
                    if FLAME_HEIGHT > 3 {
                        self.heat[x][FLAME_HEIGHT - 3] = (base as u16 * 9 / 10) as u8;
                    }
                    if FLAME_HEIGHT > 4 {
                        self.heat[x][FLAME_HEIGHT - 4] = (base as u16 * 8 / 10) as u8;
                    }
                } else {
                    // Cool down columns without surfaces
                    self.heat[x][FLAME_HEIGHT - 1] =
                        self.heat[x][FLAME_HEIGHT - 1].saturating_sub(10);
                }

                // Propagate heat upward (from bottom to top)
                for fy in (0..FLAME_HEIGHT - 4).rev() {
                    // Get heat from row below
                    let below = self.heat[x][fy + 1] as u16;

                    // Get neighbors from adjacent columns (for spread)
                    let left_x = if x > 0 { x - 1 } else { 0 };
                    let right_x = if x + 1 < fire_w { x + 1 } else { fire_w - 1 };

                    let below_left = self.heat[left_x][fy + 1] as u16;
                    let below_right = self.heat[right_x][fy + 1] as u16;

                    // Weighted average - favor center for tall vertical flames
                    let avg = (below * 4 + below_left + below_right) / 6;

                    // Gentle cooling - flames rise high before fading
                    let height_from_bottom = (FLAME_HEIGHT - 1 - fy) as u16;
                    let base_cooling = 1 + height_from_bottom / 15;
                    let random_cooling = (self.rng.next_u8() % 3) as u16;

                    let heat = avg.saturating_sub(base_cooling + random_cooling);
                    self.heat[x][fy] = heat as u8;
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

        if self.heat.is_empty() {
            return;
        }

        let scale = FIRE_SCALE as i32;

        // Render flames ABOVE each region surface (so they're not masked)
        for (fire_x, col) in self.heat.iter().enumerate() {
            let surface = self.surface_y[fire_x];
            if surface < 0 {
                continue; // No region at this column
            }

            let screen_x = fire_x as i32 * scale;

            // Draw flame column upward from surface
            // fire_y=FLAME_HEIGHT-1 is heat source (at surface), fire_y=0 is top of flame
            for (fire_y, &heat) in col.iter().enumerate() {
                if heat < 8 {
                    continue; // Skip very dim pixels
                }

                // y_offset: how far above the surface this fire cell is
                // fire_y=FLAME_HEIGHT-1 -> y_offset=0 (at surface)
                // fire_y=0 -> y_offset=FLAME_HEIGHT-1 (far above)
                let y_offset = (FLAME_HEIGHT - 1 - fire_y) as i32;

                // Render flames starting ABOVE the surface (surface - 1 and up)
                // Skip the first few fire rows so flames don't overlap region
                if y_offset < 1 {
                    continue; // Don't render at or below surface
                }

                let screen_y = surface - y_offset;

                if screen_y < 0 {
                    continue;
                }

                let (r, g, b) = self.palette[heat as usize];

                // Draw a block of pixels for this fire cell
                for dy in 0..scale {
                    for dx in 0..scale {
                        let px = screen_x + dx;
                        let py = screen_y + dy;
                        if py >= 0 && py < surface {
                            buffer.set_pixel(px, py, r, g, b);
                        }
                    }
                }
            }
        }
    }

    fn name(&self) -> &str {
        "Region Fire"
    }

    fn region_color(&self) -> (u8, u8, u8) {
        // Regions should be masked black (fire rises above them)
        (0, 0, 0)
    }
}
