use super::Point;
use serde::{Deserialize, Serialize};

/// A simple polygon defined by vertices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Polygon {
    pub vertices: Vec<Point>,
}

impl Polygon {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
        }
    }

    pub fn from_vertices(vertices: Vec<Point>) -> Self {
        Self { vertices }
    }

    pub fn add_vertex(&mut self, x: f32, y: f32) {
        self.vertices.push(Point::new(x, y));
    }

    pub fn is_closed(&self) -> bool {
        self.vertices.len() >= 3
    }

    /// Point-in-polygon test using ray casting algorithm
    pub fn contains(&self, x: f32, y: f32) -> bool {
        if self.vertices.len() < 3 {
            return false;
        }

        let mut inside = false;
        let n = self.vertices.len();

        let mut j = n - 1;
        for i in 0..n {
            let vi = &self.vertices[i];
            let vj = &self.vertices[j];

            if ((vi.y > y) != (vj.y > y)) && (x < (vj.x - vi.x) * (y - vi.y) / (vj.y - vi.y) + vi.x)
            {
                inside = !inside;
            }
            j = i;
        }

        inside
    }

    /// Get the bounding box (min_x, min_y, max_x, max_y)
    pub fn bounds(&self) -> Option<(f32, f32, f32, f32)> {
        if self.vertices.is_empty() {
            return None;
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for v in &self.vertices {
            min_x = min_x.min(v.x);
            min_y = min_y.min(v.y);
            max_x = max_x.max(v.x);
            max_y = max_y.max(v.y);
        }

        Some((min_x, min_y, max_x, max_y))
    }

    /// Get edges as line segments
    pub fn edges(&self) -> impl Iterator<Item = (&Point, &Point)> {
        let n = self.vertices.len();
        (0..n).map(move |i| (&self.vertices[i], &self.vertices[(i + 1) % n]))
    }

    /// Convert vertices to tuple format for fill_polygon API
    pub fn as_tuples(&self) -> Vec<(f32, f32)> {
        self.vertices.iter().map(|v| (v.x, v.y)).collect()
    }

    /// Calculate the centroid (geometric center) of the polygon
    /// Returns None if the polygon is empty
    pub fn centroid(&self) -> Option<Point> {
        if self.vertices.is_empty() {
            return None;
        }

        let n = self.vertices.len() as f32;
        let sum_x: f32 = self.vertices.iter().map(|v| v.x).sum();
        let sum_y: f32 = self.vertices.iter().map(|v| v.y).sum();

        Some(Point::new(sum_x / n, sum_y / n))
    }
}

impl Default for Polygon {
    fn default() -> Self {
        Self::new()
    }
}
