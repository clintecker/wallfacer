mod bobs;
mod copper_bars;
mod dvd;
mod earth;
mod earth2;
mod fire;
mod glenz;
mod plasma;
mod rotozoomer;
mod scroller_demo;
mod starfield;
mod testpattern;
mod text_fx_demo;
mod tunnel;
mod worms;

pub use bobs::Bobs;
pub use copper_bars::CopperBars;
pub use dvd::Dvd;
pub use earth::Earth;
pub use earth2::Earth2;
pub use fire::Fire;
pub use glenz::Glenz;
pub use plasma::Plasma;
pub use rotozoomer::Rotozoomer;
pub use scroller_demo::ScrollerDemo;
pub use starfield::Starfield;
pub use testpattern::TestPattern;
pub use text_fx_demo::TextFxDemo;
pub use tunnel::Tunnel;
pub use worms::Worms;

use crate::display::PixelBuffer;
use crate::regions::Scene;

/// Trait for all demoscene-style effects
pub trait Effect {
    /// Update effect state (called each frame)
    /// - dt: delta time in seconds
    /// - width/height: buffer dimensions for collision detection
    /// - scene: user-defined regions
    fn update(&mut self, dt: f32, width: u32, height: u32, scene: &Scene);

    /// Render effect to the pixel buffer
    fn render(&self, buffer: &mut PixelBuffer);

    /// Effect name for UI/debugging
    fn name(&self) -> &str;

    /// Color to use for masking user-defined regions (default: black)
    fn region_color(&self) -> (u8, u8, u8) {
        (0, 0, 0)
    }
}

/// Color utilities for effects
pub mod color {
    use crate::util::hsv_to_rgb;

    /// Grayscale color (same value for R, G, B)
    #[inline]
    pub fn gray(v: u8) -> (u8, u8, u8) {
        (v, v, v)
    }

    /// Create a classic demoscene color palette
    pub fn make_palette(size: usize) -> Vec<(u8, u8, u8)> {
        (0..size)
            .map(|i| {
                let t = i as f32 / size as f32;
                hsv_to_rgb(t * 360.0, 0.8, 0.9)
            })
            .collect()
    }

    /// Fire palette (black -> red -> orange -> yellow -> white)
    pub fn fire_palette() -> Vec<(u8, u8, u8)> {
        let mut palette = Vec::with_capacity(256);
        for i in 0..256 {
            let t = i as f32 / 255.0;
            let r = (t * 3.0).min(1.0);
            let g = ((t - 0.33) * 3.0).clamp(0.0, 1.0);
            let b = ((t - 0.66) * 3.0).clamp(0.0, 1.0);
            palette.push(((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8));
        }
        palette
    }
}
