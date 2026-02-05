//! Earth Globe Effect
//!
//! A rotating 3D sphere with procedural land/ocean, a cloud layer,
//! directional lighting, and an atmosphere glow.

use std::f32::consts::{PI, TAU};

use super::Effect;
use crate::display::PixelBuffer;
use crate::geometry::{circle_polygon_collision, reflect};
use crate::math3d::{project, Mesh, Vec3};
use crate::noise::fbm;
use crate::regions::Scene;
use crate::texture::Texture;
use crate::util::Rng;

// Texture dimensions (power of 2 width for fast sampling)
const TEX_W: u32 = 512;
const TEX_H: u32 = 256;

// ============================================================================
// Texture generation
// ============================================================================

/// Convert equirectangular UV to a 3D point on unit sphere (for noise sampling)
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
        let latitude = (v - 0.5) * PI; // -PI/2 to PI/2
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
                // Ice caps — near poles
                let ice_blend = ((abs_lat - 1.2) / 0.2).min(1.0);
                let base_r = 200 + (55.0 * ice_blend) as u8;
                let base_g = 210 + (45.0 * ice_blend) as u8;
                let base_b = 220 + (35.0 * ice_blend) as u8;
                (base_r, base_g, base_b)
            } else if n > land_threshold {
                // Land — gradient from green to brown based on noise + latitude
                let land_t = ((n - land_threshold) / (1.0 - land_threshold)).min(1.0);

                // Higher elevation = more brown/rocky, lower = greener
                let polar_factor = (abs_lat / 1.2).powi(2);

                // Green lowlands
                let gr = 40.0 + land_t * 80.0 + polar_factor * 60.0;
                let gg = 120.0 - land_t * 40.0 - polar_factor * 40.0;
                let gb = 30.0 + land_t * 20.0 + polar_factor * 30.0;

                // Mix toward tundra near poles
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
                // Ocean — deep blue with subtle variation
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
    let seed = 137; // Different seed from earth
    let noise_scale = 4.5;
    let cloud_threshold = 0.42;

    for py in 0..TEX_H {
        let v = py as f32 / TEX_H as f32;

        for px in 0..TEX_W {
            let u = px as f32 / TEX_W as f32;
            let (sx, sy, sz) = uv_to_sphere(u, v);

            let n = fbm(
                sx * noise_scale,
                sy * noise_scale,
                sz * noise_scale,
                3,
                seed,
            );

            let alpha = if n > cloud_threshold {
                let t = ((n - cloud_threshold) / (1.0 - cloud_threshold)).min(1.0);
                (t * 180.0) as u8
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
    x: f32, // 0..1 normalized screen position
    y: f32,
    base_bright: f32,   // base brightness 0..255
    twinkle_speed: f32, // radians per second
    twinkle_depth: f32, // 0..1 how much brightness varies
    phase: f32,         // initial phase offset
    color_temp: u8,     // 0=blue-white, 1=white, 2=warm
}

struct Starburst {
    x: f32, // screen position 0..1
    y: f32,
    age: f32,         // seconds since start
    duration: f32,    // total lifetime
    peak_bright: f32, // max brightness at peak
    spike_len: f32,   // max spike length in pixels
}

struct ShootingStar {
    x: f32, // current head position (pixels)
    y: f32,
    vx: f32, // velocity pixels/sec
    vy: f32,
    age: f32,
    duration: f32,  // total lifetime
    trail_len: f32, // trail length in pixels
    brightness: f32,
}

// ============================================================================
// Earth effect
// ============================================================================

pub struct Earth {
    time: f32,
    earth_mesh: Mesh,
    cloud_mesh: Mesh,
    earth_texture: Texture,
    cloud_texture: Texture,
    earth_rotation: Vec3,
    cloud_rotation: Vec3,
    light_dir: Vec3,
    // Bounce state (screen-space center position)
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
    rng: Rng,
}

impl Earth {
    pub fn new() -> Self {
        let mut rng = Rng::new(9876543);

        let mut stars = Vec::with_capacity(300);
        for _ in 0..300 {
            stars.push(Star {
                x: rng.next_f32(),
                y: rng.next_f32(),
                base_bright: 60.0 + rng.next_f32() * 195.0,
                twinkle_speed: 1.5 + rng.next_f32() * 6.0,
                twinkle_depth: 0.3 + rng.next_f32() * 0.7,
                phase: rng.next_f32() * TAU,
                color_temp: (rng.next_u64() % 3) as u8,
            });
        }

        Self {
            time: 0.0,
            earth_mesh: Mesh::sphere(150.0, 3),
            cloud_mesh: Mesh::sphere(155.0, 2),
            earth_texture: generate_earth_texture(),
            cloud_texture: generate_cloud_texture(),
            earth_rotation: Vec3::zero(),
            cloud_rotation: Vec3::zero(),
            light_dir: Vec3::new(0.8, 0.3, -0.5).normalize(),
            pos_x: 320.0,
            pos_y: 240.0,
            vel_x: 55.0,
            vel_y: 35.0,
            stars,
            starbursts: Vec::new(),
            shooting_stars: Vec::new(),
            next_burst_timer: 1.5,
            next_shoot_timer: 3.0,
            rng,
        }
    }
}

impl Default for Earth {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a 3D point on the sphere to equirectangular UV
fn point_to_uv(p: Vec3) -> (f32, f32) {
    let n = p.normalize();
    let u = 0.5 + n.z.atan2(n.x) / TAU;
    let v = 0.5 - n.y.asin() / PI;
    (u, v)
}

impl Effect for Earth {
    fn update(&mut self, dt: f32, width: u32, height: u32, scene: &Scene) {
        self.time += dt;
        // Earth rotates slowly around Y
        self.earth_rotation.y += 0.15 * dt;
        // Slight axial tilt wobble
        self.earth_rotation.x = 0.4; // ~23 degree tilt

        // Clouds rotate a bit faster (different wind speed)
        self.cloud_rotation.y += 0.20 * dt;
        self.cloud_rotation.x = 0.4; // Match tilt

        // Bounce movement — scale camera distance to screen size
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

        // Polygon region collision (circle around globe center)
        for region in &scene.regions {
            let verts = region.polygon().as_tuples();
            if let Some((nx, ny, penetration)) =
                circle_polygon_collision(self.pos_x, self.pos_y, visual_radius, &verts)
            {
                // Only reflect if moving INTO the region (prevents oscillation)
                let dot = self.vel_x * nx + self.vel_y * ny;
                if dot < 0.0 {
                    let (new_vx, new_vy) = reflect(self.vel_x, self.vel_y, nx, ny);
                    self.vel_x = new_vx;
                    self.vel_y = new_vy;
                }
                // Push exactly clear of overlap + small margin
                self.pos_x += nx * (penetration + 1.0);
                self.pos_y += ny * (penetration + 1.0);
                break;
            }
        }

        // --- Spawn starbursts ---
        self.next_burst_timer -= dt;
        if self.next_burst_timer <= 0.0 {
            // Pick a random star to burst
            let idx = (self.rng.next_u64() % self.stars.len() as u64) as usize;
            let star = &self.stars[idx];
            self.starbursts.push(Starburst {
                x: star.x,
                y: star.y,
                age: 0.0,
                duration: 0.4 + self.rng.next_f32() * 0.8,
                peak_bright: 200.0 + self.rng.next_f32() * 55.0,
                spike_len: 8.0 + self.rng.next_f32() * 20.0,
            });
            // Next burst in 0.8-2.5 seconds
            self.next_burst_timer = 0.8 + self.rng.next_f32() * 1.7;
        }

        // Tick starbursts, remove expired
        for burst in &mut self.starbursts {
            burst.age += dt;
        }
        self.starbursts.retain(|b| b.age < b.duration);

        // --- Spawn shooting stars ---
        self.next_shoot_timer -= dt;
        if self.next_shoot_timer <= 0.0 {
            let screen_w = width as f32;
            let screen_h = height as f32;
            // Start from a random edge-ish position in the upper half
            let start_x = self.rng.next_f32() * screen_w;
            let start_y = self.rng.next_f32() * screen_h * 0.4;
            // Angle: mostly downward-diagonal
            let angle = 0.3 + self.rng.next_f32() * 0.8;
            let speed = 300.0 + self.rng.next_f32() * 400.0;
            self.shooting_stars.push(ShootingStar {
                x: start_x,
                y: start_y,
                vx: angle.cos() * speed,
                vy: angle.sin() * speed,
                age: 0.0,
                duration: 0.4 + self.rng.next_f32() * 0.6,
                trail_len: 30.0 + self.rng.next_f32() * 60.0,
                brightness: 180.0 + self.rng.next_f32() * 75.0,
            });
            // Next shooting star in 2-6 seconds
            self.next_shoot_timer = 2.0 + self.rng.next_f32() * 4.0;
        }

        // Tick shooting stars
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

        // --- Background stars with twinkling ---
        for star in &self.stars {
            let px = (star.x * width) as i32;
            let py = (star.y * height) as i32;

            // Multi-frequency twinkle: primary oscillation + faster flutter
            let primary = (self.time * star.twinkle_speed + star.phase).sin();
            let flutter = (self.time * star.twinkle_speed * 3.7 + star.phase * 2.3).sin() * 0.3;
            let twinkle = ((primary + flutter) * star.twinkle_depth + (1.0 - star.twinkle_depth))
                .clamp(0.0, 1.0);

            let bright = (star.base_bright * twinkle).min(255.0);
            let b = bright as u8;

            // Color temperature tinting
            let (r, g, bl) = match star.color_temp {
                0 => (b.saturating_sub(20), b.saturating_sub(10), b), // blue-white
                2 => (b, b.saturating_sub(15), b.saturating_sub(40)), // warm/amber
                _ => (b, b, b),                                       // pure white
            };

            buffer.set_pixel(px, py, r, g, bl);

            // Brighter stars get a cross-shaped 3px highlight
            if bright > 180.0 {
                let dim = (bright * 0.4) as u8;
                let (dr, dg, db) = match star.color_temp {
                    0 => (dim.saturating_sub(10), dim, dim),
                    2 => (dim, dim.saturating_sub(8), dim.saturating_sub(20)),
                    _ => (dim, dim, dim),
                };
                buffer.set_pixel(px - 1, py, dr, dg, db);
                buffer.set_pixel(px + 1, py, dr, dg, db);
                buffer.set_pixel(px, py - 1, dr, dg, db);
                buffer.set_pixel(px, py + 1, dr, dg, db);
            }
        }

        // --- Starbursts ---
        for burst in &self.starbursts {
            let bx = (burst.x * width) as i32;
            let by = (burst.y * height) as i32;

            // Envelope: quick rise, slow fade
            let t = burst.age / burst.duration;
            let envelope = if t < 0.15 {
                t / 0.15 // fast attack
            } else {
                ((1.0 - t) / 0.85).max(0.0) // slow decay
            };

            let bright = (burst.peak_bright * envelope).min(255.0) as u8;
            let spike = (burst.spike_len * envelope) as i32;

            // Bright center
            buffer.blend_pixel(bx, by, bright, bright, bright, bright);

            // 4-point star spikes (horizontal + vertical)
            for d in 1..=spike {
                let falloff = (1.0 - d as f32 / spike as f32).powi(2);
                let sb = (bright as f32 * falloff) as u8;
                buffer.blend_pixel_additive(bx + d, by, sb, sb, sb);
                buffer.blend_pixel_additive(bx - d, by, sb, sb, sb);
                buffer.blend_pixel_additive(bx, by + d, sb, sb, sb);
                buffer.blend_pixel_additive(bx, by - d, sb, sb, sb);
            }

            // Diagonal spikes (shorter)
            let diag_spike = spike * 2 / 3;
            for d in 1..=diag_spike {
                let falloff = (1.0 - d as f32 / diag_spike as f32).powi(2);
                let sb = (bright as f32 * falloff * 0.6) as u8;
                buffer.blend_pixel_additive(bx + d, by + d, sb, sb, sb);
                buffer.blend_pixel_additive(bx - d, by - d, sb, sb, sb);
                buffer.blend_pixel_additive(bx + d, by - d, sb, sb, sb);
                buffer.blend_pixel_additive(bx - d, by + d, sb, sb, sb);
            }
        }

        // --- Shooting stars ---
        for star in &self.shooting_stars {
            let t = star.age / star.duration;
            // Fade in quickly, fade out slowly
            let envelope = if t < 0.1 {
                t / 0.1
            } else {
                ((1.0 - t) / 0.9).max(0.0)
            };

            let bright = (star.brightness * envelope).min(255.0);
            let speed = (star.vx * star.vx + star.vy * star.vy).sqrt();
            let dx = -star.vx / speed; // trail direction (opposite to motion)
            let dy = -star.vy / speed;
            let trail_pixels = (star.trail_len * envelope) as i32;

            // Draw trail from head backwards
            for i in 0..trail_pixels {
                let falloff = (1.0 - i as f32 / trail_pixels as f32).powi(2);
                let px = (star.x + dx * i as f32) as i32;
                let py = (star.y + dy * i as f32) as i32;
                let b = (bright * falloff) as u8;
                // Slight blue-white tint for the head, fading to warm at the tail
                let tail_t = i as f32 / trail_pixels as f32;
                let r = b;
                let g = (b as f32 * (1.0 - tail_t * 0.2)) as u8;
                let bl = (b as f32 * (1.0 - tail_t * 0.5)) as u8;
                buffer.blend_pixel_additive(px, py, r, g, bl);
                // Wider head
                if i < trail_pixels / 4 {
                    let side_b = b / 3;
                    buffer.blend_pixel_additive(
                        px + dy as i32,
                        py - dx as i32,
                        side_b,
                        side_b,
                        side_b,
                    );
                    buffer.blend_pixel_additive(
                        px - dy as i32,
                        py + dx as i32,
                        side_b,
                        side_b,
                        side_b,
                    );
                }
            }
        }

        // --- Atmosphere glow (drawn first, behind the sphere) ---
        let sphere_screen_radius = (150.0 * fov / camera_z) as i32;
        let scale = sphere_screen_radius as f32 / 150.0;
        let center_x = cx as i32;
        let center_y = cy as i32;

        for i in (0..6).rev() {
            let glow_radius = sphere_screen_radius + ((8 + i * 6) as f32 * scale) as i32;
            let alpha = 12 + i as u8 * 4;
            buffer.fill_circle_blend(center_x, center_y, glow_radius, 60, 120, 255, alpha);
        }

        // --- Earth sphere ---
        self.render_sphere(
            buffer,
            &self.earth_mesh,
            &self.earth_rotation,
            &self.earth_texture,
            fov,
            camera_z,
            cx,
            cy,
            false,
        );

        // --- Cloud layer ---
        self.render_sphere(
            buffer,
            &self.cloud_mesh,
            &self.cloud_rotation,
            &self.cloud_texture,
            fov,
            camera_z,
            cx,
            cy,
            true,
        );
    }

    fn name(&self) -> &str {
        "Earth"
    }

    fn region_color(&self) -> (u8, u8, u8) {
        (10, 20, 45)
    }
}

impl Earth {
    fn render_sphere(
        &self,
        buffer: &mut PixelBuffer,
        mesh: &Mesh,
        rotation: &Vec3,
        texture: &Texture,
        fov: f32,
        camera_z: f32,
        cx: f32,
        cy: f32,
        is_cloud: bool,
    ) {
        // Transform vertices
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

        // Collect visible faces with backface culling and depth sort
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

                // Backface cull: skip faces pointing away from camera
                // Camera looks along +Z, so faces with normal.z >= 0 face away
                if normal.z >= 0.0 {
                    return None;
                }

                let center_z = (v0.z + v1.z + v2.z) / 3.0;
                Some((i, center_z))
            })
            .collect();

        // Painter's algorithm: draw back-to-front
        visible_faces.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        for (face_idx, _depth) in visible_faces {
            let face = &mesh.faces[face_idx];

            // Project vertices to screen
            let mut projected = Vec::with_capacity(3);
            let mut all_visible = true;
            for &vi in face {
                if let Some((sx, sy)) = project(transformed[vi], fov, cx, cy) {
                    projected.push((sx, sy));
                } else {
                    all_visible = false;
                    break;
                }
            }
            if !all_visible || projected.len() < 3 {
                continue;
            }

            // Compute face center in pre-camera space (for UV mapping)
            let v0 = transformed[face[0]] - Vec3::new(0.0, 0.0, camera_z);
            let v1 = transformed[face[1]] - Vec3::new(0.0, 0.0, camera_z);
            let v2 = transformed[face[2]] - Vec3::new(0.0, 0.0, camera_z);
            let face_center = Vec3::new(
                (v0.x + v1.x + v2.x) / 3.0,
                (v0.y + v1.y + v2.y) / 3.0,
                (v0.z + v1.z + v2.z) / 3.0,
            );

            // UV from face center on sphere surface
            let (u, v) = point_to_uv(face_center);

            // Lighting: dot product with light direction (in pre-camera space)
            let world_normal = (v1 - v0).cross(&(v2 - v0)).normalize();
            let intensity = world_normal.dot(&self.light_dir).max(0.05);

            if is_cloud {
                let (cr, cg, cb, ca) = texture.sample_rgba(u, v);

                if ca < 10 {
                    continue;
                }

                let lit_r = (cr as f32 * intensity).min(255.0) as u8;
                let lit_g = (cg as f32 * intensity).min(255.0) as u8;
                let lit_b = (cb as f32 * intensity).min(255.0) as u8;

                buffer.fill_polygon_blend(&projected, lit_r, lit_g, lit_b, ca);
            } else {
                // Earth surface
                let (tr, tg, tb) = texture.sample(u, v);

                let lit_r = (tr as f32 * intensity).min(255.0) as u8;
                let lit_g = (tg as f32 * intensity).min(255.0) as u8;
                let lit_b = (tb as f32 * intensity).min(255.0) as u8;

                buffer.fill_polygon(&projected, lit_r, lit_g, lit_b);
            }
        }
    }
}
