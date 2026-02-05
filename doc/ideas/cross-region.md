# Cross-Region Effects

Effects that create relationships between multiple regions.

## Lightning / Electricity

Procedural lightning arcs between nearby regions.

**How it works:**
- Find pairs of nearby regions (by centroid distance)
- Generate jagged line between them (midpoint displacement)
- Animate with flickering intensity
- Add glow/bloom

**Variations:**
- Connect centroids
- Connect nearest edge points
- Chain lightning through multiple regions

## Particle Streams

Particles flowing from region to region.

**How it works:**
- Tag regions as "source" or "sink"
- Emit particles from source centroids
- Attract toward sink centroids
- Follow bezier curves between them

**Variations:**
- Fireflies/sparks
- Liquid flow
- Data stream visualization

## Infection / Contagion

One effect spreads from region to region over time.

**How it works:**
- Start with one "infected" region
- Each frame, nearby regions have chance to become infected
- Infected regions change effect (color, pattern)

**Creates dramatic reveals across the projection surface.**

## Synchronized Pulse

All regions pulse together in patterns.

**How it works:**
- Global phase variable
- Each region flashes based on phase
- Sequential ordering creates wave across surface

**Variations:**
- All simultaneous
- Distance-based delay from trigger point
- Chase pattern (1, 2, 3, 4, 1, 2...)

## Portal Effect

One region shows content from another region's perspective.

**How it works:**
- Render scene into texture
- Map texture into destination region
- Offset/transform as if looking through portal

## Tetris / Stacking

Shapes falling and stacking within regions.

**How it works:**
- Falling blocks within each region
- Physics-based stacking
- Blocks confined to polygon boundaries

## Network Visualization

Regions as nodes, connections as edges.

**How it works:**
- Draw lines between region centroids
- Animate pulses traveling along connections
- Thicker lines for "stronger" connections
