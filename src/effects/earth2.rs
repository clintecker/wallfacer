//! Earth 2.0 — Gouraud-Shaded Globe with Bloom
//!
//! Upgraded globe using per-vertex texture sampling and lighting,
//! smooth Gouraud shading, specular ocean highlights, gradient atmosphere,
//! subpixel stars, AA shooting stars, and full-frame bloom.

use std::f32::consts::{PI, TAU};

use super::Effect;
use crate::display::PixelBuffer;
use crate::geometry::{rect_polygon_collision, reflect};
use crate::math3d::{project, Mesh, Vec3};
use crate::regions::Scene;
use crate::texture::Texture;

// Texture dimensions (power of 2 width for fast sampling)
const TEX_W: u32 = 512;
const TEX_H: u32 = 256;

// ============================================================================
// Value Noise with FBM
// ============================================================================

fn noise_hash(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let mut h = seed.wrapping_add(x as u32).wrapping_mul(374761393);
    h = h.wrapping_add(y as u32).wrapping_mul(668265263);
    h = h.wrapping_add(z as u32).wrapping_mul(2147483647);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h = h ^ (h >> 16);
    (h & 0x7fff) as f32 / 0x7fff as f32
}

fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn value_noise(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let iz = z.floor() as i32;
    let fx = smoothstep(x - ix as f32);
    let fy = smoothstep(y - iy as f32);
    let fz = smoothstep(z - iz as f32);

    let c000 = noise_hash(ix, iy, iz, seed);
    let c100 = noise_hash(ix + 1, iy, iz, seed);
    let c010 = noise_hash(ix, iy + 1, iz, seed);
    let c110 = noise_hash(ix + 1, iy + 1, iz, seed);
    let c001 = noise_hash(ix, iy, iz + 1, seed);
    let c101 = noise_hash(ix + 1, iy, iz + 1, seed);
    let c011 = noise_hash(ix, iy + 1, iz + 1, seed);
    let c111 = noise_hash(ix + 1, iy + 1, iz + 1, seed);

    let x0 = c000 + (c100 - c000) * fx;
    let x1 = c010 + (c110 - c010) * fx;
    let x2 = c001 + (c101 - c001) * fx;
    let x3 = c011 + (c111 - c011) * fx;

    let y0 = x0 + (x1 - x0) * fy;
    let y1 = x2 + (x3 - x2) * fy;

    y0 + (y1 - y0) * fz
}

fn fbm(x: f32, y: f32, z: f32, octaves: u32, seed: u32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 0.5;
    let mut frequency = 1.0;
    for _ in 0..octaves {
        value += amplitude * value_noise(x * frequency, y * frequency, z * frequency, seed);
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    value
}

// ============================================================================
// Texture generation
// ============================================================================

fn uv_to_sphere(u: f32, v: f32) -> (f32, f32, f32) {
    let lon = u * TAU;
    let lat = (v - 0.5) * PI;
    let cos_lat = lat.cos();
    (cos_lat * lon.cos(), lat.sin(), cos_lat * lon.sin())
}

fn generate_earth_texture() -> Texture {
    let mut data = vec![0u8; (TEX_W * TEX_H * 4) as usize];
    let seed = 42;
    let noise_scale = 3.0;
    let land_threshold = 0.45;

    for py in 0..TEX_H {
        let v = py as f32 / TEX_H as f32;
        let latitude = (v - 0.5) * PI;
        let abs_lat = latitude.abs();

        for px in 0..TEX_W {
            let u = px as f32 / TEX_W as f32;
            let (sx, sy, sz) = uv_to_sphere(u, v);

            let n = fbm(
                sx * noise_scale,
                sy * noise_scale,
                sz * noise_scale,
                4,
                seed,
            );

            let (r, g, b) = if abs_lat > 1.2 {
                let ice_blend = ((abs_lat - 1.2) / 0.2).min(1.0);
                let base_r = 200 + (55.0 * ice_blend) as u8;
                let base_g = 210 + (45.0 * ice_blend) as u8;
                let base_b = 220 + (35.0 * ice_blend) as u8;
                (base_r, base_g, base_b)
            } else if n > land_threshold {
                let land_t = ((n - land_threshold) / (1.0 - land_threshold)).min(1.0);
                let polar_factor = (abs_lat / 1.2).powi(2);
                let gr = 40.0 + land_t * 80.0 + polar_factor * 60.0;
                let gg = 120.0 - land_t * 40.0 - polar_factor * 40.0;
                let gb = 30.0 + land_t * 20.0 + polar_factor * 30.0;
                let tundra_blend = ((abs_lat - 0.9) / 0.3).clamp(0.0, 1.0);
                let tr = gr + (180.0 - gr) * tundra_blend;
                let tg = gg + (175.0 - gg) * tundra_blend;
                let tb = gb + (160.0 - gb) * tundra_blend;
                (
                    tr.min(255.0) as u8,
                    tg.min(255.0) as u8,
                    tb.min(255.0) as u8,
                )
            } else {
                let depth = 1.0 - (n / land_threshold);
                let or = (10.0 + depth * 15.0) as u8;
                let og = (30.0 + depth * 40.0 + n * 30.0) as u8;
                let ob = (80.0 + depth * 80.0 + n * 20.0) as u8;
                (or, og, ob)
            };

            let idx = ((py * TEX_W + px) * 4) as usize;
            data[idx] = r;
            data[idx + 1] = g;
            data[idx + 2] = b;
            data[idx + 3] = 255;
        }
    }

    Texture::from_rgba(TEX_W, TEX_H, data).unwrap()
}

fn generate_cloud_texture() -> Texture {
    let mut data = vec![0u8; (TEX_W * TEX_H * 4) as usize];
    let cloud_threshold = 0.38;

    for py in 0..TEX_H {
        let v = py as f32 / TEX_H as f32;

        for px in 0..TEX_W {
            let u = px as f32 / TEX_W as f32;
            let (sx, sy, sz) = uv_to_sphere(u, v);

            // Large-scale cloud formations (seed 137)
            let base = fbm(sx * 3.5, sy * 3.5, sz * 3.5, 5, 137);
            // Fine detail layer for wispy edges (seed 251)
            let detail = fbm(sx * 8.0, sy * 8.0, sz * 8.0, 3, 251);
            // Blend: base shapes the formations, detail breaks up edges
            let n = base * 0.75 + detail * 0.25;

            let alpha = if n > cloud_threshold {
                let t = ((n - cloud_threshold) / (1.0 - cloud_threshold)).min(1.0);
                // Steep curve: crisp cloud bodies with narrow soft fringe
                (t.powf(0.3) * 170.0).min(170.0) as u8
            } else {
                0
            };

            let idx = ((py * TEX_W + px) * 4) as usize;
            data[idx] = 255;
            data[idx + 1] = 255;
            data[idx + 2] = 255;
            data[idx + 3] = alpha;
        }
    }

    Texture::from_rgba(TEX_W, TEX_H, data).unwrap()
}

// ============================================================================
// Background sky types
// ============================================================================

struct Star {
    x: f32,
    y: f32,
    base_bright: f32,
    twinkle_speed: f32,
    twinkle_depth: f32,
    phase: f32,
    color_temp: u8,
}

struct Starburst {
    x: f32,
    y: f32,
    age: f32,
    duration: f32,
    peak_bright: f32,
    spike_len: f32,
}

struct ShootingStar {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    age: f32,
    duration: f32,
    trail_len: f32,
    brightness: f32,
}

// ============================================================================
// UV helper
// ============================================================================

fn point_to_uv(p: Vec3) -> (f32, f32) {
    let n = p.normalize();
    let u = 0.5 + n.z.atan2(n.x) / TAU;
    let v = 0.5 - n.y.asin() / PI;
    (u, v)
}

// ============================================================================
// Earth2 effect
// ============================================================================

pub struct Earth2 {
    time: f32,
    earth_mesh: Mesh,
    earth_texture: Texture,
    cloud_texture: Texture,
    earth_rotation: Vec3,
    cloud_rotation: Vec3,
    light_dir: Vec3,
    // Per-vertex precomputed texture colors
    earth_vertex_colors: Vec<(u8, u8, u8)>,
    // Bounce state
    pos_x: f32,
    pos_y: f32,
    vel_x: f32,
    vel_y: f32,
    // Background sky
    stars: Vec<Star>,
    starbursts: Vec<Starburst>,
    shooting_stars: Vec<ShootingStar>,
    next_burst_timer: f32,
    next_shoot_timer: f32,
    rng_state: u64,
}

impl Earth2 {
    fn xorshift(state: &mut u64) -> u64 {
        *state ^= *state << 13;
        *state ^= *state >> 7;
        *state ^= *state << 17;
        *state
    }

    fn rng_f32(state: &mut u64) -> f32 {
        Self::xorshift(state);
        (*state % 10000) as f32 / 10000.0
    }

    pub fn new() -> Self {
        let mut rng = 9876543u64;

        let mut stars = Vec::with_capacity(300);
        for _ in 0..300 {
            stars.push(Star {
                x: Self::rng_f32(&mut rng),
                y: Self::rng_f32(&mut rng),
                base_bright: 60.0 + Self::rng_f32(&mut rng) * 195.0,
                twinkle_speed: 1.5 + Self::rng_f32(&mut rng) * 6.0,
                twinkle_depth: 0.3 + Self::rng_f32(&mut rng) * 0.7,
                phase: Self::rng_f32(&mut rng) * TAU,
                color_temp: (Self::xorshift(&mut rng) % 3) as u8,
            });
        }

        let earth_mesh = Mesh::sphere(150.0, 3);
        let earth_texture = generate_earth_texture();
        let cloud_texture = generate_cloud_texture();

        // Precompute per-vertex texture colors for earth
        let earth_vertex_colors: Vec<(u8, u8, u8)> = earth_mesh
            .vertices
            .iter()
            .map(|v| {
                let (u, v) = point_to_uv(v.normalize());
                earth_texture.sample(u, v)
            })
            .collect();

        Self {
            time: 0.0,
            earth_mesh,
            earth_texture,
            cloud_texture,
            earth_rotation: Vec3::zero(),
            cloud_rotation: Vec3::zero(),
            light_dir: Vec3::new(0.8, 0.3, -0.5).normalize(),
            earth_vertex_colors,
            pos_x: 320.0,
            pos_y: 240.0,
            vel_x: 55.0,
            vel_y: 35.0,
            stars,
            starbursts: Vec::new(),
            shooting_stars: Vec::new(),
            next_burst_timer: 1.5,
            next_shoot_timer: 3.0,
            rng_state: rng,
        }
    }
}

impl Default for Earth2 {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Earth2 {
    fn update(&mut self, dt: f32, width: u32, height: u32, scene: &Scene) {
        self.time += dt;
        self.earth_rotation.y += 0.15 * dt;
        self.earth_rotation.x = 0.4;
        self.cloud_rotation.y += 0.20 * dt;
        self.cloud_rotation.x = 0.4;

        // DVD-style bounce — scale camera distance to screen size
        let fov = 400.0;
        let min_dim = width.min(height) as f32;
        let camera_z = 400_000.0 / min_dim;
        let earth_screen_r = 150.0 * fov / camera_z;
        let visual_radius = earth_screen_r * 1.30;
        let screen_w = width as f32;
        let screen_h = height as f32;

        let new_x = self.pos_x + self.vel_x * dt;
        let new_y = self.pos_y + self.vel_y * dt;

        if new_x - visual_radius <= 0.0 {
            self.pos_x = visual_radius;
            self.vel_x = self.vel_x.abs();
        } else if new_x + visual_radius >= screen_w {
            self.pos_x = screen_w - visual_radius;
            self.vel_x = -self.vel_x.abs();
        } else {
            self.pos_x = new_x;
        }

        if new_y - visual_radius <= 0.0 {
            self.pos_y = visual_radius;
            self.vel_y = self.vel_y.abs();
        } else if new_y + visual_radius >= screen_h {
            self.pos_y = screen_h - visual_radius;
            self.vel_y = -self.vel_y.abs();
        } else {
            self.pos_y = new_y;
        }

        // Polygon region collision (bounding square around globe)
        let side = visual_radius * 2.0;
        for region in &scene.regions {
            let verts = region.polygon.as_tuples();
            if let Some((nx, ny, dist)) = rect_polygon_collision(
                self.pos_x - visual_radius,
                self.pos_y - visual_radius,
                side,
                side,
                &verts,
            ) {
                let (new_vx, new_vy) = reflect(self.vel_x, self.vel_y, nx, ny);
                self.vel_x = new_vx;
                self.vel_y = new_vy;
                let push_dist = dist + 10.0;
                self.pos_x += nx * push_dist;
                self.pos_y += ny * push_dist;
                break;
            }
        }

        // --- Spawn starbursts ---
        self.next_burst_timer -= dt;
        if self.next_burst_timer <= 0.0 {
            let idx = (Self::xorshift(&mut self.rng_state) % self.stars.len() as u64) as usize;
            let star = &self.stars[idx];
            self.starbursts.push(Starburst {
                x: star.x,
                y: star.y,
                age: 0.0,
                duration: 0.4 + Self::rng_f32(&mut self.rng_state) * 0.8,
                peak_bright: 200.0 + Self::rng_f32(&mut self.rng_state) * 55.0,
                spike_len: 8.0 + Self::rng_f32(&mut self.rng_state) * 20.0,
            });
            self.next_burst_timer = 0.8 + Self::rng_f32(&mut self.rng_state) * 1.7;
        }
        for burst in &mut self.starbursts {
            burst.age += dt;
        }
        self.starbursts.retain(|b| b.age < b.duration);

        // --- Spawn shooting stars ---
        self.next_shoot_timer -= dt;
        if self.next_shoot_timer <= 0.0 {
            let start_x = Self::rng_f32(&mut self.rng_state) * screen_w;
            let start_y = Self::rng_f32(&mut self.rng_state) * screen_h * 0.4;
            let angle = 0.3 + Self::rng_f32(&mut self.rng_state) * 0.8;
            let speed = 300.0 + Self::rng_f32(&mut self.rng_state) * 400.0;
            self.shooting_stars.push(ShootingStar {
                x: start_x,
                y: start_y,
                vx: angle.cos() * speed,
                vy: angle.sin() * speed,
                age: 0.0,
                duration: 0.4 + Self::rng_f32(&mut self.rng_state) * 0.6,
                trail_len: 30.0 + Self::rng_f32(&mut self.rng_state) * 60.0,
                brightness: 180.0 + Self::rng_f32(&mut self.rng_state) * 75.0,
            });
            self.next_shoot_timer = 2.0 + Self::rng_f32(&mut self.rng_state) * 4.0;
        }
        for star in &mut self.shooting_stars {
            star.x += star.vx * dt;
            star.y += star.vy * dt;
            star.age += dt;
        }
        self.shooting_stars.retain(|s| s.age < s.duration);
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        let width = buffer.width() as f32;
        let height = buffer.height() as f32;
        let fov = 400.0;
        let min_dim = width.min(height);
        let camera_z = 400_000.0 / min_dim;
        let cx = self.pos_x;
        let cy = self.pos_y;

        buffer.clear(0, 0, 0);

        // ---- 1. Stars (subpixel) ----
        for star in &self.stars {
            let primary = (self.time * star.twinkle_speed + star.phase).sin();
            let flutter = (self.time * star.twinkle_speed * 3.7 + star.phase * 2.3).sin() * 0.3;
            let twinkle = ((primary + flutter) * star.twinkle_depth + (1.0 - star.twinkle_depth))
                .clamp(0.0, 1.0);

            let bright = (star.base_bright * twinkle).min(255.0);
            let b = bright as u8;

            let (r, g, bl) = match star.color_temp {
                0 => (b.saturating_sub(20), b.saturating_sub(10), b),
                2 => (b, b.saturating_sub(15), b.saturating_sub(40)),
                _ => (b, b, b),
            };

            buffer.splat_pixel(star.x * width, star.y * height, r, g, bl, twinkle);
        }

        // ---- 2. Starbursts (gradient core + AA spikes) ----
        for burst in &self.starbursts {
            let bx = burst.x * width;
            let by = burst.y * height;

            let t = burst.age / burst.duration;
            let envelope = if t < 0.15 {
                t / 0.15
            } else {
                ((1.0 - t) / 0.85).max(0.0)
            };

            let bright = (burst.peak_bright * envelope).min(255.0);
            let spike = burst.spike_len * envelope;

            // Gradient core
            buffer.fill_circle_gradient(bx as i32, by as i32, 3, 255, 255, 255, 1.5);

            let b = bright as u8;

            // Cardinal spikes (AA lines)
            buffer.line_aa_additive(bx, by, bx + spike, by, b, b, b);
            buffer.line_aa_additive(bx, by, bx - spike, by, b, b, b);
            buffer.line_aa_additive(bx, by, bx, by + spike, b, b, b);
            buffer.line_aa_additive(bx, by, bx, by - spike, b, b, b);

            // Diagonal spikes (shorter, dimmer)
            let diag = spike * 0.66;
            let db = (bright * 0.6).min(255.0) as u8;
            buffer.line_aa_additive(bx, by, bx + diag, by + diag, db, db, db);
            buffer.line_aa_additive(bx, by, bx - diag, by - diag, db, db, db);
            buffer.line_aa_additive(bx, by, bx + diag, by - diag, db, db, db);
            buffer.line_aa_additive(bx, by, bx - diag, by + diag, db, db, db);
        }

        // ---- 3. Shooting stars (AA trails) ----
        for star in &self.shooting_stars {
            let t = star.age / star.duration;
            let envelope = if t < 0.1 {
                t / 0.1
            } else {
                ((1.0 - t) / 0.9).max(0.0)
            };

            let bright = (star.brightness * envelope).min(255.0);
            let speed = (star.vx * star.vx + star.vy * star.vy).sqrt();
            let dx = -star.vx / speed;
            let dy = -star.vy / speed;
            let trail = star.trail_len * envelope;

            // 3 segments with decreasing brightness for fade trail
            let segments = 3;
            let seg_len = trail / segments as f32;
            for s in 0..segments {
                let falloff = (1.0 - s as f32 / segments as f32).powi(2);
                let seg_bright = (bright * falloff).min(255.0) as u8;
                let x0 = star.x + dx * seg_len * s as f32;
                let y0 = star.y + dy * seg_len * s as f32;
                let x1 = star.x + dx * seg_len * (s + 1) as f32;
                let y1 = star.y + dy * seg_len * (s + 1) as f32;
                buffer.line_aa_additive(x0, y0, x1, y1, seg_bright, seg_bright, seg_bright);
            }
        }

        // ---- 4. Subtle background glow (drawn before planet, mostly hidden) ----
        let sphere_screen_radius = 150.0 * fov / camera_z;
        let center_x = cx as i32;
        let center_y = cy as i32;
        let atmo_padding = (sphere_screen_radius * 0.30) as i32;
        buffer.fill_circle_gradient(
            center_x,
            center_y,
            sphere_screen_radius as i32 + atmo_padding,
            15,
            30,
            80,
            2.0,
        );

        // ---- 5. Earth sphere (Gouraud shaded) ----
        self.render_earth(buffer, fov, camera_z, cx, cy);

        // ---- 6. Cloud layer (Gouraud shaded + alpha blended) ----
        self.render_clouds(buffer, fov, camera_z, cx, cy);

        // ---- 7. Atmosphere rim aura (additive, drawn after planet) ----
        {
            let inner = sphere_screen_radius * 0.93; // starts just inside limb
            let outer = sphere_screen_radius * 1.28; // extends well beyond
            let range = outer - inner;
            let bx0 = ((cx - outer) as i32).max(0);
            let by0 = ((cy - outer) as i32).max(0);
            let bx1 = ((cx + outer) as i32 + 1).min(buffer.width() as i32);
            let by1 = ((cy + outer) as i32 + 1).min(buffer.height() as i32);

            for py in by0..by1 {
                let dy = py as f32 - cy;
                let dy2 = dy * dy;
                for px in bx0..bx1 {
                    let dx = px as f32 - cx;
                    let dist = (dx * dx + dy2).sqrt();
                    if dist > inner && dist < outer {
                        let t = 1.0 - (dist - inner) / range;
                        let intensity = t * t * t; // cubic falloff — bright at limb, soft fade out
                        let r = (30.0 * intensity) as u8;
                        let g = (100.0 * intensity) as u8;
                        let b = (255.0 * intensity) as u8;
                        buffer.blend_pixel_additive(px, py, r, g, b);
                    }
                }
            }
        }

        // ---- 8. Bloom post-processing ----
        buffer.bloom(200, 2, 0.5);
    }

    fn name(&self) -> &str {
        "Earth 2.0"
    }

    fn region_color(&self) -> (u8, u8, u8) {
        (10, 20, 45)
    }
}

impl Earth2 {
    fn render_earth(&self, buffer: &mut PixelBuffer, fov: f32, camera_z: f32, cx: f32, cy: f32) {
        let mesh = &self.earth_mesh;
        let rotation = &self.earth_rotation;

        // Transform vertices: rotate then push along Z for camera
        let transformed: Vec<Vec3> = mesh
            .vertices
            .iter()
            .map(|v| {
                v.rotate_x(rotation.x)
                    .rotate_y(rotation.y)
                    .rotate_z(rotation.z)
                    + Vec3::new(0.0, 0.0, camera_z)
            })
            .collect();

        // World-space rotated vertices (before camera translation) for lighting
        let world_verts: Vec<Vec3> = mesh
            .vertices
            .iter()
            .map(|v| {
                v.rotate_x(rotation.x)
                    .rotate_y(rotation.y)
                    .rotate_z(rotation.z)
            })
            .collect();

        // Per-vertex lighting
        let view_dir = Vec3::new(0.0, 0.0, -1.0); // camera looks along -Z in world space
        let lit_colors: Vec<(u8, u8, u8)> = world_verts
            .iter()
            .enumerate()
            .map(|(i, wv)| {
                let normal = wv.normalize();
                let diff = normal.dot(&self.light_dir).max(0.05);

                let (tr, tg, tb) = self.earth_vertex_colors[i];

                // Detect ocean: blue > green in base texture color
                let is_ocean = tb > tg;

                let mut fr = tr as f32 * diff;
                let mut fg = tg as f32 * diff;
                let mut fb = tb as f32 * diff;

                if is_ocean {
                    // Blinn-Phong specular for ocean
                    let half = (self.light_dir + view_dir).normalize();
                    let spec = normal.dot(&half).max(0.0).powf(10.0) * 0.35;
                    let spec255 = spec * 255.0;
                    fr = (fr + spec255).min(255.0);
                    fg = (fg + spec255).min(255.0);
                    fb = (fb + spec255).min(255.0);
                }

                (fr as u8, fg as u8, fb as u8)
            })
            .collect();

        // Backface cull + depth sort
        let mut visible_faces: Vec<(usize, f32)> = mesh
            .faces
            .iter()
            .enumerate()
            .filter_map(|(i, face)| {
                let v0 = transformed[face[0]];
                let v1 = transformed[face[1]];
                let v2 = transformed[face[2]];
                let edge1 = v1 - v0;
                let edge2 = v2 - v0;
                let normal = edge1.cross(&edge2).normalize();
                if normal.z >= 0.0 {
                    return None;
                }
                let center_z = (v0.z + v1.z + v2.z) / 3.0;
                Some((i, center_z))
            })
            .collect();

        visible_faces.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        for (face_idx, _) in visible_faces {
            let face = &mesh.faces[face_idx];

            // Project vertices to screen + gather lit colors
            let mut gouraud_verts: Vec<(f32, f32, u8, u8, u8)> = Vec::with_capacity(3);
            let mut all_visible = true;

            for &vi in face {
                if let Some((sx, sy)) = project(transformed[vi], fov, cx, cy) {
                    let (r, g, b) = lit_colors[vi];
                    gouraud_verts.push((sx, sy, r, g, b));
                } else {
                    all_visible = false;
                    break;
                }
            }

            if !all_visible || gouraud_verts.len() < 3 {
                continue;
            }

            buffer.fill_polygon_gouraud(&gouraud_verts);
        }
    }

    fn render_clouds(&self, buffer: &mut PixelBuffer, fov: f32, camera_z: f32, cx: f32, cy: f32) {
        let cloud_r = 155.0_f32;
        let rot = &self.cloud_rotation;

        // Screen-space bounding box for cloud sphere
        let screen_r = (cloud_r * fov / (camera_z - cloud_r)) + 2.0;
        let w = buffer.width();
        let bx0 = ((cx - screen_r) as i32).max(0);
        let by0 = ((cy - screen_r) as i32).max(0);
        let bx1 = ((cx + screen_r) as i32 + 1).min(w as i32);
        let by1 = ((cy + screen_r) as i32 + 1).min(buffer.height() as i32);

        // Quadratic coefficients (c term is constant for the whole sphere)
        let c_term = camera_z * camera_z - cloud_r * cloud_r;

        let bytes = buffer.as_bytes_mut();

        for py in by0..by1 {
            let dy = (py as f32 - cy) / fov;
            let dy2 = dy * dy;

            for px in bx0..bx1 {
                let dx = (px as f32 - cx) / fov;

                // Ray-sphere intersection: ray from (0,0,0) dir (dx, dy, 1)
                let a = dx * dx + dy2 + 1.0;
                let disc = camera_z * camera_z - a * c_term;
                if disc < 0.0 {
                    continue;
                }

                let t = (camera_z - disc.sqrt()) / a;

                // Normal at hit point (hit relative to sphere center, divided by R)
                let nx = t * dx / cloud_r;
                let ny = t * dy / cloud_r;
                let nz = (t - camera_z) / cloud_r;

                // Inverse-rotate to get unrotated position for UV lookup
                let unrot = Vec3::new(nx, ny, nz)
                    .rotate_z(-rot.z)
                    .rotate_y(-rot.y)
                    .rotate_x(-rot.x);

                let (u, v) = point_to_uv(unrot);
                let (_, _, _, alpha) = self.cloud_texture.sample_rgba(u, v);
                if alpha < 5 {
                    continue;
                }

                // Per-pixel diffuse lighting (higher ambient for clouds)
                let normal = Vec3::new(nx, ny, nz);
                let diff = normal.dot(&self.light_dir).max(0.18);
                let bright = (255.0 * diff) as u16;

                // Alpha-blend white cloud onto buffer
                let a16 = alpha as u16;
                let inv = 255 - a16;
                let idx = ((py as u32 * w + px as u32) * 4) as usize;
                bytes[idx + 1] = ((bright * a16 + bytes[idx + 1] as u16 * inv) / 255) as u8;
                bytes[idx + 2] = ((bright * a16 + bytes[idx + 2] as u16 * inv) / 255) as u8;
                bytes[idx + 3] = ((bright * a16 + bytes[idx + 3] as u16 * inv) / 255) as u8;
            }
        }
    }
}
