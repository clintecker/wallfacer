# Distortion Effects

Warping, stretching, and transforming images/textures in real-time.

## Rotozoomer

Rotating and zooming a tiled texture across the screen.

**How it works:** For each screen pixel, calculate source texture coordinate by applying rotation matrix and scale. Wrap coordinates for seamless tiling.

**History:** Invented by Chaos/Sanity on Amiga 500 in 1989. Became the foundation for texture mapping in 3D engines.

**Variations:**
- Multi-layer rotozoom (parallax)
- Per-region independent rotation/zoom
- Animated texture (plasma as source)

**Region interaction:** Each region has its own rotozoom with independent parameters.

## Twister

A rotating, twisted vertical column.

**How it works:** Vertical slices of an image/texture, each rotated by different amount based on Y position and time.

**Variations:**
- Single twister
- Multiple interleaved twisters
- Textured vs. solid color bars

## Warp / Displacement

Distorting an image based on a displacement map.

**How it works:** For each pixel, read displacement from a separate buffer. Offset source coordinates by displacement amount.

**Uses:**
- Water ripple effect
- Heat shimmer
- Lens distortion
- Shockwave from explosions

## Moire Patterns

Interference patterns from overlapping regular structures.

**How it works:** Draw two sets of concentric circles or parallel lines. Interference creates organic flowing patterns.

**Region interaction:** Each region's centroid becomes a moire origin point.

## Feedback / Recursion

Using the previous frame as input for the current frame.

**How it works:** Copy previous frame, apply transformation (scale, rotate, color shift), blend with new content.

**Classic effects:**
- Infinite tunnel zoom
- Spiral vortex
- Trailing/echo effect

**Region interaction:** Feedback only within specific regions, creating localized infinity effects.

## Lens Effect

Magnifying or distorting a circular region.

**How it works:** For pixels within lens radius, calculate refracted source position using lens equation.

**Variations:**
- Following mouse/cursor
- Multiple bouncing lenses
- Fisheye distortion
