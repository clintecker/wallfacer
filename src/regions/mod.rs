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

/// A named region that maps to a real-world object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    pub name: String,
    pub polygon: Polygon,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Region {
    pub fn new(name: impl Into<String>, polygon: Polygon) -> Self {
        Self {
            name: name.into(),
            polygon,
            tags: Vec::new(),
        }
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        self.polygon.contains(x, y)
    }
}
