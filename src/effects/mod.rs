mod bobs;
mod copper_bars;
mod dot_tunnel;
mod dvd;
mod earth;
mod earth2;
mod ethereal_ink;
mod fire;
mod glenz;
mod julia;
mod lightning;
mod living_wall;
mod mandelbrot;
mod pipes;
mod plasma;
mod raycaster;
mod ripples;
mod rotozoomer;
mod rubber;
mod scroller_demo;
mod snowfall;
mod starfield;
mod testpattern;
mod text_fx_demo;
mod tunnel;
mod vector_balls;
mod vines;
mod worms;

pub use bobs::Bobs;
pub use copper_bars::CopperBars;
pub use dot_tunnel::DotTunnel;
pub use dvd::Dvd;
pub use earth::Earth;
pub use earth2::Earth2;
pub use ethereal_ink::EtherealInk;
pub use fire::Fire;
pub use glenz::Glenz;
pub use julia::Julia;
pub use lightning::LightningStorm;
pub use living_wall::LivingWall;
pub use mandelbrot::Mandelbrot;
pub use pipes::Pipes;
pub use plasma::Plasma;
pub use raycaster::Raycaster;
pub use ripples::Ripples;
pub use rotozoomer::Rotozoomer;
pub use rubber::Rubber;
pub use scroller_demo::ScrollerDemo;
pub use snowfall::Snowfall;
pub use starfield::Starfield;
pub use testpattern::TestPattern;
pub use text_fx_demo::TextFxDemo;
pub use tunnel::Tunnel;
pub use vector_balls::VectorBalls;
pub use vines::Vines;
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
