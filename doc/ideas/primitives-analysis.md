# Primitives Analysis

Analysis of existing drawing primitives and new primitives needed for demoscene effects.

## Existing Primitives Catalog

### PixelBuffer (pixel_buffer.rs)

| Primitive | Signature | Description |
|-----------|-----------|-------------|
| `clear` | `(r, g, b)` | Fill entire buffer with solid color |
| `set_pixel` | `(x, y, r, g, b)` | Single pixel, bounds checked |
| `blend_pixel` | `(x, y, r, g, b, a)` | Alpha-blended pixel |
| `set_pixel_unchecked` | `(x, y, r, g, b)` | Fast pixel, no bounds check (unsafe) |
| `hline` | `(x1, x2, y, r, g, b)` | Horizontal line |
| `line` | `(x0, y0, x1, y1, r, g, b)` | Bresenham line drawing |
| `fill_rect` | `(x, y, w, h, r, g, b)` | Filled rectangle |
| `fill_polygon` | `(vertices, r, g, b)` | Scanline polygon fill |
| `fill_polygon_blend` | `(vertices, r, g, b, a)` | Alpha-blended polygon fill |
| `as_bytes` / `as_bytes_mut` | `()` | Raw pixel buffer access |

### Font System (font.rs)

| Primitive | Description |
|-----------|-------------|
| `draw_glyph` / `draw_glyph_scaled` | Render 8x8 bitmap glyph |
| `draw_char` / `draw_char_scaled` | Render single character |
| `draw_text` / `draw_text_scaled` | Render text string |
| `draw_text_centered` / `draw_text_centered_scaled` | Centered text |
| `draw_text_boxed` | Text with background box |
| `get_glyph` | Lookup glyph bitmap data |
| `text_width` / `text_width_scaled` | Measure text pixel width |
| `hsv_to_rgb` | HSV to RGB color conversion |

### Text Effects (text_fx.rs)

**Offset Functions** (return dx, dy):
- `wave`, `wobble`, `bounce`, `spread`, `circle`

**Color Functions** (return r, g, b):
- `rainbow`, `gradient`, `fade`, `pulse_brightness`, `lerp`

**Visibility Functions** (return 0.0-1.0):
- `blink`, `pulse`, `strobe`, `flash`, `blink_seq`, `blink_rand`, `fade_in`, `fade_out`

**Transform Functions**:
- `draw_char_flipped`, `draw_text_flipped`, `draw_text_reflected`
- `draw_text_shadowed`, `draw_text_outlined`

### Color Utilities (effects/mod.rs)

| Utility | Description |
|---------|-------------|
| `color::gray` | Grayscale color |
| `color::hsv_to_rgb` | HSV to RGB |
| `color::make_palette` | Generate demoscene palette |
| `color::fire_palette` | Fire gradient palette |

### Geometry (regions/)

| Primitive | Description |
|-----------|-------------|
| `Polygon::contains` | Point-in-polygon test |
| `Polygon::bounds` | Bounding box (min_x, min_y, max_x, max_y) |
| `Polygon::edges` | Iterator over edge segments |
| `Point::distance_to` | Distance between points |

---

## Effect Analysis

### Legend
- ✅ = Has primitive
- ⚠️ = Partially covered (needs extension)
- ❌ = Needs new primitive

---

## CLASSICS

### Raster Bars (Copper Bars)
| Need | Status | Primitive |
|------|--------|-----------|
| Horizontal gradient lines | ⚠️ | `hline` exists but need `hline_blend` |
| Palette animation | ❌ | `indexed_buffer` or color cycling |
| Sine wave positioning | ✅ | Can compute per-scanline |

**New primitives:** `hline_blend`, palette/indexed color support

### Plasma
| Need | Status | Primitive |
|------|--------|-----------|
| Per-pixel color from function | ✅ | `set_pixel_unchecked` in loop |
| Sine table | ✅ | Already in plasma.rs |
| Palette lookup | ✅ | `make_palette` exists |

**New primitives:** None required

### Fire
| Need | Status | Primitive |
|------|--------|-----------|
| Cellular automaton buffer | ✅ | Already implemented |
| Palette lookup | ✅ | `fire_palette` exists |

**New primitives:** None required

### Tunnel
| Need | Status | Primitive |
|------|--------|-----------|
| Per-pixel texture lookup | ❌ | `sample_texture(u, v)` |
| Polar coordinate conversion | ❌ | Helper functions |
| Texture wrapping | ❌ | Part of texture system |

**New primitives:** Texture sampling system

### Scrollers
| Need | Status | Primitive |
|------|--------|-----------|
| Text rendering | ✅ | Full system exists |
| Per-char positioning | ✅ | `draw_char_scaled` |
| Wave/wobble effects | ✅ | `text_fx::offset` module |

**New primitives:** None required

---

## SPRITES & BOBS

### Bobs (Bouncing Objects)
| Need | Status | Primitive |
|------|--------|-----------|
| Sprite rendering | ❌ | `draw_sprite(x, y, sprite_data)` |
| Sprite blending | ❌ | `draw_sprite_blend` |
| Collision with polygon | ❌ | `polygon_edge_collision` |

**New primitives:** Sprite system, polygon collision

### Shadebobs
| Need | Status | Primitive |
|------|--------|-----------|
| Additive blending | ❌ | `blend_pixel_additive` |
| Filled circle | ❌ | `fill_circle` |
| Circle with falloff | ❌ | `fill_circle_gradient` |

**New primitives:** Additive blending, circle primitives

### Dot Balls / Spheres
| Need | Status | Primitive |
|------|--------|-----------|
| 3D point projection | ❌ | `project_3d(x, y, z)` |
| Depth-based brightness | ✅ | Can compute |
| 3D rotation matrices | ❌ | `rotate_3d` helpers |

**New primitives:** 3D math utilities

### Sinus Dots
| Need | Status | Primitive |
|------|--------|-----------|
| Point rendering | ✅ | `set_pixel` |
| Lissajous computation | ✅ | Math only |
| Trail effect | ❌ | `fade_buffer` or trails system |

**New primitives:** Buffer fade/trails

---

## 3D VECTOR

### Wireframe Vectors
| Need | Status | Primitive |
|------|--------|-----------|
| Line drawing | ✅ | `line` exists |
| 3D projection | ❌ | `project_3d` |
| 3D transforms | ❌ | Matrix/rotation helpers |
| Depth sorting | ❌ | Z-buffer or painter's algorithm |

**New primitives:** 3D projection, transforms, depth handling

### Glenz Vectors
| Need | Status | Primitive |
|------|--------|-----------|
| Filled polygons | ✅ | `fill_polygon` |
| Additive blending | ❌ | `fill_polygon_additive` |
| 3D transforms | ❌ | See wireframe |

**New primitives:** Additive polygon fill

### Gouraud Shading
| Need | Status | Primitive |
|------|--------|-----------|
| Per-vertex colors | ❌ | `fill_polygon_gouraud` |
| Color interpolation | ✅ | `lerp` exists |

**New primitives:** Gouraud-shaded polygon fill

### 3D Morphing
| Need | Status | Primitive |
|------|--------|-----------|
| Vertex interpolation | ✅ | Simple lerp |
| Mesh data structure | ❌ | `Mesh` type with vertices/faces |

**New primitives:** Mesh data structure

---

## DISTORTION

### Rotozoomer
| Need | Status | Primitive |
|------|--------|-----------|
| Texture source | ❌ | `Texture` type |
| Texture sampling | ❌ | `sample_texture(u, v)` |
| Coordinate transform | ✅ | Math only |

**New primitives:** Texture system

### Twister
| Need | Status | Primitive |
|------|--------|-----------|
| Vertical slice rendering | ✅ | Can use `vline` or pixel loops |
| Texture sampling | ❌ | See rotozoomer |

**New primitives:** Texture system, `vline`

### Warp / Displacement
| Need | Status | Primitive |
|------|--------|-----------|
| Displacement map | ❌ | Second buffer for offsets |
| Pixel read | ❌ | `get_pixel(x, y)` |
| Offset sampling | ❌ | Part of displacement system |

**New primitives:** `get_pixel`, displacement mapping

### Moire Patterns
| Need | Status | Primitive |
|------|--------|-----------|
| Concentric circles | ❌ | `draw_circle` (outline) |
| Distance calculation | ✅ | `Point::distance_to` |

**New primitives:** Circle outline

### Feedback / Recursion
| Need | Status | Primitive |
|------|--------|-----------|
| Buffer copy | ❌ | `copy_buffer` or double buffering |
| Scaled blit | ❌ | `blit_scaled(src, dst, transform)` |
| Buffer read | ❌ | `get_pixel` |

**New primitives:** Buffer operations, scaled blit

### Lens Effect
| Need | Status | Primitive |
|------|--------|-----------|
| Pixel read | ❌ | `get_pixel` |
| Distortion math | ✅ | Compute only |

**New primitives:** `get_pixel`

---

## GEOMETRY-DRIVEN

### Edge Glow
| Need | Status | Primitive |
|------|--------|-----------|
| Distance to edge | ❌ | `distance_to_polygon_edge` |
| Glow falloff | ✅ | Math |
| Line with thickness | ❌ | `line_thick` or glow shader |

**New primitives:** Distance to edge, thick line

### Polygon Morphing
| Need | Status | Primitive |
|------|--------|-----------|
| Vertex lerp | ✅ | Simple math |
| Centroid calculation | ❌ | `Polygon::centroid` |

**New primitives:** `Polygon::centroid`

### Voronoi
| Need | Status | Primitive |
|------|--------|-----------|
| Nearest point lookup | ❌ | Voronoi algorithm |
| Distance field | ❌ | Per-pixel distance calc |

**New primitives:** Voronoi helpers (or brute force per-pixel)

### Shockwave Ripples
| Need | Status | Primitive |
|------|--------|-----------|
| Distance from point | ✅ | `Point::distance_to` |
| Sine wave modulation | ✅ | Math |
| Centroid | ❌ | `Polygon::centroid` |

**New primitives:** `Polygon::centroid`

### Vector Balls on Vertices
| Need | Status | Primitive |
|------|--------|-----------|
| Filled circle | ❌ | `fill_circle` |
| Shaded sphere | ❌ | `fill_circle_shaded` |
| Vertex access | ✅ | `polygon.vertices` |

**New primitives:** Circle primitives, sphere shading

### Laser Grid
| Need | Status | Primitive |
|------|--------|-----------|
| Line drawing | ✅ | `line` |
| Glow effect | ❌ | `line_glow` or bloom pass |
| Edge iteration | ✅ | `polygon.edges()` |

**New primitives:** Line glow / bloom

---

## CROSS-REGION

### Lightning
| Need | Status | Primitive |
|------|--------|-----------|
| Jagged line | ❌ | `line_lightning(p1, p2, jitter)` |
| Glow | ❌ | Line glow |
| Centroid | ❌ | `Polygon::centroid` |

**New primitives:** Lightning line, centroid

### Particle Streams
| Need | Status | Primitive |
|------|--------|-----------|
| Particle system | ❌ | `ParticleSystem` type |
| Point rendering | ✅ | `set_pixel` |
| Bezier curves | ❌ | `bezier_point(t, p0, p1, p2, p3)` |

**New primitives:** Particle system, bezier math

### Infection/Contagion
| Need | Status | Primitive |
|------|--------|-----------|
| Region proximity | ❌ | `regions_near(region, distance)` |
| State per region | ✅ | Can track externally |

**New primitives:** Region proximity query

---

## METABALLS

### 2D Metaballs
| Need | Status | Primitive |
|------|--------|-----------|
| Density field calc | ❌ | `metaball_field(balls, x, y)` |
| Threshold rendering | ✅ | Per-pixel with `set_pixel` |
| Gradient coloring | ✅ | Palette lookup |

**New primitives:** Metaball field calculation

### 3D Metaballs
| Need | Status | Primitive |
|------|--------|-----------|
| Marching cubes | ❌ | Full algorithm |
| 3D mesh rendering | ❌ | Mesh + projection |

**New primitives:** Marching cubes (advanced)

---

## PSYCHEDELIC

### Fractal Zoom
| Need | Status | Primitive |
|------|--------|-----------|
| Complex iteration | ✅ | Math only |
| Palette lookup | ✅ | Existing |
| Per-pixel rendering | ✅ | `set_pixel_unchecked` |

**New primitives:** None required (compute-heavy)

### Kaleidoscope
| Need | Status | Primitive |
|------|--------|-----------|
| Angular coordinate transform | ✅ | Math |
| Texture/buffer sampling | ❌ | `get_pixel` |

**New primitives:** `get_pixel`

### Color Cycling
| Need | Status | Primitive |
|------|--------|-----------|
| Indexed color buffer | ❌ | `IndexedBuffer` type |
| Palette rotation | ❌ | `rotate_palette` |

**New primitives:** Indexed color system

### Interference Patterns
| Need | Status | Primitive |
|------|--------|-----------|
| Multiple wave sources | ✅ | Math |
| Per-pixel sine sum | ✅ | Math |

**New primitives:** None required

### Reaction-Diffusion
| Need | Status | Primitive |
|------|--------|-----------|
| Double buffer | ❌ | Two buffers + swap |
| Neighbor sampling | ❌ | Part of buffer ops |
| Convolution | ❌ | `convolve_3x3` helper |

**New primitives:** Double buffering, convolution

---

## AUDIO-REACTIVE

### Spectrum Analyzer
| Need | Status | Primitive |
|------|--------|-----------|
| Filled rect | ✅ | `fill_rect` |
| Audio FFT data | ❌ | External audio system |

**New primitives:** Audio input (external)

### Beat Detection
| Need | Status | Primitive |
|------|--------|-----------|
| Audio analysis | ❌ | External |
| Trigger system | ✅ | Can implement in effect |

**New primitives:** Audio input (external)

---

## New Primitives Summary (Deduplicated)

### High Priority (enables many effects)

1. **`get_pixel(x, y) -> (r, g, b)`** - Read pixel from buffer
   - Needed for: Kaleidoscope, lens, feedback, displacement

2. **`fill_circle(cx, cy, r, color)`** - Filled circle
   - Needed for: Shadebobs, vector balls, metaballs viz, dots

3. **`fill_circle_blend(cx, cy, r, color, alpha)`** - Blended circle
   - Needed for: Glow effects, soft particles

4. **`Polygon::centroid() -> Point`** - Polygon center
   - Needed for: Ripples, lightning, particles, voronoi seeds

5. **`blend_pixel_additive(x, y, r, g, b)`** - Additive blending
   - Needed for: Shadebobs, glenz vectors, glow, fire

6. **`hline_blend(x1, x2, y, r, g, b, a)`** - Blended horizontal line
   - Needed for: Raster bars, gradient fills

### Medium Priority (enables several effects)

7. **`draw_circle(cx, cy, r, color)`** - Circle outline
   - Needed for: Moire, ripples, targeting reticles

8. **`vline(y1, y2, x, color)`** - Vertical line
   - Needed for: Twister, vertical effects

9. **`line_thick(x0, y0, x1, y1, thickness, color)`** - Thick line
   - Needed for: Edge glow, laser grid, wireframes

10. **3D Projection utilities**
    - `project_3d(x, y, z, fov) -> (screen_x, screen_y)`
    - `rotate_x/y/z(point, angle) -> point`
    - Needed for: All 3D vector effects

11. **`Texture` type + `sample_texture(tex, u, v)`**
    - Needed for: Rotozoomer, tunnel, textured 3D

12. **`copy_region(src_buf, dst_buf, src_rect, dst_rect)`**
    - Needed for: Feedback, transitions, double buffering

### Lower Priority (specialized effects)

13. **`fill_polygon_additive(vertices, r, g, b)`** - Additive polygon
    - Needed for: Glenz vectors

14. **`fill_polygon_gouraud(vertices, colors)`** - Shaded polygon
    - Needed for: Smooth-shaded 3D

15. **`line_lightning(p1, p2, segments, jitter)`** - Jagged line
    - Needed for: Lightning effect

16. **`bezier_point(t, p0, p1, p2, p3)`** - Bezier interpolation
    - Needed for: Particle paths, smooth curves

17. **`distance_to_line_segment(point, p1, p2)`** - Point-to-segment distance
    - Needed for: Edge glow, collision

18. **`Mesh` data structure** - Vertices + faces + normals
    - Needed for: 3D objects, morphing

19. **`ParticleSystem`** - Particle management
    - Needed for: Particle streams, explosions, sparks

20. **Indexed color system** (`IndexedBuffer` + palette rotation)
    - Needed for: Classic color cycling effects

---

## Minimal Implementation Order

To enable the most effects with least work:

### Phase 1: Core Graphics (enables ~60% of effects)
1. `get_pixel` - buffer reading
2. `fill_circle` / `fill_circle_blend` - circles
3. `Polygon::centroid` - geometry helper
4. `blend_pixel_additive` - additive blending

### Phase 2: Enhanced Primitives (enables ~80% of effects)
5. `draw_circle` - circle outline
6. `hline_blend` / `vline` - gradient lines
7. 3D projection utilities
8. `line_thick` - thick lines

### Phase 3: Advanced Systems (enables remaining effects)
9. Texture system
10. Buffer copy/blit operations
11. Mesh data structure
12. Particle system
13. Indexed color buffer
