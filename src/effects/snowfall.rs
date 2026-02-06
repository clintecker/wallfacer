use super::Effect;
use crate::display::PixelBuffer;
use crate::regions::Scene;
use crate::util::Rng;
use std::f32::consts::TAU;

const MAX_FLAKES: usize = 3000;
const TARGET_FLAKES: usize = 2000;
const SPAWN_RATE: f32 = 300.0;
const GROUND_SNOW_CAP: i32 = 60;
const REGION_SNOW_CAP: i32 = 30;
/// Max height difference between adjacent columns before snow slides (1 = 45° angle of repose)
const SLIDE_THRESHOLD: i32 = 1;
const WIND_AMPLITUDE: f32 = 40.0;
const WIND_PERIOD: f32 = 8.0;

struct Flake {
    x: f32,
    y: f32,
    vy: f32,
    size: u8,
    brightness: u8,
}

pub struct Snowfall {
    flakes: Vec<Flake>,
    active: usize,

    /// Per-column: topmost Y of the first region surface (screen_h if none)
    surface_top: Vec<i32>,
    /// Per-column: bottommost Y of that same region (screen_h if none)
    surface_bot: Vec<i32>,
    ground_snow: Vec<i32>,
    region_snow: Vec<i32>,
    /// Per-column: max snow allowed (0 on steep/edge surfaces)
    snow_cap: Vec<i32>,

    rng: Rng,
    time: f32,
    spawn_accum: f32,
    screen_w: u32,
    screen_h: u32,
    scene_fingerprint: u64,
}

impl Snowfall {
    pub fn new() -> Self {
        let mut flakes = Vec::with_capacity(MAX_FLAKES);
        for _ in 0..MAX_FLAKES {
            flakes.push(Flake {
                x: 0.0,
                y: 0.0,
                vy: 60.0,
                size: 1,
                brightness: 220,
            });
        }
        Self {
            flakes,
            active: 0,
            surface_top: Vec::new(),
            surface_bot: Vec::new(),
            ground_snow: Vec::new(),
            region_snow: Vec::new(),
            snow_cap: Vec::new(),
            rng: Rng::new(0x5A0F),
            time: 0.0,
            spawn_accum: 0.0,
            screen_w: 0,
            screen_h: 0,
            scene_fingerprint: u64::MAX,
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

    fn rebuild_surface(&mut self, width: u32, height: u32, scene: &Scene, fingerprint: u64) {
        let w = width as usize;
        let h = height as i32;

        self.surface_top = vec![h; w];
        self.surface_bot = vec![h; w];
        self.ground_snow = vec![0; w];
        self.region_snow = vec![0; w];

        // For each region, scan columns to find top and bottom of region surface
        // Skip top chyron (would catch all snow), but allow bottom chyron as a surface
        for region in &scene.regions {
            if region.name == "chyron_top" {
                continue;
            }
            let shape = region.get_shape();
            if let Some((min_x, min_y, max_x, max_y)) = shape.bounds() {
                let x0 = (min_x as i32).max(0) as usize;
                let x1 = ((max_x as i32) + 1).min(width as i32) as usize;
                let y_start = (min_y as i32).max(0);
                let y_end = ((max_y as i32) + 1).min(h);

                for col in x0..x1 {
                    let mut this_top = h;
                    let mut this_bot = 0i32;
                    let mut found = false;

                    for row in y_start..y_end {
                        if shape.contains(col as f32 + 0.5, row as f32 + 0.5) {
                            if !found {
                                this_top = row;
                                found = true;
                            }
                            this_bot = row;
                        }
                    }

                    // Use the topmost region at each column
                    if found && this_top < self.surface_top[col] {
                        self.surface_top[col] = this_top;
                        self.surface_bot[col] = this_bot;
                    }
                }
            }
        }

        // Build per-column snow cap based on surface slope.
        // slope > 1 pixel/column (>45°) → no accumulation
        self.snow_cap = vec![0; w];
        for x in 0..w {
            if self.surface_top[x] >= h {
                continue; // no region here
            }

            let center = self.surface_top[x];
            let left = if x > 0 { self.surface_top[x - 1] } else { h };
            let right = if x + 1 < w {
                self.surface_top[x + 1]
            } else {
                h
            };

            let left_ok = left < h && (left - center).abs() <= 1;
            let right_ok = right < h && (right - center).abs() <= 1;

            if left_ok && right_ok {
                // Interior flat surface (≤45°) — full cap
                self.snow_cap[x] = REGION_SNOW_CAP;
            } else if left_ok || right_ok {
                // Edge column — small taper
                self.snow_cap[x] = REGION_SNOW_CAP / 4;
            }
            // else: steep or cliff edge — cap stays 0
        }

        self.screen_w = width;
        self.screen_h = height;
        self.scene_fingerprint = fingerprint;
        self.active = 0;
    }

    fn spawn_flake(&mut self, width: u32) {
        if self.active >= MAX_FLAKES {
            return;
        }
        let f = &mut self.flakes[self.active];
        f.x = self.rng.range_f32(0.0, width as f32);
        f.y = self.rng.range_f32(-20.0, 0.0);
        // Larger flakes fall slower (more realistic)
        let size_roll = self.rng.next_f32();
        f.size = if size_roll < 0.15 {
            4 // 15% large fluffy flakes
        } else if size_roll < 0.40 {
            3 // 25% medium flakes
        } else if size_roll < 0.70 {
            2 // 30% small flakes
        } else {
            1 // 30% tiny flakes
        };
        f.vy = self.rng.range_f32(30.0, 80.0) + (5 - f.size) as f32 * 15.0;
        f.brightness = 200 + (self.rng.next_u32() % 56) as u8;
        self.active += 1;
    }
}

impl Default for Snowfall {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Snowfall {
    fn update(&mut self, dt: f32, width: u32, height: u32, scene: &Scene) {
        let fp = Self::scene_fingerprint(scene);
        if width != self.screen_w || height != self.screen_h || fp != self.scene_fingerprint {
            self.rebuild_surface(width, height, scene, fp);
        }

        self.time += dt;
        let w = width as f32;
        let h_i = height as i32;
        let wind = (self.time * TAU / WIND_PERIOD).sin() * WIND_AMPLITUDE;

        // Pass 1: Move + Land
        let mut i = 0;
        while i < self.active {
            let f = &mut self.flakes[i];
            f.x += wind * dt;
            f.y += f.vy * dt;

            // Wrap X
            if f.x < 0.0 {
                f.x += w;
            } else if f.x >= w {
                f.x -= w;
            }

            let col = f.x as usize;
            let fy = f.y as i32;
            let mut landed = false;
            let w_usize = width as usize;

            if col < self.surface_top.len() {
                // Accumulation and spread based on flake size
                // Bigger flakes create wider, taller snow deposits
                let (center_acc, spread) = match f.size {
                    1 => (1, 0),      // tiny: 1 height, no spread
                    2 => (2, 1),      // small: 2 height, 1 col each side
                    3 => (2, 1),      // medium: 2 height, 1 col each side
                    _ => (3, 2),      // fluffy: 3 height, 2 cols each side
                };

                // Region surface check:
                // Only land if (a) column allows accumulation, (b) flake is at/above
                // the snow surface, and (c) flake is NOT below the region bottom
                // (i.e. blown underneath — let it keep falling to the ground)
                let cap = self.snow_cap[col];
                let has_region = self.surface_top[col] < h_i;
                if cap > 0 && has_region {
                    let snow_surface = self.surface_top[col] - self.region_snow[col];
                    let region_bottom = self.surface_bot[col];
                    if fy >= snow_surface && fy <= region_bottom {
                        // Add to center column
                        if self.region_snow[col] < cap {
                            self.region_snow[col] = (self.region_snow[col] + center_acc).min(cap);
                        }
                        // Spread to adjacent columns (smaller amount)
                        for offset in 1..=spread {
                            let side_acc = center_acc / (offset as i32 + 1); // diminishing
                            if col >= offset {
                                let left = col - offset;
                                if self.surface_top[left] < h_i && self.snow_cap[left] > 0 {
                                    self.region_snow[left] = (self.region_snow[left] + side_acc).min(self.snow_cap[left]);
                                }
                            }
                            if col + offset < w_usize {
                                let right = col + offset;
                                if self.surface_top[right] < h_i && self.snow_cap[right] > 0 {
                                    self.region_snow[right] = (self.region_snow[right] + side_acc).min(self.snow_cap[right]);
                                }
                            }
                        }
                        landed = true;
                    }
                }

                // Ground check
                if !landed {
                    let ground_top = (h_i - 1) - self.ground_snow[col];
                    if fy >= ground_top {
                        // Add to center column
                        if self.ground_snow[col] < GROUND_SNOW_CAP {
                            self.ground_snow[col] = (self.ground_snow[col] + center_acc).min(GROUND_SNOW_CAP);
                        }
                        // Spread to adjacent columns
                        for offset in 1..=spread {
                            let side_acc = center_acc / (offset as i32 + 1);
                            if col >= offset {
                                let left = col - offset;
                                self.ground_snow[left] = (self.ground_snow[left] + side_acc).min(GROUND_SNOW_CAP);
                            }
                            if col + offset < w_usize {
                                let right = col + offset;
                                self.ground_snow[right] = (self.ground_snow[right] + side_acc).min(GROUND_SNOW_CAP);
                            }
                        }
                        landed = true;
                    }
                }
            }

            if landed {
                self.active -= 1;
                self.flakes.swap(i, self.active);
            } else {
                i += 1;
            }
        }

        // Pass 2: Slide/settle (5 passes for faster redistribution with tight threshold)
        let w_usize = width as usize;
        if w_usize > 1 {
            for _ in 0..5 {
                // Ground slide — enforces 45° max angle of repose
                for x in 0..w_usize - 1 {
                    let diff = self.ground_snow[x] - self.ground_snow[x + 1];
                    if diff > SLIDE_THRESHOLD {
                        self.ground_snow[x] -= 1;
                        self.ground_snow[x + 1] += 1;
                    } else if diff < -SLIDE_THRESHOLD {
                        self.ground_snow[x] += 1;
                        self.ground_snow[x + 1] -= 1;
                    }
                }

                // Region slide
                for x in 0..w_usize - 1 {
                    let left_has_surface = self.surface_top[x] < h_i;
                    let right_has_surface = self.surface_top[x + 1] < h_i;

                    if left_has_surface && right_has_surface {
                        let diff = self.region_snow[x] - self.region_snow[x + 1];
                        if diff > SLIDE_THRESHOLD {
                            self.region_snow[x] -= 1;
                            self.region_snow[x + 1] += 1;
                        } else if diff < -SLIDE_THRESHOLD {
                            self.region_snow[x] += 1;
                            self.region_snow[x + 1] -= 1;
                        }
                        // Enforce per-column caps
                        for &col in &[x, x + 1] {
                            if self.region_snow[col] > self.snow_cap[col] {
                                self.region_snow[col] = self.snow_cap[col];
                            }
                        }
                    } else {
                        // Edge cascade: excess snow falls off as new flakes
                        let edge_col = if left_has_surface && !right_has_surface {
                            x
                        } else if !left_has_surface && right_has_surface {
                            x + 1
                        } else {
                            continue;
                        };

                        if self.region_snow[edge_col] > self.snow_cap[edge_col] {
                            self.region_snow[edge_col] -= 1;
                            if self.active < MAX_FLAKES {
                                let spawn_y =
                                    self.surface_top[edge_col] - self.region_snow[edge_col];
                                let f = &mut self.flakes[self.active];
                                f.x = edge_col as f32;
                                f.y = spawn_y as f32;
                                f.vy = self.rng.range_f32(40.0, 80.0);
                                f.size = 1;
                                f.brightness = 230;
                                self.active += 1;
                            }
                        }
                    }
                }
            }
        }

        // Pass 3: Emit
        if self.active < TARGET_FLAKES {
            self.spawn_accum += SPAWN_RATE * dt;
            while self.spawn_accum >= 1.0 && self.active < MAX_FLAKES {
                self.spawn_flake(width);
                self.spawn_accum -= 1.0;
            }
        }
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        let w = buffer.width() as i32;
        let h = buffer.height() as i32;

        // Background: dark gradient sky
        for row in 0..h {
            let t = row as f32 / h as f32;
            let r = (5.0 + t * 15.0) as u8;
            let g = (8.0 + t * 12.0) as u8;
            let b = (20.0 + t * 10.0) as u8;
            buffer.hline(0, w - 1, row, r, g, b);
        }

        // Ground snow
        for x in 0..w as usize {
            let snow_h = self.ground_snow.get(x).copied().unwrap_or(0);
            if snow_h > 0 {
                let tint = (230 + (x as u32 * 7 % 26)) as u8;
                let roughness = ((x.wrapping_mul(17).wrapping_add(31)) % 3) as i32 - 1;
                let y_top = (h - 1 - snow_h + roughness).max(0);
                buffer.vline(x as i32, y_top, h - 1, tint, tint, tint);
            }
        }

        // Region snow
        for x in 0..w as usize {
            let surf = self.surface_top.get(x).copied().unwrap_or(h);
            let snow_h = self.region_snow.get(x).copied().unwrap_or(0);
            if snow_h > 0 && surf < h {
                let tint = (230 + (x as u32 * 13 % 26)) as u8;
                let roughness = ((x.wrapping_mul(23).wrapping_add(7)) % 3) as i32 - 1;
                let y_top = (surf - snow_h + roughness).max(0);
                buffer.vline(x as i32, y_top, surf - 1, tint, tint, tint);
            }
        }

        // Falling flakes - varied sizes for depth
        for i in 0..self.active {
            let f = &self.flakes[i];
            let px = f.x as i32;
            let py = f.y as i32;
            let b = f.brightness;
            match f.size {
                1 => {
                    buffer.set_pixel(px, py, b, b, b);
                },
                2 => {
                    buffer.set_pixel(px, py, b, b, b);
                    buffer.set_pixel(px + 1, py, b, b, b);
                    buffer.set_pixel(px, py + 1, b, b, b);
                    buffer.set_pixel(px + 1, py + 1, b, b, b);
                },
                3 => {
                    // 3x3 cross pattern
                    buffer.set_pixel(px, py, b, b, b);
                    buffer.set_pixel(px - 1, py, b, b, b);
                    buffer.set_pixel(px + 1, py, b, b, b);
                    buffer.set_pixel(px, py - 1, b, b, b);
                    buffer.set_pixel(px, py + 1, b, b, b);
                },
                _ => {
                    // 5-pixel diamond + corners for fluffy look
                    buffer.fill_circle(px, py, 2, b, b, b);
                },
            }
        }
    }

    fn name(&self) -> &str {
        "Snowfall"
    }
}
