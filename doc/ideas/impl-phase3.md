# Phase 3 Implementation Plan

Advanced systems that enable the remaining demoscene effects.

## Overview

Phase 3 introduces larger subsystems: textures, buffer operations, meshes, and particles.

---

## 11. Texture System

**Purpose:** Enable rotozoomer, tunnel, and textured 3D effects.

**Location:** New file `src/texture.rs`

**Implementation:**

```rust
//! Texture System for Demoscene Effects
//!
//! Provides texture storage, sampling, and procedural generation.

/// A texture stored as RGBA pixels
#[derive(Clone)]
pub struct Texture {
    width: u32,
    height: u32,
    pixels: Vec<u8>, // RGBA format, 4 bytes per pixel
}

impl Texture {
    /// Create a new empty texture
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; (width * height * 4) as usize],
        }
    }

    /// Create texture from raw RGBA data
    pub fn from_rgba(width: u32, height: u32, data: Vec<u8>) -> Option<Self> {
        if data.len() == (width * height * 4) as usize {
            Some(Self {
                width,
                height,
                pixels: data,
            })
        } else {
            None
        }
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Set a pixel in the texture
    #[inline]
    pub fn set_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8, a: u8) {
        if x < self.width && y < self.height {
            let idx = ((y * self.width + x) * 4) as usize;
            self.pixels[idx] = r;
            self.pixels[idx + 1] = g;
            self.pixels[idx + 2] = b;
            self.pixels[idx + 3] = a;
        }
    }

    /// Sample texture with UV coordinates (0.0 to 1.0, wrapping)
    /// Returns (r, g, b) - alpha is discarded for simplicity
    #[inline]
    pub fn sample(&self, u: f32, v: f32) -> (u8, u8, u8) {
        // Wrap UV coordinates
        let u = u.rem_euclid(1.0);
        let v = v.rem_euclid(1.0);

        let x = (u * self.width as f32) as u32 % self.width;
        let y = (v * self.height as f32) as u32 % self.height;

        let idx = ((y * self.width + x) * 4) as usize;
        (self.pixels[idx], self.pixels[idx + 1], self.pixels[idx + 2])
    }

    /// Sample with bilinear interpolation for smoother results
    pub fn sample_bilinear(&self, u: f32, v: f32) -> (u8, u8, u8) {
        let u = u.rem_euclid(1.0) * self.width as f32;
        let v = v.rem_euclid(1.0) * self.height as f32;

        let x0 = u.floor() as u32 % self.width;
        let y0 = v.floor() as u32 % self.height;
        let x1 = (x0 + 1) % self.width;
        let y1 = (y0 + 1) % self.height;

        let fx = u.fract();
        let fy = v.fract();

        // Sample 4 corners
        let c00 = self.get_pixel_internal(x0, y0);
        let c10 = self.get_pixel_internal(x1, y0);
        let c01 = self.get_pixel_internal(x0, y1);
        let c11 = self.get_pixel_internal(x1, y1);

        // Bilinear interpolation with clamping for numerical stability
        let lerp = |a: u8, b: u8, t: f32| -> u8 {
            let result = a as f32 + (b as f32 - a as f32) * t;
            result.clamp(0.0, 255.0) as u8
        };

        let r = lerp(lerp(c00.0, c10.0, fx), lerp(c01.0, c11.0, fx), fy);
        let g = lerp(lerp(c00.1, c10.1, fx), lerp(c01.1, c11.1, fx), fy);
        let b = lerp(lerp(c00.2, c10.2, fx), lerp(c01.2, c11.2, fx), fy);

        (r, g, b)
    }

    #[inline]
    fn get_pixel_internal(&self, x: u32, y: u32) -> (u8, u8, u8) {
        let idx = ((y * self.width + x) * 4) as usize;
        (self.pixels[idx], self.pixels[idx + 1], self.pixels[idx + 2])
    }
}

// ============================================================================
// Procedural Texture Generators
// ============================================================================

impl Texture {
    /// Generate a checkerboard pattern
    pub fn checkerboard(size: u32, tile_size: u32, c1: (u8, u8, u8), c2: (u8, u8, u8)) -> Self {
        let mut tex = Self::new(size, size);
        for y in 0..size {
            for x in 0..size {
                let checker = ((x / tile_size) + (y / tile_size)) % 2 == 0;
                let (r, g, b) = if checker { c1 } else { c2 };
                tex.set_pixel(x, y, r, g, b, 255);
            }
        }
        tex
    }

    /// Generate an XOR pattern (classic demoscene texture)
    pub fn xor_pattern(size: u32) -> Self {
        let mut tex = Self::new(size, size);
        for y in 0..size {
            for x in 0..size {
                let v = (x ^ y) as u8;
                tex.set_pixel(x, y, v, v, v, 255);
            }
        }
        tex
    }

    /// Generate a plasma texture using sine waves
    pub fn plasma(size: u32, palette: &[(u8, u8, u8)]) -> Self {
        let mut tex = Self::new(size, size);
        let scale = std::f32::consts::TAU / size as f32;

        for y in 0..size {
            for x in 0..size {
                let fx = x as f32 * scale;
                let fy = y as f32 * scale;

                // Sum of sines
                let v1 = (fx * 2.0).sin();
                let v2 = (fy * 3.0).sin();
                let v3 = ((fx + fy) * 1.5).sin();
                let v4 = ((fx * fx + fy * fy).sqrt() * 2.0).sin();

                let sum = (v1 + v2 + v3 + v4 + 4.0) / 8.0; // Normalize to 0-1
                let idx = (sum * (palette.len() - 1) as f32) as usize;
                let (r, g, b) = palette[idx.min(palette.len() - 1)];

                tex.set_pixel(x, y, r, g, b, 255);
            }
        }
        tex
    }
}
```

---

## 12. Buffer Operations

**Purpose:** Enable feedback effects, transitions, and double buffering.

**Location:** Add to `src/display/pixel_buffer.rs`

**Implementation:**

```rust
impl PixelBuffer {
    /// Copy another buffer onto this one at position (x, y)
    pub fn blit(&mut self, src: &PixelBuffer, x: i32, y: i32) {
        let src_w = src.width() as i32;
        let src_h = src.height() as i32;

        for sy in 0..src_h {
            let dy = y + sy;
            if dy < 0 || dy >= HEIGHT as i32 {
                continue;
            }

            for sx in 0..src_w {
                let dx = x + sx;
                if dx < 0 || dx >= WIDTH as i32 {
                    continue;
                }

                if let Some((r, g, b)) = src.get_pixel(sx, sy) {
                    self.set_pixel(dx, dy, r, g, b);
                }
            }
        }
    }

    /// Blit with alpha blending
    pub fn blit_blend(&mut self, src: &PixelBuffer, x: i32, y: i32, alpha: u8) {
        let src_w = src.width() as i32;
        let src_h = src.height() as i32;

        for sy in 0..src_h {
            let dy = y + sy;
            if dy < 0 || dy >= HEIGHT as i32 {
                continue;
            }

            for sx in 0..src_w {
                let dx = x + sx;
                if dx < 0 || dx >= WIDTH as i32 {
                    continue;
                }

                if let Some((r, g, b)) = src.get_pixel(sx, sy) {
                    self.blend_pixel(dx, dy, r, g, b, alpha);
                }
            }
        }
    }

    /// Fade the entire buffer (multiply all colors by factor)
    /// factor: 0.0 = black, 1.0 = unchanged
    pub fn fade(&mut self, factor: f32) {
        let factor = factor.clamp(0.0, 1.0);
        let factor_u8 = (factor * 255.0) as u16;

        for chunk in self.pixels.chunks_exact_mut(4) {
            // Skip alpha (chunk[0]), fade RGB
            chunk[1] = ((chunk[1] as u16 * factor_u8) / 255) as u8;
            chunk[2] = ((chunk[2] as u16 * factor_u8) / 255) as u8;
            chunk[3] = ((chunk[3] as u16 * factor_u8) / 255) as u8;
        }
    }

    /// Copy contents from another buffer
    pub fn copy_from(&mut self, src: &PixelBuffer) {
        self.pixels.copy_from_slice(src.as_bytes());
    }

    /// Scroll buffer contents from source to destination (for feedback effects)
    /// Positive dx scrolls right, positive dy scrolls down
    /// Uses double-buffering approach to avoid 1.2MB allocation per call
    pub fn scroll_from(&mut self, src: &PixelBuffer, dx: i32, dy: i32) {
        // Clear destination
        self.clear(0, 0, 0);

        // Copy with offset using row-based approach
        for y in 0..HEIGHT as i32 {
            let src_y = y - dy;
            if src_y < 0 || src_y >= HEIGHT as i32 {
                continue;
            }

            // Calculate valid x range for this row
            let x_start = 0.max(-dx);
            let x_end = (WIDTH as i32).min(WIDTH as i32 - dx);
            if x_start >= x_end {
                continue;
            }

            let src_x_start = (x_start - dx) as u32;
            let src_row_start = pixel_index(src_x_start, src_y as u32);
            let dst_row_start = pixel_index(x_start as u32, y as u32);
            let row_bytes = ((x_end - x_start) * 4) as usize;

            self.pixels[dst_row_start..dst_row_start + row_bytes]
                .copy_from_slice(&src.pixels[src_row_start..src_row_start + row_bytes]);
        }
    }

    /// Scroll in place (convenience wrapper, requires temp buffer)
    /// For better performance in hot loops, use scroll_from with double-buffering
    pub fn scroll(&mut self, dx: i32, dy: i32) {
        let old_pixels = self.pixels.clone();
        self.clear(0, 0, 0);

        for y in 0..HEIGHT as i32 {
            let src_y = y - dy;
            if src_y < 0 || src_y >= HEIGHT as i32 {
                continue;
            }

            let x_start = 0.max(-dx);
            let x_end = (WIDTH as i32).min(WIDTH as i32 - dx);
            if x_start >= x_end {
                continue;
            }

            let src_x_start = (x_start - dx) as u32;
            let src_row_start = pixel_index(src_x_start, src_y as u32);
            let dst_row_start = pixel_index(x_start as u32, y as u32);
            let row_bytes = ((x_end - x_start) * 4) as usize;

            self.pixels[dst_row_start..dst_row_start + row_bytes]
                .copy_from_slice(&old_pixels[src_row_start..src_row_start + row_bytes]);
        }
    }

    /// Scale and blit (for zoom feedback effects)
    /// scale > 1.0 zooms in, scale < 1.0 zooms out
    pub fn blit_scaled_centered(&mut self, src: &PixelBuffer, scale: f32) {
        let cx = (WIDTH / 2) as f32;
        let cy = (HEIGHT / 2) as f32;

        for y in 0..HEIGHT as i32 {
            for x in 0..WIDTH as i32 {
                // Map destination to source coordinates
                let sx = ((x as f32 - cx) / scale + cx) as i32;
                let sy = ((y as f32 - cy) / scale + cy) as i32;

                if let Some((r, g, b)) = src.get_pixel(sx, sy) {
                    self.set_pixel(x, y, r, g, b);
                }
            }
        }
    }
}
```

---

## 13. `fill_polygon_additive(vertices, r, g, b)`

**Purpose:** Polygon fill with additive blending for glenz vectors.

**Location:** `src/display/pixel_buffer.rs`

**Implementation:**

```rust
/// Fill a polygon with additive blending (for glenz effect)
pub fn fill_polygon_additive(&mut self, vertices: &[(f32, f32)], r: u8, g: u8, b: u8) {
    if vertices.len() < 3 {
        return;
    }

    // Find bounding box
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;
    for (_, y) in vertices {
        min_y = min_y.min(*y);
        max_y = max_y.max(*y);
    }

    let min_y = (min_y as i32).max(0);
    let max_y = (max_y as i32).min(HEIGHT as i32 - 1);

    // Preallocate intersections buffer (reused per scanline)
    let mut intersections = Vec::with_capacity(vertices.len());

    // Scanline fill with additive blending
    let n = vertices.len();
    for y in min_y..=max_y {
        intersections.clear(); // Reuse allocation
        let yf = y as f32 + 0.5;

        for i in 0..n {
            let (x1, y1) = vertices[i];
            let (x2, y2) = vertices[(i + 1) % n];

            if (y1 <= yf && y2 > yf) || (y2 <= yf && y1 > yf) {
                let x = x1 + (yf - y1) / (y2 - y1) * (x2 - x1);
                intersections.push(x as i32);
            }
        }

        intersections.sort_unstable();
        for pair in intersections.chunks_exact(2) {
            let x_start = pair[0].max(0);
            let x_end = pair[1].min(WIDTH as i32 - 1);
            for x in x_start..=x_end {
                self.blend_pixel_additive(x, y, r, g, b);
            }
        }
    }
}
```

---

## 14. Mesh Data Structure

**Purpose:** Store and manipulate 3D objects for vector effects.

**Location:** Add to `src/math3d.rs`

**Implementation:**

```rust
use std::collections::HashMap;

/// A 3D mesh consisting of vertices and triangle faces
#[derive(Clone, Default)]
pub struct Mesh {
    pub vertices: Vec<Vec3>,
    pub faces: Vec<[usize; 3]>, // Indices into vertices
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
            Vec3::new( h, -h, -h), // 1: back-bottom-right
            Vec3::new( h,  h, -h), // 2: back-top-right
            Vec3::new(-h,  h, -h), // 3: back-top-left
            Vec3::new(-h, -h,  h), // 4: front-bottom-left
            Vec3::new( h, -h,  h), // 5: front-bottom-right
            Vec3::new( h,  h,  h), // 6: front-top-right
            Vec3::new(-h,  h,  h), // 7: front-top-left
        ];

        // Two triangles per face, 6 faces
        let faces = vec![
            // Front
            [4, 5, 6], [4, 6, 7],
            // Back
            [1, 0, 3], [1, 3, 2],
            // Left
            [0, 4, 7], [0, 7, 3],
            // Right
            [5, 1, 2], [5, 2, 6],
            // Top
            [7, 6, 2], [7, 2, 3],
            // Bottom
            [0, 1, 5], [0, 5, 4],
        ];

        Self { vertices, faces }
    }

    /// Create a simple sphere using icosahedron subdivision
    pub fn sphere(radius: f32, subdivisions: u32) -> Self {
        // Start with icosahedron
        let t = (1.0 + 5.0_f32.sqrt()) / 2.0;
        let mut vertices = vec![
            Vec3::new(-1.0,  t, 0.0).normalize() * radius,
            Vec3::new( 1.0,  t, 0.0).normalize() * radius,
            Vec3::new(-1.0, -t, 0.0).normalize() * radius,
            Vec3::new( 1.0, -t, 0.0).normalize() * radius,
            Vec3::new(0.0, -1.0,  t).normalize() * radius,
            Vec3::new(0.0,  1.0,  t).normalize() * radius,
            Vec3::new(0.0, -1.0, -t).normalize() * radius,
            Vec3::new(0.0,  1.0, -t).normalize() * radius,
            Vec3::new( t, 0.0, -1.0).normalize() * radius,
            Vec3::new( t, 0.0,  1.0).normalize() * radius,
            Vec3::new(-t, 0.0, -1.0).normalize() * radius,
            Vec3::new(-t, 0.0,  1.0).normalize() * radius,
        ];

        let mut faces = vec![
            [0, 11, 5], [0, 5, 1], [0, 1, 7], [0, 7, 10], [0, 10, 11],
            [1, 5, 9], [5, 11, 4], [11, 10, 2], [10, 7, 6], [7, 1, 8],
            [3, 9, 4], [3, 4, 2], [3, 2, 6], [3, 6, 8], [3, 8, 9],
            [4, 9, 5], [2, 4, 11], [6, 2, 10], [8, 6, 7], [9, 8, 1],
        ];

        // Subdivide
        for _ in 0..subdivisions {
            let mut new_faces = Vec::new();
            let mut midpoint_cache = HashMap::new();

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
}

/// Helper for sphere subdivision - gets or creates midpoint vertex
/// (module-private, only used by Mesh::sphere)
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
    ).normalize() * radius;

    let idx = vertices.len();
    vertices.push(mid);
    cache.insert(key, idx);
    idx
}
```

---

## 15. Particle System

**Purpose:** Particle streams, explosions, sparks, and ambient effects.

**Location:** New file `src/particles.rs`

**Implementation:**

```rust
//! Particle System for Demoscene Effects

use crate::display::PixelBuffer;
use crate::math3d::Vec2;

/// A single particle
#[derive(Clone)]
pub struct Particle {
    pub pos: Vec2,
    pub vel: Vec2,
    pub life: f32,      // Remaining lifetime (0 = dead)
    pub max_life: f32,  // Initial lifetime
    pub color: (u8, u8, u8),
    pub size: f32,
}

impl Particle {
    pub fn new(pos: Vec2, vel: Vec2, life: f32, color: (u8, u8, u8)) -> Self {
        Self {
            pos,
            vel,
            life,
            max_life: life,
            color,
            size: 1.0,
        }
    }

    /// Get normalized age (0 = just born, 1 = about to die)
    #[inline]
    pub fn age(&self) -> f32 {
        1.0 - (self.life / self.max_life)
    }

    /// Check if particle is still alive
    #[inline]
    pub fn is_alive(&self) -> bool {
        self.life > 0.0
    }
}

/// Particle system manager
pub struct ParticleSystem {
    particles: Vec<Particle>,
    gravity: Vec2,
    drag: f32,
}

impl ParticleSystem {
    pub fn new() -> Self {
        Self {
            particles: Vec::new(),
            gravity: Vec2::new(0.0, 0.0),
            drag: 0.0,
        }
    }

    /// Set gravity (applied each frame)
    pub fn with_gravity(mut self, gravity: Vec2) -> Self {
        self.gravity = gravity;
        self
    }

    /// Set drag (velocity multiplier each frame, 0.99 = slight drag)
    pub fn with_drag(mut self, drag: f32) -> Self {
        self.drag = drag;
        self
    }

    /// Emit a single particle
    pub fn emit(&mut self, particle: Particle) {
        self.particles.push(particle);
    }

    /// Emit particles in a burst from a point
    pub fn emit_burst(
        &mut self,
        pos: Vec2,
        count: usize,
        speed: f32,
        life: f32,
        color: (u8, u8, u8),
    ) {
        use std::f32::consts::TAU;

        for i in 0..count {
            let angle = (i as f32 / count as f32) * TAU;
            let vel = Vec2::new(angle.cos() * speed, angle.sin() * speed);
            self.emit(Particle::new(pos, vel, life, color));
        }
    }

    /// Emit particles in a random burst
    pub fn emit_burst_random(
        &mut self,
        pos: Vec2,
        count: usize,
        speed: f32,
        spread: f32,
        life: f32,
        color: (u8, u8, u8),
        rng: &mut impl FnMut() -> f32, // Returns 0.0-1.0
    ) {
        use std::f32::consts::TAU;

        for _ in 0..count {
            let angle = rng() * TAU;
            let s = speed * (1.0 - spread + spread * rng());
            let vel = Vec2::new(angle.cos() * s, angle.sin() * s);
            let l = life * (0.5 + 0.5 * rng());
            self.emit(Particle::new(pos, vel, l, color));
        }
    }

    /// Update all particles
    pub fn update(&mut self, dt: f32) {
        for p in &mut self.particles {
            if !p.is_alive() {
                continue;
            }

            // Apply velocity (using Vec2 operators from Phase 2)
            p.pos = p.pos + p.vel * dt;

            // Apply gravity
            p.vel = p.vel + self.gravity * dt;

            // Apply drag (clamped to prevent velocity reversal on lag spikes)
            if self.drag > 0.0 {
                let drag_factor = (1.0 - self.drag * dt).max(0.0);
                p.vel = p.vel * drag_factor;
            }

            // Age
            p.life -= dt;
        }

        // Remove dead particles
        self.particles.retain(|p| p.is_alive());
    }

    /// Render all particles as single pixels
    pub fn render(&self, buffer: &mut PixelBuffer) {
        for p in &self.particles {
            let alpha = (p.life / p.max_life * 255.0) as u8;
            buffer.blend_pixel(
                p.pos.x as i32,
                p.pos.y as i32,
                p.color.0,
                p.color.1,
                p.color.2,
                alpha,
            );
        }
    }

    /// Render particles as circles (for larger effects)
    pub fn render_circles(&self, buffer: &mut PixelBuffer) {
        for p in &self.particles {
            let alpha = (p.life / p.max_life * 255.0) as u8;
            let radius = (p.size * (1.0 - p.age() * 0.5)) as i32;
            if radius > 0 {
                buffer.fill_circle_blend(
                    p.pos.x as i32,
                    p.pos.y as i32,
                    radius,
                    p.color.0,
                    p.color.1,
                    p.color.2,
                    alpha,
                );
            }
        }
    }

    /// Render with additive blending (for sparks/glow)
    pub fn render_additive(&self, buffer: &mut PixelBuffer) {
        for p in &self.particles {
            let intensity = p.life / p.max_life;
            buffer.blend_pixel_additive(
                p.pos.x as i32,
                p.pos.y as i32,
                (p.color.0 as f32 * intensity) as u8,
                (p.color.1 as f32 * intensity) as u8,
                (p.color.2 as f32 * intensity) as u8,
            );
        }
    }

    /// Get particle count (for debugging/limiting)
    pub fn count(&self) -> usize {
        self.particles.len()
    }

    /// Clear all particles
    pub fn clear(&mut self) {
        self.particles.clear();
    }
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self::new()
    }
}
```

---

## File Changes Summary

| File | Changes |
|------|---------|
| `src/texture.rs` | New file with Texture struct, sampling (bilinear with clamping), procedural generators |
| `src/display/pixel_buffer.rs` | Add buffer operations (blit, fade, scroll, scroll_from for double-buffering, blit_scaled) and fill_polygon_additive (with preallocated scanline buffer) |
| `src/math3d.rs` | Add Mesh struct (with Default derive), cube, sphere, transforms, HashMap import |
| `src/particles.rs` | New file with Particle and ParticleSystem (uses Vec2 operators, stable drag) |
| `src/main.rs` | Add `mod texture; mod particles;` |

**Note:** Phase 2 was updated to add Vec2 operator traits (Add, Sub, Mul) for API consistency with Vec3.

---

## Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_texture_sample_wrap() {
        let tex = Texture::checkerboard(8, 4, (255, 0, 0), (0, 0, 255));
        // UV > 1.0 should wrap
        let c1 = tex.sample(0.0, 0.0);
        let c2 = tex.sample(1.0, 0.0); // Should wrap to same as 0.0
        assert_eq!(c1, c2);
    }

    #[test]
    fn test_mesh_cube_vertices() {
        let cube = Mesh::cube(2.0);
        assert_eq!(cube.vertices.len(), 8);
        assert_eq!(cube.faces.len(), 12); // 6 faces * 2 triangles
    }

    #[test]
    fn test_particle_system_emit_and_decay() {
        let mut ps = ParticleSystem::new();
        ps.emit(Particle::new(
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            1.0,
            (255, 255, 255),
        ));
        assert_eq!(ps.count(), 1);

        // Update past lifetime
        ps.update(2.0);
        assert_eq!(ps.count(), 0); // Should be removed
    }

    #[test]
    fn test_buffer_fade() {
        let mut buf = PixelBuffer::new();
        buf.set_pixel(100, 100, 200, 200, 200);
        buf.fade(0.5);
        let (r, g, b) = buf.get_pixel(100, 100).unwrap();
        assert!(r < 150 && r > 50); // Should be ~100
    }
}
```

---

## Implementation Order

1. Texture system - independent, no dependencies
2. Buffer operations - depends on get_pixel from Phase 1
3. fill_polygon_additive - depends on blend_pixel_additive from Phase 1
4. Mesh - depends on Vec3 from Phase 2
5. ParticleSystem - depends on Vec2 from Phase 2 and fill_circle_blend from Phase 1
