//! 3D Math Utilities for Demoscene Effects
//!
//! Provides basic 3D vector operations, rotations, and perspective projection.

use std::collections::HashMap;
use std::ops::{Add, Mul, Neg, Sub};

/// 3D Vector
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    #[inline]
    pub const fn zero() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    #[inline]
    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    #[inline]
    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > 0.0 {
            Self {
                x: self.x / len,
                y: self.y / len,
                z: self.z / len,
            }
        } else {
            *self
        }
    }

    #[inline]
    pub fn dot(&self, other: &Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    #[inline]
    pub fn cross(&self, other: &Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    /// Approximate equality check for floating point comparison
    #[inline]
    pub fn approx_eq(&self, other: &Self, epsilon: f32) -> bool {
        (self.x - other.x).abs() < epsilon
            && (self.y - other.y).abs() < epsilon
            && (self.z - other.z).abs() < epsilon
    }

    /// Rotate around X axis
    #[inline]
    pub fn rotate_x(&self, angle: f32) -> Self {
        let (sin, cos) = angle.sin_cos();
        Self {
            x: self.x,
            y: self.y * cos - self.z * sin,
            z: self.y * sin + self.z * cos,
        }
    }

    /// Rotate around Y axis
    #[inline]
    pub fn rotate_y(&self, angle: f32) -> Self {
        let (sin, cos) = angle.sin_cos();
        Self {
            x: self.x * cos + self.z * sin,
            y: self.y,
            z: -self.x * sin + self.z * cos,
        }
    }

    /// Rotate around Z axis
    #[inline]
    pub fn rotate_z(&self, angle: f32) -> Self {
        let (sin, cos) = angle.sin_cos();
        Self {
            x: self.x * cos - self.y * sin,
            y: self.x * sin + self.y * cos,
            z: self.z,
        }
    }

    /// Apply all three rotations (commonly needed for 3D objects)
    #[inline]
    pub fn rotate_xyz(&self, rx: f32, ry: f32, rz: f32) -> Self {
        self.rotate_x(rx).rotate_y(ry).rotate_z(rz)
    }
}

impl Add for Vec3 {
    type Output = Self;
    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl Sub for Vec3 {
    type Output = Self;
    #[inline]
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl Neg for Vec3 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;
    #[inline]
    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }
}

impl Mul<Vec3> for f32 {
    type Output = Vec3;
    #[inline]
    fn mul(self, v: Vec3) -> Vec3 {
        v * self
    }
}

/// 2D Vector (for screen coordinates and 2D effects)
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline]
    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    #[inline]
    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len > 0.0 {
            Self {
                x: self.x / len,
                y: self.y / len,
            }
        } else {
            *self
        }
    }

    /// Approximate equality check for floating point comparison
    #[inline]
    pub fn approx_eq(&self, other: &Self, epsilon: f32) -> bool {
        (self.x - other.x).abs() < epsilon && (self.y - other.y).abs() < epsilon
    }
}

impl Add for Vec2 {
    type Output = Self;
    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Sub for Vec2 {
    type Output = Self;
    #[inline]
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl Neg for Vec2 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
        }
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;
    #[inline]
    fn mul(self, scalar: f32) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }
}

impl Mul<Vec2> for f32 {
    type Output = Vec2;
    #[inline]
    fn mul(self, v: Vec2) -> Vec2 {
        v * self
    }
}

/// Project a 3D point to 2D screen coordinates
///
/// - `point`: The 3D point to project
/// - `fov`: Field of view (distance from eye to projection plane)
/// - `cx`, `cy`: Screen center coordinates
///
/// Returns (screen_x, screen_y) or None if point is behind camera
#[inline]
pub fn project(point: Vec3, fov: f32, cx: f32, cy: f32) -> Option<(f32, f32)> {
    if point.z <= 0.0 {
        return None;
    }
    let scale = fov / point.z;
    Some((cx + point.x * scale, cy + point.y * scale))
}

/// Project a 3D point, returning proximity factor for brightness/size scaling
///
/// - `point`: The 3D point to project
/// - `fov`: Field of view (distance from eye to projection plane)
/// - `cx`, `cy`: Screen center coordinates
/// - `max_z`: Maximum Z depth for normalization
///
/// Returns (screen_x, screen_y, proximity) where proximity is 0.0-1.0
/// (1.0 = closest to camera, 0.0 = at max_z distance)
#[inline]
pub fn project_with_depth(
    point: Vec3,
    fov: f32,
    cx: f32,
    cy: f32,
    max_z: f32,
) -> Option<(f32, f32, f32)> {
    if point.z <= 0.0 {
        return None;
    }
    let scale = fov / point.z;
    let proximity = 1.0 - (point.z / max_z).min(1.0);
    Some((cx + point.x * scale, cy + point.y * scale, proximity))
}

/// Linear interpolation between two Vec3 points
///
/// Note: `t` is not clamped to [0, 1], allowing extrapolation.
/// Use t=0.0 for `a`, t=1.0 for `b`, values outside for extrapolation.
#[inline]
pub fn lerp(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    Vec3 {
        x: a.x + (b.x - a.x) * t,
        y: a.y + (b.y - a.y) * t,
        z: a.z + (b.z - a.z) * t,
    }
}

// ============================================================================
// Mesh
// ============================================================================

/// A 3D mesh consisting of vertices and triangle faces
#[derive(Clone, Default)]
pub struct Mesh {
    pub vertices: Vec<Vec3>,
    pub faces: Vec<[usize; 3]>,
}

impl Mesh {
    /// Create an empty mesh
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            faces: Vec::new(),
        }
    }

    /// Create a unit cube centered at origin
    pub fn cube(size: f32) -> Self {
        let h = size / 2.0;
        let vertices = vec![
            Vec3::new(-h, -h, -h), // 0: back-bottom-left
            Vec3::new(h, -h, -h),  // 1: back-bottom-right
            Vec3::new(h, h, -h),   // 2: back-top-right
            Vec3::new(-h, h, -h),  // 3: back-top-left
            Vec3::new(-h, -h, h),  // 4: front-bottom-left
            Vec3::new(h, -h, h),   // 5: front-bottom-right
            Vec3::new(h, h, h),    // 6: front-top-right
            Vec3::new(-h, h, h),   // 7: front-top-left
        ];

        // Two triangles per face, 6 faces
        let faces = vec![
            // Front
            [4, 5, 6],
            [4, 6, 7],
            // Back
            [1, 0, 3],
            [1, 3, 2],
            // Left
            [0, 4, 7],
            [0, 7, 3],
            // Right
            [5, 1, 2],
            [5, 2, 6],
            // Top
            [7, 6, 2],
            [7, 2, 3],
            // Bottom
            [0, 1, 5],
            [0, 5, 4],
        ];

        Self { vertices, faces }
    }

    /// Create a sphere using icosahedron subdivision
    pub fn sphere(radius: f32, subdivisions: u32) -> Self {
        // Helper function for subdivision - gets or creates midpoint vertex
        fn get_midpoint(
            vertices: &mut Vec<Vec3>,
            cache: &mut HashMap<(usize, usize), usize>,
            i0: usize,
            i1: usize,
            radius: f32,
        ) -> usize {
            let key = if i0 < i1 { (i0, i1) } else { (i1, i0) };

            if let Some(&idx) = cache.get(&key) {
                return idx;
            }

            let v0 = vertices[i0];
            let v1 = vertices[i1];
            let mid = Vec3::new(
                (v0.x + v1.x) / 2.0,
                (v0.y + v1.y) / 2.0,
                (v0.z + v1.z) / 2.0,
            )
            .normalize()
                * radius;

            let idx = vertices.len();
            vertices.push(mid);
            cache.insert(key, idx);
            idx
        }

        // Start with icosahedron
        let t = (1.0 + 5.0_f32.sqrt()) / 2.0;
        let mut vertices = vec![
            Vec3::new(-1.0, t, 0.0).normalize() * radius,
            Vec3::new(1.0, t, 0.0).normalize() * radius,
            Vec3::new(-1.0, -t, 0.0).normalize() * radius,
            Vec3::new(1.0, -t, 0.0).normalize() * radius,
            Vec3::new(0.0, -1.0, t).normalize() * radius,
            Vec3::new(0.0, 1.0, t).normalize() * radius,
            Vec3::new(0.0, -1.0, -t).normalize() * radius,
            Vec3::new(0.0, 1.0, -t).normalize() * radius,
            Vec3::new(t, 0.0, -1.0).normalize() * radius,
            Vec3::new(t, 0.0, 1.0).normalize() * radius,
            Vec3::new(-t, 0.0, -1.0).normalize() * radius,
            Vec3::new(-t, 0.0, 1.0).normalize() * radius,
        ];

        let mut faces = vec![
            [0, 11, 5],
            [0, 5, 1],
            [0, 1, 7],
            [0, 7, 10],
            [0, 10, 11],
            [1, 5, 9],
            [5, 11, 4],
            [11, 10, 2],
            [10, 7, 6],
            [7, 1, 8],
            [3, 9, 4],
            [3, 4, 2],
            [3, 2, 6],
            [3, 6, 8],
            [3, 8, 9],
            [4, 9, 5],
            [2, 4, 11],
            [6, 2, 10],
            [8, 6, 7],
            [9, 8, 1],
        ];

        // Subdivide
        for _ in 0..subdivisions {
            let mut new_faces = Vec::new();
            let mut midpoint_cache: HashMap<(usize, usize), usize> = HashMap::new();

            for face in &faces {
                let v0 = face[0];
                let v1 = face[1];
                let v2 = face[2];

                let a = get_midpoint(&mut vertices, &mut midpoint_cache, v0, v1, radius);
                let b = get_midpoint(&mut vertices, &mut midpoint_cache, v1, v2, radius);
                let c = get_midpoint(&mut vertices, &mut midpoint_cache, v2, v0, radius);

                new_faces.push([v0, a, c]);
                new_faces.push([v1, b, a]);
                new_faces.push([v2, c, b]);
                new_faces.push([a, b, c]);
            }

            faces = new_faces;
        }

        Self { vertices, faces }
    }

    /// Rotate all vertices
    pub fn rotate(&mut self, rx: f32, ry: f32, rz: f32) {
        for v in &mut self.vertices {
            *v = v.rotate_xyz(rx, ry, rz);
        }
    }

    /// Scale all vertices
    pub fn scale(&mut self, factor: f32) {
        for v in &mut self.vertices {
            *v = *v * factor;
        }
    }

    /// Translate all vertices
    pub fn translate(&mut self, offset: Vec3) {
        for v in &mut self.vertices {
            *v = *v + offset;
        }
    }

    /// Get face center for depth sorting
    pub fn face_center(&self, face_idx: usize) -> Vec3 {
        let face = &self.faces[face_idx];
        let v0 = self.vertices[face[0]];
        let v1 = self.vertices[face[1]];
        let v2 = self.vertices[face[2]];
        Vec3::new(
            (v0.x + v1.x + v2.x) / 3.0,
            (v0.y + v1.y + v2.y) / 3.0,
            (v0.z + v1.z + v2.z) / 3.0,
        )
    }

    /// Get face normal (for backface culling and lighting)
    pub fn face_normal(&self, face_idx: usize) -> Vec3 {
        let face = &self.faces[face_idx];
        let v0 = self.vertices[face[0]];
        let v1 = self.vertices[face[1]];
        let v2 = self.vertices[face[2]];
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        edge1.cross(&edge2).normalize()
    }
}
