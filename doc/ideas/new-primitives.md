# New Primitives Required

Minimal set of new drawing primitives to implement all demoscene effects.

## Phase 1: Core Graphics
*Enables ~60% of effects*

### 1. `get_pixel(x, y) -> Option<(u8, u8, u8)>`
Read a pixel from the buffer. Essential for feedback, displacement, and kaleidoscope effects.

```rust
pub fn get_pixel(&self, x: i32, y: i32) -> Option<(u8, u8, u8)>
```

### 2. `fill_circle(cx, cy, radius, r, g, b)`
Filled circle using midpoint algorithm.

```rust
pub fn fill_circle(&mut self, cx: i32, cy: i32, radius: i32, r: u8, g: u8, b: u8)
```

### 3. `fill_circle_blend(cx, cy, radius, r, g, b, a)`
Filled circle with alpha blending for soft particles and glow.

```rust
pub fn fill_circle_blend(&mut self, cx: i32, cy: i32, radius: i32, r: u8, g: u8, b: u8, a: u8)
```

### 4. `Polygon::centroid() -> Point`
Calculate geometric center of polygon.

```rust
impl Polygon {
    pub fn centroid(&self) -> Point {
        let n = self.vertices.len() as f32;
        let sum_x: f32 = self.vertices.iter().map(|v| v.x).sum();
        let sum_y: f32 = self.vertices.iter().map(|v| v.y).sum();
        Point::new(sum_x / n, sum_y / n)
    }
}
```

### 5. `blend_pixel_additive(x, y, r, g, b)`
Additive blending (saturating add). Critical for glenz vectors, glow, and shadebobs.

```rust
pub fn blend_pixel_additive(&mut self, x: i32, y: i32, r: u8, g: u8, b: u8)
```

---

## Phase 2: Enhanced Primitives
*Enables ~80% of effects*

### 6. `draw_circle(cx, cy, radius, r, g, b)`
Circle outline using midpoint algorithm.

```rust
pub fn draw_circle(&mut self, cx: i32, cy: i32, radius: i32, r: u8, g: u8, b: u8)
```

### 7. `hline_blend(x1, x2, y, r, g, b, a)`
Horizontal line with alpha blending for raster bars.

```rust
pub fn hline_blend(&mut self, x1: i32, x2: i32, y: i32, r: u8, g: u8, b: u8, a: u8)
```

### 8. `vline(x, y1, y2, r, g, b)`
Vertical line (complements `hline`).

```rust
pub fn vline(&mut self, x: i32, y1: i32, y2: i32, r: u8, g: u8, b: u8)
```

### 9. 3D Projection Module
Basic 3D math for vector graphics.

```rust
pub mod math3d {
    pub struct Vec3 { pub x: f32, pub y: f32, pub z: f32 }

    pub fn project(p: Vec3, fov: f32, cx: f32, cy: f32) -> (f32, f32) {
        let scale = fov / (fov + p.z);
        (cx + p.x * scale, cy + p.y * scale)
    }

    pub fn rotate_x(p: Vec3, angle: f32) -> Vec3;
    pub fn rotate_y(p: Vec3, angle: f32) -> Vec3;
    pub fn rotate_z(p: Vec3, angle: f32) -> Vec3;
}
```

### 10. `line_thick(x0, y0, x1, y1, thickness, r, g, b)`
Line with variable thickness for edge glow and laser effects.

```rust
pub fn line_thick(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, thickness: i32, r: u8, g: u8, b: u8)
```

---

## Phase 3: Advanced Systems
*Enables remaining effects*

### 11. Texture System

```rust
pub struct Texture {
    width: u32,
    height: u32,
    pixels: Vec<u8>,  // RGBA
}

impl Texture {
    pub fn sample(&self, u: f32, v: f32) -> (u8, u8, u8);  // Wrapped UV
    pub fn sample_bilinear(&self, u: f32, v: f32) -> (u8, u8, u8);
}
```

### 12. Buffer Operations

```rust
impl PixelBuffer {
    pub fn blit(&mut self, src: &PixelBuffer, x: i32, y: i32);
    pub fn blit_scaled(&mut self, src: &PixelBuffer, dst_rect: Rect, src_rect: Rect);
    pub fn fade(&mut self, factor: f32);  // Multiply all pixels by factor
}
```

### 13. `fill_polygon_additive(vertices, r, g, b)`
Polygon fill with additive blending for glenz effect.

```rust
pub fn fill_polygon_additive(&mut self, vertices: &[(f32, f32)], r: u8, g: u8, b: u8)
```

### 14. Mesh Data Structure

```rust
pub struct Mesh {
    pub vertices: Vec<Vec3>,
    pub faces: Vec<[usize; 3]>,  // Triangle indices
}

impl Mesh {
    pub fn cube(size: f32) -> Self;
    pub fn sphere(radius: f32, segments: u32) -> Self;
    pub fn transform(&mut self, matrix: &Mat4);
}
```

### 15. Particle System

```rust
pub struct Particle {
    pub pos: Vec2,
    pub vel: Vec2,
    pub life: f32,
    pub color: (u8, u8, u8),
}

pub struct ParticleSystem {
    particles: Vec<Particle>,
}

impl ParticleSystem {
    pub fn emit(&mut self, pos: Vec2, count: usize);
    pub fn update(&mut self, dt: f32);
    pub fn render(&self, buffer: &mut PixelBuffer);
}
```

---

## Utility Helpers

### Distance Functions
```rust
pub fn distance_to_segment(point: Point, p1: Point, p2: Point) -> f32;
pub fn distance_to_polygon_edge(point: Point, polygon: &Polygon) -> f32;
```

### Bezier Curves
```rust
pub fn bezier_quadratic(t: f32, p0: Point, p1: Point, p2: Point) -> Point;
pub fn bezier_cubic(t: f32, p0: Point, p1: Point, p2: Point, p3: Point) -> Point;
```

### Lightning Generation
```rust
pub fn generate_lightning(p1: Point, p2: Point, segments: u32, jitter: f32) -> Vec<Point>;
```

---

## Summary Table

| Priority | Primitive | Effects Enabled |
|----------|-----------|-----------------|
| **P1** | `get_pixel` | Feedback, displacement, kaleidoscope, lens |
| **P1** | `fill_circle` | Shadebobs, vector balls, metaballs, dots |
| **P1** | `fill_circle_blend` | Glow, soft particles |
| **P1** | `Polygon::centroid` | Ripples, lightning, particles |
| **P1** | `blend_pixel_additive` | Glenz, glow, shadebobs, fire |
| **P2** | `draw_circle` | Moire, ripples, reticles |
| **P2** | `hline_blend` | Raster bars, gradients |
| **P2** | `vline` | Twister, vertical effects |
| **P2** | 3D math module | All vector graphics |
| **P2** | `line_thick` | Edge glow, lasers |
| **P3** | Texture system | Rotozoomer, tunnel |
| **P3** | Buffer ops | Feedback, transitions |
| **P3** | `fill_polygon_additive` | Glenz vectors |
| **P3** | Mesh structure | 3D objects |
| **P3** | ParticleSystem | Particles, sparks |

---

## Implementation Notes

- Phase 1 primitives are simple additions to `PixelBuffer`
- Phase 2 adds slightly more complex drawing algorithms
- Phase 3 involves new data structures and systems
- All primitives should maintain the existing unsafe optimization pattern for hot paths
- Consider SIMD optimization for blend operations
