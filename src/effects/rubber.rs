use super::Effect;
use crate::display::PixelBuffer;
use crate::math3d::{project, Mesh, Vec3};
use crate::regions::Scene;
use crate::util::hsv_to_rgb;

pub struct Rubber {
    time: f32,
    base_vertices: Vec<Vec3>,
    mesh: Mesh,
    rotation: Vec3,
    rotation_speed: Vec3,
}

impl Rubber {
    pub fn new() -> Self {
        let mesh = Mesh::cube(160.0);
        let base_vertices = mesh.vertices.clone();

        Self {
            time: 0.0,
            base_vertices,
            mesh,
            rotation: Vec3::zero(),
            rotation_speed: Vec3::new(0.6, 0.9, 0.4),
        }
    }

    fn deform_vertices(&self) -> Vec<Vec3> {
        self.base_vertices
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let dir = v.normalize();
                let fi = i as f32;

                // Different frequencies per axis for complex organic motion
                let dx = (self.time * 2.3 + fi * 0.8).sin() * 25.0;
                let dy = (self.time * 1.7 + fi * 1.2).sin() * 25.0;
                let dz = (self.time * 3.1 + fi * 0.5).sin() * 25.0;

                // Displace along vertex normal + individual axis wobble
                let radial = (self.time * 2.0 + fi * 0.9).sin() * 20.0;

                Vec3::new(
                    v.x + dx + dir.x * radial,
                    v.y + dy + dir.y * radial,
                    v.z + dz + dir.z * radial,
                )
            })
            .collect()
    }
}

impl Default for Rubber {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Rubber {
    fn update(&mut self, dt: f32, _width: u32, _height: u32, _scene: &Scene) {
        self.time += dt;
        self.rotation.x += self.rotation_speed.x * dt;
        self.rotation.y += self.rotation_speed.y * dt;
        self.rotation.z += self.rotation_speed.z * dt;
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        let width = buffer.width() as f32;
        let height = buffer.height() as f32;
        let cx = width / 2.0;
        let cy = height / 2.0;
        let scale = width.min(height) / 480.0;
        let fov = 400.0 * scale;
        let camera_z = 400.0;

        buffer.clear(0, 0, 0);

        let deformed = self.deform_vertices();

        // Transform vertices: rotate + translate to camera distance
        let transformed: Vec<Vec3> = deformed
            .iter()
            .map(|v| {
                v.rotate_x(self.rotation.x)
                    .rotate_y(self.rotation.y)
                    .rotate_z(self.rotation.z)
                    + Vec3::new(0.0, 0.0, camera_z)
            })
            .collect();

        // Light direction
        let light_dir = Vec3::new(-0.5, -0.7, -0.5).normalize();

        // Build face data: index, depth, normal, visibility
        let mut face_data: Vec<(usize, f32)> = Vec::new();

        for (i, face) in self.mesh.faces.iter().enumerate() {
            let v0 = transformed[face[0]];
            let v1 = transformed[face[1]];
            let v2 = transformed[face[2]];

            let edge1 = v1 - v0;
            let edge2 = v2 - v0;
            let normal = edge1.cross(&edge2).normalize();

            // Backface cull: skip faces pointing away from viewer
            let to_camera = Vec3::new(0.0, 0.0, -1.0);
            if normal.dot(&to_camera) <= 0.0 {
                continue;
            }

            let center_z = (v0.z + v1.z + v2.z) / 3.0;
            face_data.push((i, center_z));
        }

        // Depth sort back-to-front
        face_data.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Render faces with Gouraud shading
        for &(face_idx, _depth) in &face_data {
            let face = &self.mesh.faces[face_idx];
            let v0 = transformed[face[0]];
            let v1 = transformed[face[1]];
            let v2 = transformed[face[2]];

            // Face normal for lighting
            let edge1 = v1 - v0;
            let edge2 = v2 - v0;
            let normal = edge1.cross(&edge2).normalize();
            let light_intensity = normal.dot(&light_dir).max(0.0);

            // Face color: hue rotates with face index + time
            let hue = (face_idx as f32 * 30.0 + self.time * 25.0).rem_euclid(360.0);
            let (base_r, base_g, base_b) = hsv_to_rgb(hue, 0.7, 0.5 + light_intensity * 0.5);

            // Per-vertex colors: vary brightness slightly per vertex for Gouraud effect
            let mut gouraud_verts: Vec<(f32, f32, u8, u8, u8)> = Vec::with_capacity(3);

            for &vert_idx in face {
                let v = transformed[vert_idx];
                if let Some((sx, sy)) = project(v, fov, cx, cy) {
                    // Vary per-vertex brightness using vertex normal approximation
                    let vert_normal = deformed[vert_idx].normalize();
                    let rotated_normal = vert_normal
                        .rotate_x(self.rotation.x)
                        .rotate_y(self.rotation.y)
                        .rotate_z(self.rotation.z);
                    let vert_light = (rotated_normal.dot(&light_dir).max(0.0) * 0.4 + 0.6).min(1.0);

                    let r = (base_r as f32 * vert_light).min(255.0) as u8;
                    let g = (base_g as f32 * vert_light).min(255.0) as u8;
                    let b = (base_b as f32 * vert_light).min(255.0) as u8;

                    gouraud_verts.push((sx, sy, r, g, b));
                }
            }

            if gouraud_verts.len() == 3 {
                buffer.fill_polygon_gouraud(&gouraud_verts);
            }
        }
    }

    fn name(&self) -> &str {
        "Rubber Cube"
    }
}
