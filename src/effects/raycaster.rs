//! Raycaster Maze Effect
//!
//! Wolfenstein-style first-person auto-walk through a procedurally generated maze.
//! DDA grid-stepping raycaster, procedural brick texture, distance fog,
//! and look-ahead auto-navigation using raycasts for pathfinding.
//! Includes debug minimap overlay showing navigation rays.

use super::Effect;
use crate::display::PixelBuffer;
use crate::regions::Scene;
use crate::texture::Texture;
use crate::util::Rng;

const MAP_SIZE: usize = 16;
const TEX_SIZE: u32 = 64;
const MINIMAP_CELL: i32 = 6;
const MINIMAP_MARGIN: i32 = 4;

/// Debug ray for minimap visualization
struct DebugRay {
    start_x: f32,
    start_y: f32,
    angle: f32,
    dist: f32,
    r: u8,
    g: u8,
    b: u8,
}

/// Wolfenstein-style raycaster with procedural maze
pub struct Raycaster {
    time: f32,
    map: Vec<u8>,
    player_x: f32,
    player_y: f32,
    player_angle: f32,
    target_angle: f32,
    turning: bool,
    move_speed: f32,
    wall_texture: Texture,
    rng: Rng,
    debug_rays: Vec<DebugRay>,
}

impl Raycaster {
    pub fn new() -> Self {
        let mut rng = Rng::new(1337);
        let map = generate_maze(&mut rng);
        let wall_texture = build_brick_texture();

        let (sx, sy) = find_open_cell(&map);
        let start_angle = find_open_direction(&map, sx, sy);

        Self {
            time: 0.0,
            map,
            player_x: sx as f32 + 0.5,
            player_y: sy as f32 + 0.5,
            player_angle: start_angle,
            target_angle: start_angle,
            turning: false,
            move_speed: 1.5,
            wall_texture,
            rng,
            debug_rays: Vec::new(),
        }
    }

    fn is_wall(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x >= MAP_SIZE as i32 || y >= MAP_SIZE as i32 {
            return true;
        }
        self.map[y as usize * MAP_SIZE + x as usize] == 1
    }

    /// Cast a ray from an arbitrary position and return the distance to the first wall
    fn cast_ray_from(&self, start_x: f32, start_y: f32, angle: f32) -> f32 {
        let ray_cos = angle.cos();
        let ray_sin = angle.sin();

        let mut map_x = start_x as i32;
        let mut map_y = start_y as i32;

        let delta_dist_x = if ray_cos == 0.0 {
            f32::MAX
        } else {
            (1.0 / ray_cos).abs()
        };
        let delta_dist_y = if ray_sin == 0.0 {
            f32::MAX
        } else {
            (1.0 / ray_sin).abs()
        };

        let (step_x, mut side_dist_x) = if ray_cos < 0.0 {
            (-1, (start_x - map_x as f32) * delta_dist_x)
        } else {
            (1, (map_x as f32 + 1.0 - start_x) * delta_dist_x)
        };
        let (step_y, mut side_dist_y) = if ray_sin < 0.0 {
            (-1, (start_y - map_y as f32) * delta_dist_y)
        } else {
            (1, (map_y as f32 + 1.0 - start_y) * delta_dist_y)
        };

        let mut hit_side;
        for _ in 0..64 {
            if side_dist_x < side_dist_y {
                side_dist_x += delta_dist_x;
                map_x += step_x;
                hit_side = 0;
            } else {
                side_dist_y += delta_dist_y;
                map_y += step_y;
                hit_side = 1;
            }

            if self.is_wall(map_x, map_y) {
                return if hit_side == 0 {
                    (map_x as f32 - start_x + (1 - step_x) as f32 * 0.5) / ray_cos
                } else {
                    (map_y as f32 - start_y + (1 - step_y) as f32 * 0.5) / ray_sin
                };
            }
        }
        f32::MAX
    }

    /// Cast a ray from the player's current position
    fn cast_ray(&self, angle: f32) -> f32 {
        self.cast_ray_from(self.player_x, self.player_y, angle)
    }

    /// Draw debug minimap overlay showing maze, player, and navigation rays
    fn draw_minimap(&self, buffer: &mut PixelBuffer) {
        let ox = MINIMAP_MARGIN;
        let oy = MINIMAP_MARGIN;

        // Draw maze grid
        for my in 0..MAP_SIZE as i32 {
            for mx in 0..MAP_SIZE as i32 {
                let is_wall = self.map[my as usize * MAP_SIZE + mx as usize] == 1;
                let (r, g, b) = if is_wall { (50, 50, 70) } else { (12, 12, 20) };
                let px = ox + mx * MINIMAP_CELL;
                let py = oy + my * MINIMAP_CELL;
                buffer.fill_rect(px, py, MINIMAP_CELL as u32, MINIMAP_CELL as u32, r, g, b);
            }
        }

        // Draw debug rays
        for ray in &self.debug_rays {
            let sx = ox + (ray.start_x * MINIMAP_CELL as f32) as i32;
            let sy = oy + (ray.start_y * MINIMAP_CELL as f32) as i32;
            let clamped_dist = ray.dist.min(10.0);
            let ex =
                ox + ((ray.start_x + ray.angle.cos() * clamped_dist) * MINIMAP_CELL as f32) as i32;
            let ey =
                oy + ((ray.start_y + ray.angle.sin() * clamped_dist) * MINIMAP_CELL as f32) as i32;
            buffer.line(sx, sy, ex, ey, ray.r, ray.g, ray.b);
        }

        // Draw player dot
        let px = ox + (self.player_x * MINIMAP_CELL as f32) as i32;
        let py = oy + (self.player_y * MINIMAP_CELL as f32) as i32;
        for dy in -1..=1_i32 {
            for dx in -1..=1_i32 {
                buffer.set_pixel(px + dx, py + dy, 0, 255, 0);
            }
        }

        // Draw player direction arrow
        let dir_len = 2.5;
        let dx =
            ox + ((self.player_x + self.player_angle.cos() * dir_len) * MINIMAP_CELL as f32) as i32;
        let dy =
            oy + ((self.player_y + self.player_angle.sin() * dir_len) * MINIMAP_CELL as f32) as i32;
        buffer.line(px, py, dx, dy, 255, 255, 255);
    }
}

impl Default for Raycaster {
    fn default() -> Self {
        Self::new()
    }
}

/// Find an open cell that's in a corridor (has open neighbors)
fn find_open_cell(map: &[u8]) -> (usize, usize) {
    let mut best = (1, 1);
    let mut best_score = 0;

    for y in 1..MAP_SIZE - 1 {
        for x in 1..MAP_SIZE - 1 {
            if map[y * MAP_SIZE + x] != 0 {
                continue;
            }
            let mut score = 0;
            if map[y * MAP_SIZE + (x + 1)] == 0 {
                score += 1;
            }
            if map[y * MAP_SIZE + (x - 1)] == 0 {
                score += 1;
            }
            if map[(y + 1) * MAP_SIZE + x] == 0 {
                score += 1;
            }
            if map[(y - 1) * MAP_SIZE + x] == 0 {
                score += 1;
            }
            if score > best_score {
                best_score = score;
                best = (x, y);
            }
        }
    }
    best
}

/// Find the direction with the most open space from a cell
fn find_open_direction(map: &[u8], cx: usize, cy: usize) -> f32 {
    let dirs: [(i32, i32, f32); 4] = [
        (1, 0, 0.0),
        (0, 1, std::f32::consts::FRAC_PI_2),
        (-1, 0, std::f32::consts::PI),
        (0, -1, -std::f32::consts::FRAC_PI_2),
    ];

    let mut best_angle = 0.0_f32;
    let mut best_dist = 0;

    for &(dx, dy, angle) in &dirs {
        let mut dist = 0;
        let mut x = cx as i32;
        let mut y = cy as i32;
        loop {
            x += dx;
            y += dy;
            if x < 0 || y < 0 || x >= MAP_SIZE as i32 || y >= MAP_SIZE as i32 {
                break;
            }
            if map[y as usize * MAP_SIZE + x as usize] == 1 {
                break;
            }
            dist += 1;
        }
        if dist > best_dist {
            best_dist = dist;
            best_angle = angle;
        }
    }
    best_angle
}

/// Generate a maze using recursive backtracker (iterative with explicit stack)
fn generate_maze(rng: &mut Rng) -> Vec<u8> {
    let mut grid = vec![1u8; MAP_SIZE * MAP_SIZE];

    let cells_w = (MAP_SIZE - 1) / 2;
    let cells_h = (MAP_SIZE - 1) / 2;
    let mut visited = vec![false; cells_w * cells_h];

    let start_cx = 0usize;
    let start_cy = 0usize;
    visited[start_cy * cells_w + start_cx] = true;
    let gx = start_cx * 2 + 1;
    let gy = start_cy * 2 + 1;
    grid[gy * MAP_SIZE + gx] = 0;

    let mut stack: Vec<(usize, usize)> = vec![(start_cx, start_cy)];

    while let Some(&(cx, cy)) = stack.last() {
        let mut neighbors = Vec::new();
        if cx > 0 && !visited[cy * cells_w + (cx - 1)] {
            neighbors.push((cx - 1, cy));
        }
        if cx + 1 < cells_w && !visited[cy * cells_w + (cx + 1)] {
            neighbors.push((cx + 1, cy));
        }
        if cy > 0 && !visited[(cy - 1) * cells_w + cx] {
            neighbors.push((cx, cy - 1));
        }
        if cy + 1 < cells_h && !visited[(cy + 1) * cells_w + cx] {
            neighbors.push((cx, cy + 1));
        }

        if neighbors.is_empty() {
            stack.pop();
            continue;
        }

        let idx = rng.next_u32() as usize % neighbors.len();
        let (nx, ny) = neighbors[idx];

        let wall_gx = cx + nx + 1;
        let wall_gy = cy + ny + 1;
        grid[wall_gy * MAP_SIZE + wall_gx] = 0;

        let dest_gx = nx * 2 + 1;
        let dest_gy = ny * 2 + 1;
        grid[dest_gy * MAP_SIZE + dest_gx] = 0;

        visited[ny * cells_w + nx] = true;
        stack.push((nx, ny));
    }

    grid
}

/// Procedural brick wall texture with color tinting
fn build_brick_texture() -> Texture {
    let size = TEX_SIZE;
    let mut tex = Texture::new(size, size);
    let brick_w = 16u32;
    let brick_h = 8u32;

    for y in 0..size {
        for x in 0..size {
            let row = y / brick_h;
            let offset = if row % 2 == 0 { 0 } else { brick_w / 2 };
            let bx = (x + offset) % brick_w;
            let by = y % brick_h;

            let mortar = bx < 1 || by < 1;
            if mortar {
                tex.set_pixel(x, y, 40, 38, 35, 255);
            } else {
                let brick_id = ((y / brick_h) * 13 + ((x + offset) / brick_w) * 29) & 0xFF;
                let base = 130 + (brick_id & 0x3F) as u16;
                let v = base.min(255) as u8;
                let r = v;
                let g = (v as f32 * 0.75) as u8;
                let b = (v as f32 * 0.55) as u8;
                tex.set_pixel(x, y, r, g, b, 255);
            }
        }
    }
    tex
}

impl Effect for Raycaster {
    fn update(&mut self, dt: f32, _width: u32, _height: u32, _scene: &Scene) {
        self.time += dt;

        use std::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

        // Check if current turn is complete
        let mut angle_diff = self.target_angle - self.player_angle;
        while angle_diff > PI {
            angle_diff -= TAU;
        }
        while angle_diff < -PI {
            angle_diff += TAU;
        }

        if angle_diff.abs() < 0.05 {
            self.player_angle = self.target_angle;
            self.turning = false;
        }

        // Cast navigation rays and store for debug minimap
        self.debug_rays.clear();

        let fwd_dist = self.cast_ray(self.player_angle);
        let left_dist = self.cast_ray(self.player_angle - FRAC_PI_2);
        let right_dist = self.cast_ray(self.player_angle + FRAC_PI_2);
        let left_45 = self.cast_ray(self.player_angle - FRAC_PI_4);
        let right_45 = self.cast_ray(self.player_angle + FRAC_PI_4);

        // Forward ray (green)
        self.debug_rays.push(DebugRay {
            start_x: self.player_x,
            start_y: self.player_y,
            angle: self.player_angle,
            dist: fwd_dist,
            r: 0,
            g: 200,
            b: 0,
        });
        // Side rays (yellow)
        self.debug_rays.push(DebugRay {
            start_x: self.player_x,
            start_y: self.player_y,
            angle: self.player_angle - FRAC_PI_2,
            dist: left_dist,
            r: 200,
            g: 200,
            b: 0,
        });
        self.debug_rays.push(DebugRay {
            start_x: self.player_x,
            start_y: self.player_y,
            angle: self.player_angle + FRAC_PI_2,
            dist: right_dist,
            r: 200,
            g: 200,
            b: 0,
        });
        // Diagonal rays (magenta)
        self.debug_rays.push(DebugRay {
            start_x: self.player_x,
            start_y: self.player_y,
            angle: self.player_angle - FRAC_PI_4,
            dist: left_45,
            r: 150,
            g: 0,
            b: 150,
        });
        self.debug_rays.push(DebugRay {
            start_x: self.player_x,
            start_y: self.player_y,
            angle: self.player_angle + FRAC_PI_4,
            dist: right_45,
            r: 150,
            g: 0,
            b: 150,
        });

        // Look-ahead: project near the wall ahead and cast side rays from there
        // This detects side corridors at T-junctions before we reach them
        let look_ahead_dist = (fwd_dist - 0.5).clamp(0.1, 3.0);
        let ahead_x = self.player_x + self.player_angle.cos() * look_ahead_dist;
        let ahead_y = self.player_y + self.player_angle.sin() * look_ahead_dist;

        let ahead_left = self.cast_ray_from(ahead_x, ahead_y, self.player_angle - FRAC_PI_2);
        let ahead_right = self.cast_ray_from(ahead_x, ahead_y, self.player_angle + FRAC_PI_2);

        // Look-ahead rays (cyan)
        self.debug_rays.push(DebugRay {
            start_x: ahead_x,
            start_y: ahead_y,
            angle: self.player_angle - FRAC_PI_2,
            dist: ahead_left,
            r: 0,
            g: 200,
            b: 255,
        });
        self.debug_rays.push(DebugRay {
            start_x: ahead_x,
            start_y: ahead_y,
            angle: self.player_angle + FRAC_PI_2,
            dist: ahead_right,
            r: 0,
            g: 200,
            b: 255,
        });

        // Navigation decisions (only when NOT mid-turn)
        if !self.turning {
            if fwd_dist < 1.5 {
                // Approaching a wall — use look-ahead rays to detect side corridors
                // at the junction point ahead, not just from current position
                let (mut best_dist, mut best_turn) = if ahead_right > ahead_left {
                    (ahead_right, FRAC_PI_2)
                } else if ahead_left > 0.0 {
                    (ahead_left, -FRAC_PI_2)
                } else {
                    (0.0_f32, PI) // dead end: U-turn
                };

                // Diagonals can spot corridors too
                if left_45 > best_dist * 0.8 && left_45 > 2.0 {
                    best_dist = left_45;
                    best_turn = -FRAC_PI_2;
                }
                if right_45 > best_dist * 0.8 && right_45 > 2.0 {
                    best_dist = right_45;
                    best_turn = FRAC_PI_2;
                }

                if best_dist < 0.8 {
                    self.target_angle = self.player_angle + PI;
                } else {
                    self.target_angle = self.player_angle + best_turn;
                }
                self.turning = true;

                // Chosen direction (bright red)
                self.debug_rays.push(DebugRay {
                    start_x: self.player_x,
                    start_y: self.player_y,
                    angle: self.target_angle,
                    dist: 3.0,
                    r: 255,
                    g: 50,
                    b: 50,
                });
            } else {
                // Open corridor — occasionally explore side corridors
                if fwd_dist > 3.0 && self.rng.next_f32() < 0.005 {
                    if left_dist > 2.5 && self.rng.next_f32() < 0.5 {
                        self.target_angle = self.player_angle - FRAC_PI_2;
                        self.turning = true;
                    } else if right_dist > 2.5 {
                        self.target_angle = self.player_angle + FRAC_PI_2;
                        self.turning = true;
                    }
                }
            }
        }

        // Smooth rotation toward target
        angle_diff = self.target_angle - self.player_angle;
        while angle_diff > PI {
            angle_diff -= TAU;
        }
        while angle_diff < -PI {
            angle_diff += TAU;
        }

        let turn_speed = 3.0 * dt;
        if angle_diff.abs() < turn_speed {
            self.player_angle = self.target_angle;
        } else {
            self.player_angle += turn_speed * angle_diff.signum();
        }

        // Move forward — slow during turns, stop if very close to wall
        let fwd_now = self.cast_ray(self.player_angle);
        let speed_factor = if self.turning { 0.4 } else { 1.0 };
        if fwd_now > 0.4 {
            let cos_a = self.player_angle.cos();
            let sin_a = self.player_angle.sin();
            let step = self.move_speed * speed_factor * dt;
            let new_x = self.player_x + cos_a * step;
            let new_y = self.player_y + sin_a * step;

            let margin = 0.25;
            if !self.is_wall(
                (new_x + margin * cos_a.signum()) as i32,
                self.player_y as i32,
            ) {
                self.player_x = new_x;
            }
            if !self.is_wall(
                self.player_x as i32,
                (new_y + margin * sin_a.signum()) as i32,
            ) {
                self.player_y = new_y;
            }
        }
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        let w = buffer.width() as i32;
        let h = buffer.height() as i32;
        let half_h = h as f32 / 2.0;
        let pixels = buffer.as_bytes_mut();
        let fov = std::f32::consts::FRAC_PI_3;
        let tex_w = self.wall_texture.width() as f32;
        let tex_h = self.wall_texture.height() as f32;

        for col in 0..w {
            let ray_offset = (col as f32 / w as f32 - 0.5) * fov;
            let ray_angle = self.player_angle + ray_offset;
            let ray_cos = ray_angle.cos();
            let ray_sin = ray_angle.sin();

            // DDA setup
            let mut map_x = self.player_x as i32;
            let mut map_y = self.player_y as i32;

            let delta_dist_x = if ray_cos == 0.0 {
                f32::MAX
            } else {
                (1.0 / ray_cos).abs()
            };
            let delta_dist_y = if ray_sin == 0.0 {
                f32::MAX
            } else {
                (1.0 / ray_sin).abs()
            };

            let (step_x, mut side_dist_x) = if ray_cos < 0.0 {
                (-1, (self.player_x - map_x as f32) * delta_dist_x)
            } else {
                (1, (map_x as f32 + 1.0 - self.player_x) * delta_dist_x)
            };
            let (step_y, mut side_dist_y) = if ray_sin < 0.0 {
                (-1, (self.player_y - map_y as f32) * delta_dist_y)
            } else {
                (1, (map_y as f32 + 1.0 - self.player_y) * delta_dist_y)
            };

            let mut hit_side = 0;
            let mut hit = false;

            for _ in 0..64 {
                if side_dist_x < side_dist_y {
                    side_dist_x += delta_dist_x;
                    map_x += step_x;
                    hit_side = 0;
                } else {
                    side_dist_y += delta_dist_y;
                    map_y += step_y;
                    hit_side = 1;
                }

                if self.is_wall(map_x, map_y) {
                    hit = true;
                    break;
                }
            }

            let perp_dist = if !hit {
                f32::MAX
            } else if hit_side == 0 {
                (map_x as f32 - self.player_x + (1 - step_x) as f32 * 0.5) / ray_cos
            } else {
                (map_y as f32 - self.player_y + (1 - step_y) as f32 * 0.5) / ray_sin
            };

            let wall_h = if perp_dist > 0.001 {
                (h as f32 / perp_dist).min(h as f32 * 4.0)
            } else {
                h as f32 * 4.0
            };

            let draw_start = ((half_h - wall_h / 2.0) as i32).max(0);
            let draw_end = ((half_h + wall_h / 2.0) as i32).min(h);

            let wall_x = if hit_side == 0 {
                self.player_y + perp_dist * ray_sin
            } else {
                self.player_x + perp_dist * ray_cos
            };
            let wall_x = wall_x - wall_x.floor();
            let tex_x = (wall_x * tex_w) as u32 % self.wall_texture.width();

            let side_dim: f32 = if hit_side == 1 { 0.7 } else { 1.0 };
            let fog = (1.0 - (perp_dist / 10.0).min(1.0)).max(0.05) * side_dim;

            // Ceiling
            for row in 0..draw_start {
                let idx = (row * w + col) as usize * 4;
                let t = 1.0 - (row as f32 / half_h);
                let cr = (8.0 * t) as u8;
                let cg = (12.0 * t) as u8;
                let cb = (30.0 * t) as u8;
                pixels[idx] = 255;
                pixels[idx + 1] = cb;
                pixels[idx + 2] = cg;
                pixels[idx + 3] = cr;
            }

            // Wall stripe
            for row in draw_start..draw_end {
                let idx = (row * w + col) as usize * 4;
                let d = (row as f32 - half_h + wall_h / 2.0) / wall_h;
                let tex_y = (d * tex_h) as u32 % self.wall_texture.height();

                let (tr, tg, tb) = self
                    .wall_texture
                    .sample(tex_x as f32 / tex_w, tex_y as f32 / tex_h);

                pixels[idx] = 255;
                pixels[idx + 1] = (tb as f32 * fog) as u8;
                pixels[idx + 2] = (tg as f32 * fog) as u8;
                pixels[idx + 3] = (tr as f32 * fog) as u8;
            }

            // Floor
            for row in draw_end..h {
                let idx = (row * w + col) as usize * 4;
                let t = (row as f32 - half_h) / half_h;
                let c = (20.0 * t) as u8;
                pixels[idx] = 255;
                pixels[idx + 1] = c / 3;
                pixels[idx + 2] = c / 2;
                pixels[idx + 3] = c;
            }
        }

        // Debug minimap overlay
        self.draw_minimap(buffer);
    }

    fn name(&self) -> &str {
        "Raycaster Maze"
    }
}
