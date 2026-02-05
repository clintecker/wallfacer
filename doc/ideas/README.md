# Demoscene Effect Ideas

Visual effects for Wallfacer's projection mapping system, inspired by the demoscene and 90s rave culture.

## Categories

- [Classics](classics.md) - Raster bars, scrollers, bobs, tunnels
- [Sprites & Bobs](sprites-bobs.md) - Bouncing objects, shadebobs, multiplexing tricks
- [Geometry-Driven](geometry-driven.md) - Effects using region shape/edges
- [3D Vector](3d-vector.md) - Rotating objects, glenz, morphing
- [Distortion](distortion.md) - Rotozoomer, warp, rubber effects
- [Cross-Region](cross-region.md) - Effects spanning multiple regions
- [Text & Typography](text.md) - Scrollers and text effects
- [Metaballs & Blobs](metaballs.md) - Organic blobby shapes
- [Fractals & Psychedelic](psychedelic.md) - Trippy effects, fractals
- [Audio-Reactive](audio-reactive.md) - Future sound-driven effects

## Implementation

- [New Primitives Required](new-primitives.md) - Minimal set of new drawing primitives
- [Full Primitives Analysis](primitives-analysis.md) - Detailed breakdown by effect

## Research Sources

- [Demo effect - Wikipedia](https://en.wikipedia.org/wiki/Demo_effect)
- [Pouët.net oldschool effects list](https://www.pouet.net/topic.php?which=7523&page=3)
- [Oldschool demo effects - seancode](https://seancode.com/demofx/)
- [The Demo Effects Collection](https://demo-effects.sourceforge.net/)
- [Realtime Visualization Methods in the Demoscene](https://old.cescg.org/CESCG-2002/BBurger/index.html)
- [90s Rave VJ history - Vice](https://www.vice.com/en/article/yp5x8j/trippin-down-memory-lane-with-a-90s-rave-vj)

## Implementation Priority

**High impact, approachable:**
1. Copper/Raster Bars - iconic, simple palette animation
2. Glenz vectors - transparent 3D objects
3. Rotozoomer per-region - each surface as animated texture
4. Bobs with polygon collision - sprites confined to regions
5. Edge glow/pulse - shows off region shapes

**Medium complexity:**
- Metaballs spanning regions
- Shadebobs with color accumulation
- Twister effect
- Voronoi from region centroids

**Advanced:**
- Real-time morphing between 3D shapes
- Marching cubes isosurfaces
- Video feedback loops
