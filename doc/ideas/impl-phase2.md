# Phase 2 Implementation Plan

Enhanced primitives that enable ~80% of demoscene effects.

## Overview

Phase 2 adds circle outlines, line variants, and 3D projection utilities.

---

## 6. `draw_circle(cx, cy, radius, r, g, b)`

**Purpose:** Circle outline for moire patterns, ripples, targeting reticles.

**Location:** `src/display/pixel_buffer.rs`

**Implementation:** Midpoint circle algorithm drawing pixels instead of spans.

```rust
/// Draw a circle outline using midpoint algorithm
pub fn draw_circle(&mut self, cx: i32, cy: i32, radius: i32, r: u8, g: u8, b: u8) {
    if radius <= 0 {
        if radius == 0 {
            self.set_pixel(cx, cy, r, g, b);
        }
        return;
    }

    let mut x = radius;
    let mut y = 0;
    let mut err = 1 - radius;

    while x >= y {
        // Draw 8 octant points
        self.set_pixel(cx + x, cy + y, r, g, b);
        self.set_pixel(cx - x, cy + y, r, g, b);
        self.set_pixel(cx + x, cy - y, r, g, b);
        self.set_pixel(cx - x, cy - y, r, g, b);
        self.set_pixel(cx + y, cy + x, r, g, b);
        self.set_pixel(cx - y, cy + x, r, g, b);
        self.set_pixel(cx + y, cy - x, r, g, b);
        self.set_pixel(cx - y, cy - x, r, g, b);

        y += 1;
        if err < 0 {
            err += 2 * y + 1;
        } else {
            x -= 1;
            err += 2 * (y - x) + 1;
        }
    }
}
```

---

## 7. `hline_blend(x1, x2, y, r, g, b, a)`

**Purpose:** Horizontal line with alpha blending for raster bars and gradients.

**Location:** `src/display/pixel_buffer.rs`

**Implementation:**

```rust
/// Draw a horizontal line with alpha blending
pub fn hline_blend(&mut self, x1: i32, x2: i32, y: i32, r: u8, g: u8, b: u8, a: u8) {
    if y < 0 || y >= HEIGHT as i32 {
        return;
    }
    let (x1, x2) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
    let start = x1.max(0);
    let end = x2.min(WIDTH as i32 - 1);
    if start > end {
        return;
    }

    let alpha = a as u16;
    for x in start..=end {
        let idx = pixel_index(x as u32, y as u32);
        self.pixels[idx] = 255; // A
        self.pixels[idx + 1] = blend_channel(b, self.pixels[idx + 1], alpha);
        self.pixels[idx + 2] = blend_channel(g, self.pixels[idx + 2], alpha);
        self.pixels[idx + 3] = blend_channel(r, self.pixels[idx + 3], alpha);
    }
}

/// Draw a horizontal line with additive blending (for glow effects)
pub fn hline_additive(&mut self, x1: i32, x2: i32, y: i32, r: u8, g: u8, b: u8) {
    if y < 0 || y >= HEIGHT as i32 {
        return;
    }
    let (x1, x2) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
    let start = x1.max(0);
    let end = x2.min(WIDTH as i32 - 1);
    if start > end {
        return;
    }

    for x in start..=end {
        let idx = pixel_index(x as u32, y as u32);
        self.pixels[idx + 1] = self.pixels[idx + 1].saturating_add(b);
        self.pixels[idx + 2] = self.pixels[idx + 2].saturating_add(g);
        self.pixels[idx + 3] = self.pixels[idx + 3].saturating_add(r);
    }
}
```

---

## 8. `vline(x, y1, y2, r, g, b)`

**Purpose:** Vertical line for twister effect and vertical patterns.

**Location:** `src/display/pixel_buffer.rs`

**Implementation:**

```rust
/// Draw a vertical line
pub fn vline(&mut self, x: i32, y1: i32, y2: i32, r: u8, g: u8, b: u8) {
    if x < 0 || x >= WIDTH as i32 {
        return;
    }
    let (y1, y2) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };
    let start = y1.max(0);
    let end = y2.min(HEIGHT as i32 - 1);
    if start > end {
        return;
    }

    for y in start..=end {
        let idx = pixel_index(x as u32, y as u32);
        write_pixel(&mut self.pixels[idx..idx + 4], r, g, b);
    }
}

/// Draw a vertical line with alpha blending
pub fn vline_blend(&mut self, x: i32, y1: i32, y2: i32, r: u8, g: u8, b: u8, a: u8) {
    if x < 0 || x >= WIDTH as i32 {
        return;
    }
    let (y1, y2) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };
    let start = y1.max(0);
    let end = y2.min(HEIGHT as i32 - 1);
    if start > end {
        return;
    }

    let alpha = a as u16;
    for y in start..=end {
        let idx = pixel_index(x as u32, y as u32);
        self.pixels[idx] = 255;
        self.pixels[idx + 1] = blend_channel(b, self.pixels[idx + 1], alpha);
        self.pixels[idx + 2] = blend_channel(g, self.pixels[idx + 2], alpha);
        self.pixels[idx + 3] = blend_channel(r, self.pixels[idx + 3], alpha);
    }
}
```

---

## 9. 3D Projection Module

**Purpose:** Basic 3D math for all vector graphics effects.

**Location:** New file `src/math3d.rs`

**Implementation:**

```rust
//! 3D Math Utilities for Demoscene Effects
//!
//! Provides basic 3D vector operations, rotations, and perspective projection.

use std::ops::{Add, Sub, Mul};

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
        Self { x: 0.0, y: 0.0, z: 0.0 }
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
            Self { x: self.x / len, y: self.y / len }
        } else {
            *self
        }
    }
}

impl Add for Vec2 {
    type Output = Self;
    #[inline]
    fn add(self, other: Self) -> Self {
        Self { x: self.x + other.x, y: self.y + other.y }
    }
}

impl Sub for Vec2 {
    type Output = Self;
    #[inline]
    fn sub(self, other: Self) -> Self {
        Self { x: self.x - other.x, y: self.y - other.y }
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;
    #[inline]
    fn mul(self, scalar: f32) -> Self {
        Self { x: self.x * scalar, y: self.y * scalar }
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
    // Avoid division by zero and points behind camera
    if point.z <= 0.0 {
        return None;
    }

    let scale = fov / point.z;
    Some((cx + point.x * scale, cy + point.y * scale))
}

/// Project a 3D point, returning depth factor for brightness scaling
///
/// Returns (screen_x, screen_y, depth_factor) where depth_factor is 0.0-1.0
#[inline]
pub fn project_with_depth(point: Vec3, fov: f32, cx: f32, cy: f32, max_z: f32) -> Option<(f32, f32, f32)> {
    if point.z <= 0.0 {
        return None;
    }

    let scale = fov / point.z;
    let depth = 1.0 - (point.z / max_z).min(1.0);
    Some((cx + point.x * scale, cy + point.y * scale, depth))
}

/// Linear interpolation between two Vec3 points
#[inline]
pub fn lerp(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    Vec3 {
        x: a.x + (b.x - a.x) * t,
        y: a.y + (b.y - a.y) * t,
        z: a.z + (b.z - a.z) * t,
    }
}
```

**Add to main.rs:**

```rust
mod math3d;
pub use math3d::{Vec3, Vec2, project, project_with_depth, lerp};
```

---

## 10. `line_thick(x0, y0, x1, y1, thickness, r, g, b)`

**Purpose:** Thick lines for edge glow, laser effects, and enhanced wireframes.

**Location:** `src/display/pixel_buffer.rs`

**Implementation:** Draw parallel lines offset perpendicular to line direction.

```rust
/// Draw a line with variable thickness
/// For thickness=1, equivalent to regular line()
pub fn line_thick(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, thickness: i32, r: u8, g: u8, b: u8) {
    if thickness <= 1 {
        self.line(x0, y0, x1, y1, r, g, b);
        return;
    }

    // Calculate perpendicular direction
    let dx = (x1 - x0) as f32;
    let dy = (y1 - y0) as f32;
    let len = (dx * dx + dy * dy).sqrt();

    if len < 0.001 {
        // Degenerate line (single point) - draw filled circle
        self.fill_circle(x0, y0, thickness / 2, r, g, b);
        return;
    }

    // Perpendicular unit vector
    let px = -dy / len;
    let py = dx / len;

    // Draw parallel lines for each offset
    let half = (thickness - 1) as f32 / 2.0;
    for i in 0..thickness {
        let offset = (i as f32) - half;
        let ox = (px * offset) as i32;
        let oy = (py * offset) as i32;
        self.line(x0 + ox, y0 + oy, x1 + ox, y1 + oy, r, g, b);
    }
}

/// Draw a thick line with rounded ends (better for glow effects)
pub fn line_thick_rounded(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, thickness: i32, r: u8, g: u8, b: u8) {
    self.line_thick(x0, y0, x1, y1, thickness, r, g, b);

    // Add rounded caps
    let radius = thickness / 2;
    self.fill_circle(x0, y0, radius, r, g, b);
    self.fill_circle(x1, y1, radius, r, g, b);
}
```

---

## File Changes Summary

| File | Changes |
|------|---------|
| `src/display/pixel_buffer.rs` | Add `draw_circle`, `hline_blend`, `hline_additive`, `vline`, `vline_blend`, `line_thick`, `line_thick_rounded` |
| `src/math3d.rs` | New file with `Vec3`, `Vec2`, `project`, rotations |
| `src/main.rs` | Add `mod math3d;` |
| `src/display/mod.rs` | Re-export math3d types if desired |

---

## Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draw_circle_single_pixel() {
        let mut buf = PixelBuffer::new();
        buf.draw_circle(100, 100, 0, 255, 0, 0);
        assert_eq!(buf.get_pixel(100, 100), Some((255, 0, 0)));
    }

    #[test]
    fn test_vline_basic() {
        let mut buf = PixelBuffer::new();
        buf.vline(50, 10, 20, 0, 255, 0);
        assert_eq!(buf.get_pixel(50, 10), Some((0, 255, 0)));
        assert_eq!(buf.get_pixel(50, 15), Some((0, 255, 0)));
        assert_eq!(buf.get_pixel(50, 20), Some((0, 255, 0)));
        assert_eq!(buf.get_pixel(50, 21), Some((0, 0, 0))); // Outside
    }

    #[test]
    fn test_vec3_rotation() {
        use crate::math3d::Vec3;
        let p = Vec3::new(1.0, 0.0, 0.0);
        let rotated = p.rotate_z(std::f32::consts::FRAC_PI_2); // 90 degrees
        assert!((rotated.x - 0.0).abs() < 0.001);
        assert!((rotated.y - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_project_behind_camera() {
        use crate::math3d::{Vec3, project};
        let behind = Vec3::new(0.0, 0.0, -10.0);
        assert!(project(behind, 200.0, 320.0, 240.0).is_none());
    }
}
```

---

## Implementation Order

1. `vline` / `vline_blend` - simplest, mirrors hline pattern
2. `hline_blend` / `hline_additive` - simple, uses existing blend_channel
3. `draw_circle` - uses existing set_pixel
4. `math3d.rs` module - independent, no dependencies
5. `line_thick` / `line_thick_rounded` - depends on line and fill_circle
