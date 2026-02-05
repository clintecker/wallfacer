//! Glenz Vectors Effect
//!
//! Transparent, glass-like 3D objects with additive blending.
//! Named after Swedish "gläns" (glisten/glitter).

use super::Effect;
use crate::display::PixelBuffer;
use crate::math3d::{project, Mesh, Vec3};
use crate::regions::Scene;
use crate::util::hsv_to_rgb;

/// Glenz vector 3D effect with transparent rotating objects
pub struct Glenz {
    time: f32,
    mesh: Mesh,
    rotation: Vec3,
    rotation_speed: Vec3,
}

impl Glenz {
    pub fn new() -> Self {
        Self {
            time: 0.0,
            mesh: Mesh::cube(150.0),
            rotation: Vec3::zero(),
            rotation_speed: Vec3::new(0.7, 1.1, 0.5),
        }
    }

    /// Switch to a different mesh shape
    pub fn set_shape(&mut self, shape: &str) {
        self.mesh = match shape {
            "sphere" => Mesh::sphere(120.0, 1),
            "cube" => Mesh::cube(150.0),
            _ => Mesh::cube(150.0),
        };
    }
}

impl Default for Glenz {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Glenz {
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
        let fov = 400.0;
        let camera_z = 350.0;

        // Clear to black
        buffer.clear(0, 0, 0);

        // Transform vertices
        let transformed: Vec<Vec3> = self
            .mesh
            .vertices
            .iter()
            .map(|v| {
                v.rotate_x(self.rotation.x)
                    .rotate_y(self.rotation.y)
                    .rotate_z(self.rotation.z)
                    + Vec3::new(0.0, 0.0, camera_z)
            })
            .collect();

        // Sort faces by depth (painter's algorithm)
        let mut face_depths: Vec<(usize, f32)> = self
            .mesh
            .faces
            .iter()
            .enumerate()
            .map(|(i, face)| {
                let center_z =
                    (transformed[face[0]].z + transformed[face[1]].z + transformed[face[2]].z)
                        / 3.0;
                (i, center_z)
            })
            .collect();
        face_depths.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Draw faces with additive blending
        for (face_idx, _depth) in face_depths {
            let face = &self.mesh.faces[face_idx];

            // Project vertices
            let mut projected = Vec::new();
            let mut visible = true;

            for &vi in face {
                if let Some((sx, sy)) = project(transformed[vi], fov, cx, cy) {
                    projected.push((sx, sy));
                } else {
                    visible = false;
                    break;
                }
            }

            if !visible || projected.len() < 3 {
                continue;
            }

            // Calculate face color based on normal (simple lighting)
            let v0 = transformed[face[0]];
            let v1 = transformed[face[1]];
            let v2 = transformed[face[2]];
            let edge1 = v1 - v0;
            let edge2 = v2 - v0;
            let normal = edge1.cross(&edge2).normalize();

            // Light from viewer direction
            let light_intensity = normal.z.abs();

            // Hue based on face index and time
            let hue = (face_idx as f32 * 30.0 + self.time * 50.0) % 360.0;
            let (r, g, b) = hsv_to_rgb(hue, 0.6, 0.4 + light_intensity * 0.4);

            // Convert to f32 tuples for polygon fill
            let vertices: Vec<(f32, f32)> = projected.iter().map(|&(x, y)| (x, y)).collect();

            // Draw with additive blending for the glenz effect
            buffer.fill_polygon_additive(&vertices, r, g, b);
        }
    }

    fn name(&self) -> &str {
        "Glenz Vectors"
    }
}
