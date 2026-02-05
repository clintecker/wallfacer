# 3D Vector Graphics

Rotating 3D objects rendered in real-time - a major flex in the pre-GPU era.

## Wireframe Vectors

3D objects drawn as connected lines.

**Evolution:**
1. Dots only (just vertices)
2. Edges (connected lines)
3. Hidden line removal
4. Filled polygons (flat shaded)
5. Gouraud shading (smooth color interpolation)
6. Phong shading (per-pixel lighting)

## Glenz Vectors

Transparent, glass-like 3D objects with an additive "diamond" look.

**How it works:** Draw polygons with additive blending. Overlapping faces create brighter areas, giving crystalline appearance.

**Named after:** Swedish word "gläns" (glisten/glitter). First seen in Kefrens' "Megademo 8" by Promax.

**Region interaction:** Different regions render different glenz objects, overlapping at boundaries.

## Rubber / Jelly Objects

Elastic, wobbly 3D shapes that deform organically.

**How it works:** Apply sine-based displacement to vertices. Offset phase by vertex index for wave propagation through the mesh.

**Variations:**
- Rubber cube
- Jelly sphere
- Flag waving simulation

## Vector Balls

Spheres rendered at each vertex of a 3D object.

**Classic look:** Chrome/metallic spheres with environment mapping, arranged as cube vertices or more complex polyhedra.

**Region interaction:** Render vector balls at polygon vertices of regions themselves.

## 3D Morphing

Smooth transformation between two different 3D shapes.

**How it works:** Both source and target meshes need same vertex count. Interpolate vertex positions over time: `pos = lerp(source, target, t)`.

**Classic morphs:**
- Cube → Sphere
- Logo → Logo
- Face → Skull

## Dot Tunnel / Starfield

Points flying toward or away from the viewer.

**How it works:** 3D points with Z velocity. Project to 2D with perspective. Recycle points that pass the camera.

**Variations:**
- Colored by depth
- Trails for motion blur
- Hyperspace effect (extreme speed)
