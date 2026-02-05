use super::Effect;
use crate::display::PixelBuffer;
use crate::math3d::{lerp, project_with_depth, Mesh, Vec3};
use crate::regions::Scene;
use crate::util::hsv_to_rgb;

use std::f32::consts::PI;

pub struct VectorBalls {
    time: f32,
    rotation: Vec3,
    targets: Vec<Vec<Vec3>>,
    current_target: usize,
    morph_progress: f32,
}

fn cuboctahedron_vertices(radius: f32) -> Vec<Vec3> {
    // 12 vertices at edge midpoints of a cube
    let a = radius * std::f32::consts::FRAC_1_SQRT_2;
    vec![
        Vec3::new(a, a, 0.0),
        Vec3::new(a, -a, 0.0),
        Vec3::new(-a, a, 0.0),
        Vec3::new(-a, -a, 0.0),
        Vec3::new(a, 0.0, a),
        Vec3::new(a, 0.0, -a),
        Vec3::new(-a, 0.0, a),
        Vec3::new(-a, 0.0, -a),
        Vec3::new(0.0, a, a),
        Vec3::new(0.0, a, -a),
        Vec3::new(0.0, -a, a),
        Vec3::new(0.0, -a, -a),
    ]
}

fn ring_vertices(radius: f32) -> Vec<Vec3> {
    (0..12)
        .map(|i| {
            let angle = i as f32 * PI * 2.0 / 12.0;
            Vec3::new(radius * angle.cos(), 0.0, radius * angle.sin())
        })
        .collect()
}

fn helix_vertices(radius: f32) -> Vec<Vec3> {
    let half_height = radius * 1.2;
    (0..12)
        .map(|i| {
            let t = i as f32 / 11.0;
            let angle = t * PI * 4.0; // two full turns
            let y = -half_height + t * 2.0 * half_height;
            let r = radius * 0.6;
            Vec3::new(r * angle.cos(), y, r * angle.sin())
        })
        .collect()
}

impl VectorBalls {
    pub fn new() -> Self {
        let ico = Mesh::sphere(1.0, 0);
        let radius = 150.0;
        let ico_verts: Vec<Vec3> = ico
            .vertices
            .iter()
            .map(|v| v.normalize() * radius)
            .collect();

        Self {
            time: 0.0,
            rotation: Vec3::zero(),
            targets: vec![
                ico_verts,
                cuboctahedron_vertices(radius),
                ring_vertices(radius),
                helix_vertices(radius),
            ],
            current_target: 0,
            morph_progress: 0.0,
        }
    }
}

impl Default for VectorBalls {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for VectorBalls {
    fn update(&mut self, dt: f32, _width: u32, _height: u32, _scene: &Scene) {
        self.time += dt;
        self.rotation.x += 0.4 * dt;
        self.rotation.y += 0.7 * dt;

        // Advance morph: ~4 seconds per shape
        self.morph_progress += dt / 4.0;
        if self.morph_progress >= 1.0 {
            self.morph_progress -= 1.0;
            self.current_target = (self.current_target + 1) % self.targets.len();
        }
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        let width = buffer.width() as f32;
        let height = buffer.height() as f32;
        let cx = width / 2.0;
        let cy = height / 2.0;
        let scale = width.min(height) / 480.0;
        let fov = 400.0 * scale;
        let camera_z = 450.0;
        let max_z = 900.0;
        let base_ball_radius = 38.0;

        buffer.clear(0, 0, 0);

        // Smooth ease-in-out morph factor
        let t = (1.0 - (PI * self.morph_progress).cos()) / 2.0;

        let src = &self.targets[self.current_target];
        let dst = &self.targets[(self.current_target + 1) % self.targets.len()];

        // Interpolate and transform vertices
        let mut balls: Vec<(f32, f32, f32, f32)> = Vec::with_capacity(12); // (sx, sy, radius, z)
        for i in 0..12 {
            let pos = lerp(src[i], dst[i], t);
            let transformed = pos.rotate_x(self.rotation.x).rotate_y(self.rotation.y)
                + Vec3::new(0.0, 0.0, camera_z);

            if let Some((sx, sy, proximity)) = project_with_depth(transformed, fov, cx, cy, max_z) {
                let r = base_ball_radius * proximity * scale;
                if r > 0.5 {
                    balls.push((sx, sy, r, transformed.z));
                }
            }
        }

        // Depth sort back-to-front (largest z first)
        balls.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap());

        // Light direction (upper-left)
        let light_dir = Vec3::new(-0.5, -0.7, -0.5).normalize();

        for &(sx, sy, radius, _z) in &balls {
            let x_start = ((sx - radius) as i32).max(0);
            let x_end = ((sx + radius) as i32).min(buffer.width() as i32 - 1);
            let y_start = ((sy - radius) as i32).max(0);
            let y_end = ((sy + radius) as i32).min(buffer.height() as i32 - 1);
            let r_sq = radius * radius;

            for py in y_start..=y_end {
                let dy = py as f32 - sy;
                let dy_sq = dy * dy;
                for px in x_start..=x_end {
                    let dx = px as f32 - sx;
                    let dist_sq = dx * dx + dy_sq;
                    if dist_sq > r_sq {
                        continue;
                    }

                    // Sphere surface normal
                    let nx = dx / radius;
                    let ny = dy / radius;
                    let nz_sq = 1.0 - nx * nx - ny * ny;
                    if nz_sq <= 0.0 {
                        continue;
                    }
                    let nz = nz_sq.sqrt();
                    let normal = Vec3::new(nx, ny, -nz);

                    // Diffuse lighting
                    let diffuse = normal.dot(&light_dir).max(0.0);

                    // Specular (reflection dot view)
                    let reflect = Vec3::new(
                        2.0 * normal.dot(&light_dir) * nx - light_dir.x,
                        2.0 * normal.dot(&light_dir) * ny - light_dir.y,
                        2.0 * normal.dot(&light_dir) * (-nz) - light_dir.z,
                    );
                    let view = Vec3::new(0.0, 0.0, -1.0);
                    let spec = reflect.dot(&view).max(0.0).powf(32.0);

                    // Environment color from normal direction
                    let hue = (nx.atan2(ny).to_degrees() + self.time * 30.0).rem_euclid(360.0);
                    let (er, eg, eb) = hsv_to_rgb(hue, 0.7, 0.9);

                    let ambient = 0.15;
                    let r = (er as f32 * (ambient + diffuse) + 255.0 * spec).min(255.0) as u8;
                    let g = (eg as f32 * (ambient + diffuse) + 255.0 * spec).min(255.0) as u8;
                    let b = (eb as f32 * (ambient + diffuse) + 255.0 * spec).min(255.0) as u8;

                    buffer.set_pixel(px, py, r, g, b);
                }
            }
        }
    }

    fn name(&self) -> &str {
        "Vector Balls"
    }
}
