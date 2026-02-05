use super::Region;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// A scene contains all mapped regions for a particular setup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub name: String,
    pub regions: Vec<Region>,
}

impl Scene {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            regions: Vec::new(),
        }
    }

    pub fn add_region(&mut self, region: Region) {
        self.regions.push(region);
    }

    pub fn remove_region(&mut self, index: usize) -> Option<Region> {
        if index < self.regions.len() {
            Some(self.regions.remove(index))
        } else {
            None
        }
    }

    /// Find which region contains a point (if any)
    pub fn region_at(&self, x: f32, y: f32) -> Option<&Region> {
        self.regions.iter().find(|r| r.contains(x, y))
    }

    /// Find all regions with a specific tag
    pub fn regions_with_tag(&self, tag: &str) -> Vec<&Region> {
        self.regions
            .iter()
            .filter(|r| r.tags.iter().any(|t| t == tag))
            .collect()
    }

    /// Save scene to a JSON file
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| e.to_string())
    }

    /// Load scene from a JSON file
    pub fn load(path: impl AsRef<Path>) -> Result<Self, String> {
        let json = fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&json).map_err(|e| e.to_string())
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new("untitled")
    }
}
