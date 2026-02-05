mod polygon;
mod scene;

pub use polygon::Polygon;
pub use scene::Scene;

use serde::{Deserialize, Serialize};

/// A point in 2D space
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn distance_to(&self, other: &Point) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// A circle defined by center and radius
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Circle {
    pub center: Point,
    pub radius: f32,
}

impl Circle {
    pub fn new(center: Point, radius: f32) -> Self {
        Self { center, radius }
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        let dx = x - self.center.x;
        let dy = y - self.center.y;
        dx * dx + dy * dy <= self.radius * self.radius
    }

    pub fn bounds(&self) -> (f32, f32, f32, f32) {
        (
            self.center.x - self.radius,
            self.center.y - self.radius,
            self.center.x + self.radius,
            self.center.y + self.radius,
        )
    }

    pub fn centroid(&self) -> Point {
        self.center
    }
}

/// A shape that can be either a polygon or a circle
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Shape {
    Polygon(Polygon),
    Circle(Circle),
}

impl Shape {
    pub fn contains(&self, x: f32, y: f32) -> bool {
        match self {
            Shape::Polygon(p) => p.contains(x, y),
            Shape::Circle(c) => c.contains(x, y),
        }
    }

    pub fn bounds(&self) -> Option<(f32, f32, f32, f32)> {
        match self {
            Shape::Polygon(p) => p.bounds(),
            Shape::Circle(c) => Some(c.bounds()),
        }
    }

    pub fn centroid(&self) -> Option<Point> {
        match self {
            Shape::Polygon(p) => p.centroid(),
            Shape::Circle(c) => Some(c.centroid()),
        }
    }

    /// Get as polygon reference (for backwards compatibility)
    pub fn as_polygon(&self) -> Option<&Polygon> {
        match self {
            Shape::Polygon(p) => Some(p),
            Shape::Circle(_) => None,
        }
    }

    /// Get as circle reference
    pub fn as_circle(&self) -> Option<&Circle> {
        match self {
            Shape::Circle(c) => Some(c),
            Shape::Polygon(_) => None,
        }
    }
}

/// A named region that maps to a real-world object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    pub name: String,
    /// Legacy field for backwards compatibility with old scene files
    #[serde(default, skip_serializing_if = "Option::is_none")]
    polygon: Option<Polygon>,
    /// New shape field - preferred
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<Shape>,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Region {
    pub fn new(name: impl Into<String>, polygon: Polygon) -> Self {
        Self {
            name: name.into(),
            polygon: None,
            shape: Some(Shape::Polygon(polygon)),
            tags: Vec::new(),
        }
    }

    pub fn new_circle(name: impl Into<String>, circle: Circle) -> Self {
        Self {
            name: name.into(),
            polygon: None,
            shape: Some(Shape::Circle(circle)),
            tags: Vec::new(),
        }
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        self.get_shape().contains(x, y)
    }

    /// Get the shape, handling legacy polygon field
    pub fn get_shape(&self) -> &Shape {
        // Prefer shape field, fallback to legacy polygon
        if let Some(ref shape) = self.shape {
            shape
        } else if let Some(ref _poly) = self.polygon {
            // This is a hack for the borrow checker - we can't return a reference
            // to a temporary. In practice this branch shouldn't be hit after
            // the first access triggers migration.
            panic!("Legacy polygon field without shape - call migrate_legacy() first")
        } else {
            panic!("Region has neither shape nor polygon")
        }
    }

    /// Get mutable access to the underlying polygon (if it is one)
    pub fn polygon_mut(&mut self) -> Option<&mut Polygon> {
        if let Some(Shape::Polygon(ref mut p)) = self.shape {
            Some(p)
        } else {
            None
        }
    }

    /// Get mutable access to the underlying circle (if it is one)
    pub fn circle_mut(&mut self) -> Option<&mut Circle> {
        if let Some(Shape::Circle(ref mut c)) = self.shape {
            Some(c)
        } else {
            None
        }
    }

    /// Check if this region is a circle
    pub fn is_circle(&self) -> bool {
        matches!(self.shape, Some(Shape::Circle(_)))
    }

    /// Check if this region is a polygon
    pub fn is_polygon(&self) -> bool {
        matches!(self.shape, Some(Shape::Polygon(_))) || self.polygon.is_some()
    }

    /// Migrate legacy polygon field to shape field
    pub fn migrate_legacy(&mut self) {
        if self.shape.is_none() {
            if let Some(poly) = self.polygon.take() {
                self.shape = Some(Shape::Polygon(poly));
            }
        }
    }
}

// Legacy accessor for backwards compatibility
impl Region {
    /// Get polygon reference (panics if region is a circle)
    /// Prefer using get_shape() and matching on the Shape enum
    pub fn polygon(&self) -> &Polygon {
        match &self.shape {
            Some(Shape::Polygon(p)) => p,
            Some(Shape::Circle(_)) => panic!("Called polygon() on a circle region"),
            None => self.polygon.as_ref().expect("No shape or polygon"),
        }
    }
}
