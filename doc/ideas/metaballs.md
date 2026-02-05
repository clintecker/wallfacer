# Metaballs & Blobs

Organic, blobby shapes that merge together - an old demoscene favorite.

## 2D Metaballs

Circular influence fields that blend smoothly when close.

**How it works:**
1. Each metaball has position and radius
2. For each pixel, sum influence from all balls: `sum += radius² / distance²`
3. If sum > threshold, pixel is "inside" the blob
4. Color based on sum value for smooth gradients

**Variations:**
- Solid threshold (hard edge)
- Gradient coloring (glow effect)
- Multiple color channels (RGB metaballs)

**Region interaction:** Metaballs confined to regions, merging at region boundaries.

## 3D Metaballs (Marching Cubes)

Volumetric blobs rendered as 3D surfaces.

**How it works:**
1. Evaluate density field on 3D grid
2. Use marching cubes algorithm to extract isosurface
3. Render resulting mesh with lighting

**More expensive but impressive 3D blobby objects.**

## Lava Lamp Effect

Slowly rising and falling blobs, splitting and merging.

**How it works:** Metaballs with simple physics - buoyancy, drag, occasional random velocity changes.

## Electric Plasma Balls

Metaball-style effect with electric/plasma coloring.

**How it works:** Use metaball field but color with electric palette (black → purple → blue → white). Add noise for crackling effect.

## Blobby Text

Text where each letter is a metaball that morphs into position.

**How it works:** Target positions from font glyphs. Metaballs animate from random positions to letter positions, merging during transition.

---

## Technical Notes

**Performance:** 2D metaballs are O(pixels × balls). Optimize with:
- Bounding box culling
- Lower resolution field, upscale result
- GPU shader implementation

**Classic reference:** Effect created by Jim Blinn for Carl Sagan's "Cosmos" TV series.
