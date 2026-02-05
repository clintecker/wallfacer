# Phase 1 Implementation Plan

Core graphics primitives that enable ~60% of demoscene effects.

## Overview

All Phase 1 primitives are additions to `PixelBuffer` in `src/display/pixel_buffer.rs`, plus one addition to `Polygon` in `src/regions/polygon.rs`.

---

## 1. `get_pixel(x, y) -> Option<(u8, u8, u8)>`

**Purpose:** Read pixel color from buffer. Essential for feedback, displacement, and kaleidoscope effects.

**Implementation:**

```rust
/// Read a pixel from the buffer (bounds checked)
/// Returns None if coordinates are out of bounds
#[inline]
pub fn get_pixel(&self, x: i32, y: i32) -> Option<(u8, u8, u8)> {
    if in_bounds(x, y) {
        let idx = pixel_index(x as u32, y as u32);
        // Pixel format is ABGR (little-endian RGBA8888)
        Some((
            self.pixels[idx + 3], // R
            self.pixels[idx + 2], // G
            self.pixels[idx + 1], // B
        ))
    } else {
        None
    }
}

/// Fast unchecked pixel read - use when bounds already verified
#[inline]
pub unsafe fn get_pixel_unchecked(&self, x: u32, y: u32) -> (u8, u8, u8) {
    let idx = pixel_index(x, y);
    (
        *self.pixels.get_unchecked(idx + 3),
        *self.pixels.get_unchecked(idx + 2),
        *self.pixels.get_unchecked(idx + 1),
    )
}
```

**Compliance notes:**
- Follows existing pattern: safe version with bounds check, unsafe unchecked version
- Uses existing `in_bounds` and `pixel_index` helpers
- Returns Option for safe version (Rust idiom)
- Matches ABGR byte order used in `set_pixel`

---

## 2. `fill_circle(cx, cy, radius, r, g, b)`

**Purpose:** Filled circle for shadebobs, metaballs, vector balls, particle dots.

**Implementation:** Use midpoint circle algorithm with horizontal span filling for efficiency.

```rust
/// Fill a circle using midpoint algorithm with scanline optimization
pub fn fill_circle(&mut self, cx: i32, cy: i32, radius: i32, r: u8, g: u8, b: u8) {
    if radius <= 0 {
        return;
    }

    // Midpoint circle algorithm, filling horizontal spans
    let mut x = radius;
    let mut y = 0;
    let mut err = 1 - radius;

    while x >= y {
        // Fill horizontal spans, avoiding duplicates
        self.hline(cx - x, cx + x, cy + y, r, g, b);
        if y != 0 {
            self.hline(cx - x, cx + x, cy - y, r, g, b);
        }
        if x != y {
            self.hline(cx - y, cx + y, cy + x, r, g, b);
            if y != 0 {
                self.hline(cx - y, cx + y, cy - x, r, g, b);
            }
        }

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

**Compliance notes:**
- Uses existing `hline` primitive for span filling
- No allocations, purely stack-based
- Handles edge cases (radius <= 0)
- Could add unchecked version later if profiling shows need

---

## 3. `fill_circle_blend(cx, cy, radius, r, g, b, a)`

**Purpose:** Alpha-blended filled circle for soft particles and glow effects.

**Implementation:**

```rust
/// Fill a circle with alpha blending
pub fn fill_circle_blend(&mut self, cx: i32, cy: i32, radius: i32, r: u8, g: u8, b: u8, a: u8) {
    if radius <= 0 {
        return;
    }

    let mut x = radius;
    let mut y = 0;
    let mut err = 1 - radius;

    while x >= y {
        // Fill horizontal spans with blending
        self.hline_blend(cx - x, cx + x, cy + y, r, g, b, a);
        self.hline_blend(cx - x, cx + x, cy - y, r, g, b, a);
        self.hline_blend(cx - y, cx + y, cy + x, r, g, b, a);
        self.hline_blend(cx - y, cx + y, cy - x, r, g, b, a);

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

**Dependency:** Requires `hline_blend` (Phase 2, item 7).

**Standalone version (inlined to avoid borrow issues):**

```rust
/// Fill a circle with alpha blending (standalone version)
/// Inlines hline_blend logic to avoid closure borrow conflicts
pub fn fill_circle_blend(&mut self, cx: i32, cy: i32, radius: i32, r: u8, g: u8, b: u8, a: u8) {
    if radius <= 0 {
        return;
    }

    let mut xi = radius;
    let mut y = 0;
    let mut err = 1 - radius;

    while xi >= y {
        // Inline helper macro to fill a horizontal span with blending
        macro_rules! fill_span {
            ($x1:expr, $x2:expr, $line_y:expr) => {{
                let line_y = $line_y;
                if line_y >= 0 && line_y < HEIGHT as i32 {
                    let (x1, x2) = if $x1 <= $x2 { ($x1, $x2) } else { ($x2, $x1) };
                    let start = x1.max(0);
                    let end = x2.min(WIDTH as i32 - 1);
                    for px in start..=end {
                        self.blend_pixel(px, line_y, r, g, b, a);
                    }
                }
            }};
        }

        fill_span!(cx - xi, cx + xi, cy + y);
        if y != 0 {
            fill_span!(cx - xi, cx + xi, cy - y);
        }
        if xi != y {
            fill_span!(cx - y, cx + y, cy + xi);
            if y != 0 {
                fill_span!(cx - y, cx + y, cy - xi);
            }
        }

        y += 1;
        if err < 0 {
            err += 2 * y + 1;
        } else {
            xi -= 1;
            err += 2 * (y - xi) + 1;
        }
    }
}
```

**Note:** Uses a macro to avoid closure borrow conflicts while keeping code readable.
Also includes duplicate span avoidance for consistency with `fill_circle`.

---

## 4. `Polygon::centroid() -> Point`

**Purpose:** Calculate geometric center of polygon for ripples, lightning, particles.

**Location:** `src/regions/polygon.rs`

**Implementation:**

```rust
/// Calculate the centroid (geometric center) of the polygon
/// Returns None if the polygon is empty (consistent with bounds())
pub fn centroid(&self) -> Option<Point> {
    if self.vertices.is_empty() {
        return None;
    }

    let n = self.vertices.len() as f32;
    let sum_x: f32 = self.vertices.iter().map(|v| v.x).sum();
    let sum_y: f32 = self.vertices.iter().map(|v| v.y).sum();

    Some(Point::new(sum_x / n, sum_y / n))
}
```

**Note:** This is the simple average centroid. For more accuracy with complex polygons, could implement signed area centroid:

```rust
/// Calculate centroid using signed area formula (more accurate for complex shapes)
pub fn centroid_accurate(&self) -> Point {
    if self.vertices.len() < 3 {
        return self.centroid(); // Fall back to simple average
    }

    let mut cx = 0.0;
    let mut cy = 0.0;
    let mut signed_area = 0.0;

    let n = self.vertices.len();
    for i in 0..n {
        let v0 = &self.vertices[i];
        let v1 = &self.vertices[(i + 1) % n];

        let cross = v0.x * v1.y - v1.x * v0.y;
        signed_area += cross;
        cx += (v0.x + v1.x) * cross;
        cy += (v0.y + v1.y) * cross;
    }

    signed_area *= 0.5;
    if signed_area.abs() < 1e-10 {
        return self.centroid(); // Degenerate polygon
    }

    let factor = 1.0 / (6.0 * signed_area);
    Point::new(cx * factor, cy * factor)
}
```

---

## 5. `blend_pixel_additive(x, y, r, g, b)`

**Purpose:** Additive blending for glenz vectors, glow, shadebobs. Colors add up and saturate at 255.

**Implementation:**

```rust
/// Additive blend a pixel (colors saturate at 255)
/// Used for glow effects, glenz vectors, and shadebobs
#[inline]
pub fn blend_pixel_additive(&mut self, x: i32, y: i32, r: u8, g: u8, b: u8) {
    if in_bounds(x, y) {
        let idx = pixel_index(x as u32, y as u32);
        // Saturating add prevents overflow
        self.pixels[idx + 1] = self.pixels[idx + 1].saturating_add(b);
        self.pixels[idx + 2] = self.pixels[idx + 2].saturating_add(g);
        self.pixels[idx + 3] = self.pixels[idx + 3].saturating_add(r);
    }
}

/// Fast unchecked additive blend
#[inline]
pub unsafe fn blend_pixel_additive_unchecked(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8) {
    let idx = pixel_index(x, y);
    let pixels = &mut self.pixels;
    *pixels.get_unchecked_mut(idx + 1) = pixels.get_unchecked(idx + 1).saturating_add(b);
    *pixels.get_unchecked_mut(idx + 2) = pixels.get_unchecked(idx + 2).saturating_add(g);
    *pixels.get_unchecked_mut(idx + 3) = pixels.get_unchecked(idx + 3).saturating_add(r);
}
```

**Compliance notes:**
- Uses `saturating_add` for correct clamping behavior
- Follows existing safe/unsafe pattern
- Very simple and efficient

---

## Testing Strategy

Add to existing test suite or create `src/display/pixel_buffer_tests.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_set_pixel_roundtrip() {
        let mut buf = PixelBuffer::new();
        buf.set_pixel(100, 100, 255, 128, 64);
        assert_eq!(buf.get_pixel(100, 100), Some((255, 128, 64)));
    }

    #[test]
    fn test_get_pixel_out_of_bounds() {
        let buf = PixelBuffer::new();
        assert_eq!(buf.get_pixel(-1, 0), None);
        assert_eq!(buf.get_pixel(0, -1), None);
        assert_eq!(buf.get_pixel(640, 0), None);
        assert_eq!(buf.get_pixel(0, 480), None);
    }

    #[test]
    fn test_additive_blend_saturates() {
        let mut buf = PixelBuffer::new();
        buf.set_pixel(50, 50, 200, 200, 200);
        buf.blend_pixel_additive(50, 50, 100, 100, 100);
        // Should saturate at 255, not overflow
        assert_eq!(buf.get_pixel(50, 50), Some((255, 255, 255)));
    }

    #[test]
    fn test_fill_circle_basic() {
        let mut buf = PixelBuffer::new();
        buf.fill_circle(320, 240, 50, 255, 0, 0);
        // Center should be filled
        assert_eq!(buf.get_pixel(320, 240), Some((255, 0, 0)));
        // Edge should be filled
        assert_eq!(buf.get_pixel(320 + 49, 240), Some((255, 0, 0)));
        // Outside should be empty
        assert_eq!(buf.get_pixel(320 + 52, 240), Some((0, 0, 0)));
    }
}
```

---

## File Changes Summary

| File | Changes |
|------|---------|
| `src/display/pixel_buffer.rs` | Add `get_pixel`, `get_pixel_unchecked`, `fill_circle`, `fill_circle_blend`, `blend_pixel_additive`, `blend_pixel_additive_unchecked` |
| `src/regions/polygon.rs` | Add `centroid()` method |

---

## Implementation Order

1. `get_pixel` / `get_pixel_unchecked` - simplest, no dependencies
2. `blend_pixel_additive` / `blend_pixel_additive_unchecked` - simple, no dependencies
3. `Polygon::centroid` - simple, separate file
4. `fill_circle` - uses existing `hline`
5. `fill_circle_blend` - uses existing `blend_pixel`
