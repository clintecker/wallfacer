# Classic Demoscene Effects

Foundational effects from the Amiga/C64/Atari ST era.

## Raster Bars (Copper Bars)

Horizontal colored bars that animate across the screen. Named after the Amiga's Copper coprocessor.

**How it works:** Change background color between scanlines during the raster beam sweep. Create smooth gradients by interpolating colors per-scanline.

**Variations:**
- Sine-wave movement (bars follow sine path)
- Overlapping transparent bars
- Bars that bend around obstacles
- Kefrens/Keftale bars (vertical variant using 1-line bobs)

**Region interaction:** Bars could warp or split when crossing region boundaries.

## Scrollers

Text sliding across the screen - the "hello world" of demos.

**Types:**
- Horizontal scroller (left-to-right or right-to-left)
- Sine scroller (wavy vertical displacement)
- DYCP (Different Y Character Position) - each character at different height
- Parallax scroller (multiple layers at different speeds)
- Rubber/elastic scroller (stretchy text)

**Region interaction:** Different regions display different messages or scroll directions.

## Plasma

Shifting display of colors using overlapping sine waves.

**How it works:** For each pixel, sum several sine functions with different frequencies/phases, map result to color palette. Animate by varying phase over time.

**Variations:**
- Classic plasma (sum of sines)
- Interference patterns
- Radial plasma from center points

**Region interaction:** Each region's centroid becomes a plasma origin point.

## Fire Effect

Cellular automaton simulating rising flames.

**How it works:**
1. Seed random hot pixels at bottom
2. For each pixel, average neighbors above
3. Subtract cooling factor
4. Map heat values to fire palette (black→red→orange→yellow→white)

**Variations:**
- Wind effect (horizontal drift)
- Multiple ignition points
- Colored fire (blue, green)

## Tunnel

Flying through an infinite tunnel toward a vanishing point.

**How it works:** For each pixel, calculate angle and distance from center. Use these as texture coordinates, animate by offsetting over time.

**Variations:**
- Textured tunnel walls
- Square/hexagonal tunnel cross-sections
- Twisted tunnel (rotation varies with depth)

**Region interaction:** Each region becomes a portal into its own tunnel.
