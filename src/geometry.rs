//! Geometry utilities for collision detection and vector math

/// Check if a point is inside a polygon using ray casting algorithm
pub fn point_in_polygon(px: f32, py: f32, vertices: &[(f32, f32)]) -> bool {
    let n = vertices.len();
    if n < 3 {
        return false;
    }

    let mut inside = false;
    let mut j = n - 1;

    for i in 0..n {
        let (xi, yi) = vertices[i];
        let (xj, yj) = vertices[j];

        // Skip horizontal edges (avoid division by zero)
        let dy = yj - yi;
        if dy.abs() > f32::EPSILON && ((yi > py) != (yj > py)) {
            let x_intersect = (xj - xi) * (py - yi) / dy + xi;
            if px < x_intersect {
                inside = !inside;
            }
        }
        j = i;
    }

    inside
}

/// Get the outward normal and distance for a point inside a polygon.
/// Returns (normal_x, normal_y, distance_to_closest_edge)
/// Handles sharp corners by using the corner bisector normal for stability.
pub fn polygon_escape_vector(px: f32, py: f32, vertices: &[(f32, f32)]) -> Option<(f32, f32, f32)> {
    let n = vertices.len();
    if n < 3 {
        return None;
    }

    // Calculate polygon centroid
    let (cx, cy) = vertices
        .iter()
        .fold((0.0, 0.0), |(sx, sy), &(x, y)| (sx + x, sy + y));
    let (cx, cy) = (cx / n as f32, cy / n as f32);

    let mut closest_dist = f32::MAX;
    let mut closest_point = (0.0_f32, 0.0_f32);
    let mut closest_edge_idx = 0;
    let mut closest_t = 0.0_f32; // Parameter along edge (0 = start vertex, len = end vertex)
    let mut closest_len = 1.0_f32;

    for i in 0..n {
        let (x1, y1) = vertices[i];
        let (x2, y2) = vertices[(i + 1) % n];

        // Edge vector
        let ex = x2 - x1;
        let ey = y2 - y1;
        let len = (ex * ex + ey * ey).sqrt();
        if len < 0.001 {
            continue;
        }

        // Normalized edge direction
        let dx = ex / len;
        let dy = ey / len;

        // Vector from edge start to point
        let px1 = px - x1;
        let py1 = py - y1;

        // Project point onto edge
        let t = (px1 * dx + py1 * dy).clamp(0.0, len);

        // Closest point on edge
        let edge_px = x1 + dx * t;
        let edge_py = y1 + dy * t;

        // Distance to edge
        let dist = ((px - edge_px) * (px - edge_px) + (py - edge_py) * (py - edge_py)).sqrt();

        if dist < closest_dist {
            closest_dist = dist;
            closest_point = (edge_px, edge_py);
            closest_edge_idx = i;
            closest_t = t;
            closest_len = len;
        }
    }

    if closest_dist < f32::MAX {
        // Check if we're near a vertex (corner) - use corner tolerance
        let corner_threshold = 3.0; // pixels
        let near_start = closest_t < corner_threshold;
        let near_end = closest_t > closest_len - corner_threshold;

        let (mut nx, mut ny) = if near_start || near_end {
            // Near a corner - use bisector of the two edges for stable escape direction
            let vertex_idx = if near_start {
                closest_edge_idx
            } else {
                (closest_edge_idx + 1) % n
            };

            // Get the two edges meeting at this vertex
            let prev_idx = (vertex_idx + n - 1) % n;
            let next_idx = (vertex_idx + 1) % n;

            let (vx, vy) = vertices[vertex_idx];
            let (prev_x, prev_y) = vertices[prev_idx];
            let (next_x, next_y) = vertices[next_idx];

            // Edge directions pointing away from vertex
            let e1x = prev_x - vx;
            let e1y = prev_y - vy;
            let e2x = next_x - vx;
            let e2y = next_y - vy;

            // Normalize
            let len1 = (e1x * e1x + e1y * e1y).sqrt();
            let len2 = (e2x * e2x + e2y * e2y).sqrt();

            if len1 > 0.001 && len2 > 0.001 {
                let e1x = e1x / len1;
                let e1y = e1y / len1;
                let e2x = e2x / len2;
                let e2y = e2y / len2;

                // Bisector is the sum of the two normalized edge directions
                // This points "into" the corner, so we negate for outward
                let bx = -(e1x + e2x);
                let by = -(e1y + e2y);
                let blen = (bx * bx + by * by).sqrt();

                if blen > 0.001 {
                    (bx / blen, by / blen)
                } else {
                    // Edges are parallel (180° corner), use perpendicular
                    (-e1y, e1x)
                }
            } else {
                // Fallback to point-based normal
                let dx = px - closest_point.0;
                let dy = py - closest_point.1;
                let len = (dx * dx + dy * dy).sqrt();
                if len > 0.001 {
                    (dx / len, dy / len)
                } else {
                    (1.0, 0.0)
                }
            }
        } else {
            // Not near a corner - use direction from closest point to test point
            let dx = px - closest_point.0;
            let dy = py - closest_point.1;
            let len = (dx * dx + dy * dy).sqrt();
            if len > 0.001 {
                (dx / len, dy / len)
            } else {
                (1.0, 0.0)
            }
        };

        // Make sure normal points away from centroid (outward)
        let to_center_x = cx - px;
        let to_center_y = cy - py;
        if nx * to_center_x + ny * to_center_y > 0.0 {
            nx = -nx;
            ny = -ny;
        }

        Some((nx, ny, closest_dist))
    } else {
        None
    }
}

/// Reflect a velocity vector against a surface normal
#[inline]
pub fn reflect(vx: f32, vy: f32, nx: f32, ny: f32) -> (f32, f32) {
    let dot = vx * nx + vy * ny;
    (vx - 2.0 * dot * nx, vy - 2.0 * dot * ny)
}

/// Normalize a 2D vector, returns (0, 0) if length is too small
#[inline]
pub fn normalize(x: f32, y: f32) -> (f32, f32) {
    let len = (x * x + y * y).sqrt();
    if len > 0.0001 {
        (x / len, y / len)
    } else {
        (0.0, 0.0)
    }
}

/// Calculate the length of a 2D vector
#[inline]
pub fn length(x: f32, y: f32) -> f32 {
    (x * x + y * y).sqrt()
}

/// Calculate squared distance between two points (avoids sqrt)
#[inline]
pub fn distance_squared(x1: f32, y1: f32, x2: f32, y2: f32) -> f32 {
    let dx = x2 - x1;
    let dy = y2 - y1;
    dx * dx + dy * dy
}

/// Check if two line segments intersect
/// Returns Some((x, y, t)) where (x,y) is intersection point and t is parameter along first segment
/// Returns None if segments don't intersect
pub fn segment_intersection(
    ax1: f32,
    ay1: f32,
    ax2: f32,
    ay2: f32, // First segment
    bx1: f32,
    by1: f32,
    bx2: f32,
    by2: f32, // Second segment
) -> Option<(f32, f32, f32)> {
    let dx1 = ax2 - ax1;
    let dy1 = ay2 - ay1;
    let dx2 = bx2 - bx1;
    let dy2 = by2 - by1;

    let cross = dx1 * dy2 - dy1 * dx2;

    // Parallel lines
    if cross.abs() < 0.0001 {
        return None;
    }

    let dx3 = bx1 - ax1;
    let dy3 = by1 - ay1;

    let t = (dx3 * dy2 - dy3 * dx2) / cross;
    let u = (dx3 * dy1 - dy3 * dx1) / cross;

    // Check if intersection is within both segments
    if t >= 0.0 && t <= 1.0 && u >= 0.0 && u <= 1.0 {
        let ix = ax1 + t * dx1;
        let iy = ay1 + t * dy1;
        Some((ix, iy, t))
    } else {
        None
    }
}

/// Check if a rectangle collides with a polygon
/// Returns the collision normal and penetration depth if collision detected
/// This checks both point-in-polygon AND edge intersections for robust detection
pub fn rect_polygon_collision(
    rx: f32,
    ry: f32, // Rectangle top-left
    rw: f32,
    rh: f32,                 // Rectangle size
    vertices: &[(f32, f32)], // Polygon vertices
) -> Option<(f32, f32, f32)> {
    let n = vertices.len();
    if n < 3 {
        return None;
    }

    // Rectangle corners
    let corners = [(rx, ry), (rx + rw, ry), (rx + rw, ry + rh), (rx, ry + rh)];

    // Rectangle edges (as pairs of corner indices)
    let rect_edges = [(0, 1), (1, 2), (2, 3), (3, 0)];

    // First check: any rectangle corner inside polygon?
    for &(px, py) in &corners {
        if point_in_polygon(px, py, vertices) {
            return polygon_escape_vector(px, py, vertices);
        }
    }

    // Second check: any polygon vertex inside rectangle?
    for &(vx, vy) in vertices {
        if vx >= rx && vx <= rx + rw && vy >= ry && vy <= ry + rh {
            // Polygon vertex is inside rectangle - find escape direction
            // Use the closest rectangle edge
            let to_left = vx - rx;
            let to_right = rx + rw - vx;
            let to_top = vy - ry;
            let to_bottom = ry + rh - vy;

            let min_dist = to_left.min(to_right).min(to_top).min(to_bottom);

            let (nx, ny) = if min_dist == to_left {
                (-1.0, 0.0)
            } else if min_dist == to_right {
                (1.0, 0.0)
            } else if min_dist == to_top {
                (0.0, -1.0)
            } else {
                (0.0, 1.0)
            };

            return Some((nx, ny, min_dist));
        }
    }

    // Third check: edge-edge intersections (handles diagonal cuts)
    let mut closest_intersection: Option<(f32, f32, f32, f32, f32, usize)> = None; // (ix, iy, dist, edge_nx, edge_ny, edge_idx)

    // Calculate polygon centroid once
    let (cx, cy) = vertices
        .iter()
        .fold((0.0, 0.0), |(sx, sy), &(x, y)| (sx + x, sy + y));
    let (cx, cy) = (cx / n as f32, cy / n as f32);

    for (i, j) in rect_edges {
        let (rx1, ry1) = corners[i];
        let (rx2, ry2) = corners[j];

        for k in 0..n {
            let (px1, py1) = vertices[k];
            let (px2, py2) = vertices[(k + 1) % n];

            if let Some((ix, iy, _t)) = segment_intersection(rx1, ry1, rx2, ry2, px1, py1, px2, py2)
            {
                // Distance from rectangle center to intersection
                let rect_cx = rx + rw / 2.0;
                let rect_cy = ry + rh / 2.0;
                let dist =
                    ((ix - rect_cx) * (ix - rect_cx) + (iy - rect_cy) * (iy - rect_cy)).sqrt();

                // Calculate normal perpendicular to polygon edge
                let edge_dx = px2 - px1;
                let edge_dy = py2 - py1;
                let edge_len = (edge_dx * edge_dx + edge_dy * edge_dy).sqrt();

                if edge_len < 0.001 {
                    continue;
                }

                // Check if intersection is near a corner
                let corner_threshold = 5.0;
                let dist_to_start = ((ix - px1) * (ix - px1) + (iy - py1) * (iy - py1)).sqrt();
                let dist_to_end = ((ix - px2) * (ix - px2) + (iy - py2) * (iy - py2)).sqrt();
                let near_corner =
                    dist_to_start < corner_threshold || dist_to_end < corner_threshold;

                let (mut nx, mut ny) = if near_corner {
                    // Near a corner - use bisector for stable normal
                    let vertex_idx = if dist_to_start < dist_to_end {
                        k
                    } else {
                        (k + 1) % n
                    };
                    let prev_idx = (vertex_idx + n - 1) % n;
                    let next_idx = (vertex_idx + 1) % n;

                    let (vx, vy) = vertices[vertex_idx];
                    let (prev_x, prev_y) = vertices[prev_idx];
                    let (next_x, next_y) = vertices[next_idx];

                    // Edge directions pointing away from vertex
                    let e1x = prev_x - vx;
                    let e1y = prev_y - vy;
                    let e2x = next_x - vx;
                    let e2y = next_y - vy;

                    let len1 = (e1x * e1x + e1y * e1y).sqrt();
                    let len2 = (e2x * e2x + e2y * e2y).sqrt();

                    if len1 > 0.001 && len2 > 0.001 {
                        let e1x = e1x / len1;
                        let e1y = e1y / len1;
                        let e2x = e2x / len2;
                        let e2y = e2y / len2;

                        // Bisector points into corner, negate for outward
                        let bx = -(e1x + e2x);
                        let by = -(e1y + e2y);
                        let blen = (bx * bx + by * by).sqrt();

                        if blen > 0.001 {
                            (bx / blen, by / blen)
                        } else {
                            (-edge_dy / edge_len, edge_dx / edge_len)
                        }
                    } else {
                        (-edge_dy / edge_len, edge_dx / edge_len)
                    }
                } else {
                    // Normal is perpendicular to edge
                    (-edge_dy / edge_len, edge_dx / edge_len)
                };

                // Make sure normal points away from polygon center
                let to_center_x = cx - ix;
                let to_center_y = cy - iy;
                if nx * to_center_x + ny * to_center_y > 0.0 {
                    nx = -nx;
                    ny = -ny;
                }

                if closest_intersection.is_none() || dist < closest_intersection.unwrap().2 {
                    closest_intersection = Some((ix, iy, dist, nx, ny, k));
                }
            }
        }
    }

    if let Some((_ix, _iy, _dist, nx, ny, _edge_idx)) = closest_intersection {
        // Estimate penetration depth based on rectangle size and normal direction
        let penetration = if nx.abs() > ny.abs() {
            rw / 2.0
        } else {
            rh / 2.0
        };
        return Some((nx, ny, penetration * 0.5));
    }

    None
}

/// Check if a circle collides with a polygon.
/// Returns (normal_x, normal_y, penetration_depth) pointing from polygon toward circle center.
/// Penetration depth is exactly how far the circle overlaps — pushing by this amount resolves it.
pub fn circle_polygon_collision(
    cx: f32,
    cy: f32,
    radius: f32,
    vertices: &[(f32, f32)],
) -> Option<(f32, f32, f32)> {
    let n = vertices.len();
    if n < 3 {
        return None;
    }

    // If circle center is inside the polygon, use escape vector
    if point_in_polygon(cx, cy, vertices) {
        if let Some((nx, ny, dist)) = polygon_escape_vector(cx, cy, vertices) {
            return Some((nx, ny, dist + radius));
        }
    }

    // Find closest point on any polygon edge to circle center
    let mut closest_dist = f32::MAX;
    let mut closest_nx = 0.0_f32;
    let mut closest_ny = 0.0_f32;

    for i in 0..n {
        let (x1, y1) = vertices[i];
        let (x2, y2) = vertices[(i + 1) % n];

        let ex = x2 - x1;
        let ey = y2 - y1;
        let len_sq = ex * ex + ey * ey;
        if len_sq < 0.001 {
            continue;
        }

        // Project center onto edge segment, clamped
        let t = ((cx - x1) * ex + (cy - y1) * ey) / len_sq;
        let t = t.clamp(0.0, 1.0);

        let near_x = x1 + t * ex;
        let near_y = y1 + t * ey;

        let dx = cx - near_x;
        let dy = cy - near_y;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist < closest_dist {
            closest_dist = dist;
            if dist > 0.001 {
                closest_nx = dx / dist;
                closest_ny = dy / dist;
            }
        }
    }

    if closest_dist < radius {
        Some((closest_nx, closest_ny, radius - closest_dist))
    } else {
        None
    }
}

/// Check if two circles collide.
/// Returns (normal_x, normal_y, penetration_depth) pointing from circle2 toward circle1.
pub fn circle_circle_collision(
    cx1: f32,
    cy1: f32,
    r1: f32,
    cx2: f32,
    cy2: f32,
    r2: f32,
) -> Option<(f32, f32, f32)> {
    let dx = cx1 - cx2;
    let dy = cy1 - cy2;
    let dist = (dx * dx + dy * dy).sqrt();
    let min_dist = r1 + r2;

    if dist < min_dist && dist > 0.001 {
        let nx = dx / dist;
        let ny = dy / dist;
        let penetration = min_dist - dist;
        Some((nx, ny, penetration))
    } else {
        None
    }
}
