# Sprites & Bobs

Moving graphical objects - the bread and butter of oldschool demos.

## Bobs (Blitter Objects)

Software-rendered sprites using the blitter for fast memory copies.

**Classic demo flex:** Showing as many bobs as possible on screen, all moving independently. No hardware sprite limit - just CPU/blitter bandwidth.

**Variations:**
- Vector bobs (3D coordinates projected to 2D)
- Star bobs (points radiating from center)
- Logo bobs (company/group logos bouncing around)

**Region interaction:** Bobs confined within polygon boundaries with edge collision detection.

## Shadebobs

Circles/squares that accumulate color when overlapping.

**How it works:** Draw shapes with additive blending. Overlapping areas get brighter, creating glowing intersections.

**Variations:**
- Color cycling shadebobs
- Trails (previous positions fade slowly)
- Multiple color channels (RGB shadebobs)

**Region interaction:** Each region accumulates its own shadebob trails independently.

## Sprite Multiplexing

Reusing hardware sprites multiple times per frame by repositioning during scanline.

**C64/Amiga trick:** Hardware had limited sprites (8 on C64), but by changing sprite position mid-frame, you could display many more.

**Software equivalent:** Object pooling - reuse sprite structs for objects that leave the screen.

## Dot Balls / Dot Spheres

3D sphere made of dots, rotating in space.

**How it works:** Pre-calculate points on sphere surface. Each frame, rotate points in 3D, project to 2D, draw as pixels with depth-based brightness.

**Variations:**
- Multiple nested spheres
- Morphing between shapes (sphere → cube → torus)
- Connected dots (wireframe sphere)

## Sinus Dots

Points following sine wave patterns creating mesmerizing trails.

**How it works:** Array of dots, each following `y = sin(x + phase)`. Offset phase per dot for wave propagation effect.

**Variations:**
- Lissajous patterns (sine on both X and Y)
- 3D Lissajous with rotation
- Multiple interweaving waves
