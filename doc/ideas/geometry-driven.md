# Geometry-Driven Effects

Effects that leverage region shape, edges, and vertices.

## Edge Glow / Border Pulse

Animated neon glow tracing region outlines.

**How it works:**
- Calculate distance from each pixel to nearest edge
- Apply glow falloff based on distance
- Animate hue or intensity over time

**Variations:**
- Chaser lights (pulse traveling around perimeter)
- Breathing glow (synchronized pulse)
- Multi-color gradient around edge

## Polygon Morphing

Regions visually breathing - vertices animate inward/outward.

**How it works:**
- Calculate centroid of polygon
- Offset vertices toward/away from centroid over time
- Render with interpolated positions

**Note:** This is visual only - doesn't change the actual mask.

## Voronoi Diagrams

Organic cell patterns seeded from region centroids.

**How it works:**
- Each region centroid is a Voronoi seed
- For each pixel, find nearest seed
- Color by seed identity or distance

**Variations:**
- Animated seeds (slowly drifting)
- Distance-based gradient coloring
- Cell borders highlighted

## Shockwave Ripples

Rings emanating from region centers.

**How it works:**
- Calculate distance from centroid
- Apply sine wave based on distance minus time
- Modulate brightness or displacement

**Interactions:** Waves from different regions interfere when overlapping.

## Vector Balls on Vertices

Metallic spheres at each polygon vertex.

**How it works:**
- Extract vertex positions from region polygons
- Render shaded sphere at each vertex
- Environment mapping for chrome look

**Classic 3D demo aesthetic using actual region geometry.**

## Laser Grid

Grid lines that follow polygon edges.

**How it works:**
- Draw bright lines along each edge
- Add glow/bloom effect
- Animate with traveling pulses

**Very Tron aesthetic.**

## Fill Patterns

Procedural patterns that respect region bounds.

**Types:**
- Scanline fills (alternating lines)
- Crosshatch
- Dot matrix
- Animated static/noise

Each region could have different fill patterns.
