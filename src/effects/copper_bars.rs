//! Copper Bars (Raster Bars) Effect
//!
//! Horizontal colored bars that animate across the screen,
//! named after the Amiga's Copper coprocessor.

use super::Effect;
use crate::display::PixelBuffer;
use crate::regions::Scene;
use crate::util::hsv_to_rgb;
use std::f32::consts::TAU;

/// Number of bars in the effect
const NUM_BARS: usize = 8;

/// Base height of each bar in pixels (designed for 480p)
const BASE_BAR_HEIGHT: f32 = 40.0;

/// A single animated copper bar
struct Bar {
    phase: f32,     // Phase offset for sine wave
    speed: f32,     // Animation speed multiplier
    hue: f32,       // Base hue (0-360)
    amplitude: f32, // Vertical movement range
}

/// Classic Amiga-style raster bars
pub struct CopperBars {
    time: f32,
    bars: Vec<Bar>,
}

impl CopperBars {
    pub fn new() -> Self {
        let bars = (0..NUM_BARS)
            .map(|i| {
                let t = i as f32 / NUM_BARS as f32;
                Bar {
                    phase: t * TAU,
                    speed: 1.0 + t * 0.5,
                    hue: t * 360.0,
                    amplitude: 0.35 + t * 0.05,
                }
            })
            .collect();

        Self { time: 0.0, bars }
    }

    /// Draw a single bar with gradient shading
    fn draw_bar(
        &self,
        buffer: &mut PixelBuffer,
        y_center: i32,
        hue: f32,
        alpha: u8,
        bar_height: i32,
    ) {
        let width = buffer.width() as i32;
        let half_height = bar_height / 2;

        for dy in -half_height..=half_height {
            let y = y_center + dy;
            if y < 0 || y >= buffer.height() as i32 {
                continue;
            }

            // Gradient: brightest at center, darker at edges
            let edge_dist = (dy.abs() as f32) / half_height as f32;
            let brightness = 1.0 - edge_dist * 0.6;
            let saturation = 0.7 + edge_dist * 0.3;

            let (r, g, b) = hsv_to_rgb(hue, saturation, brightness);

            // Apply alpha for overlapping effect
            let r = (r as u16 * alpha as u16 / 255) as u8;
            let g = (g as u16 * alpha as u16 / 255) as u8;
            let b = (b as u16 * alpha as u16 / 255) as u8;

            buffer.hline_blend(0, width - 1, y, r, g, b, alpha);
        }
    }
}

impl Default for CopperBars {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for CopperBars {
    fn update(&mut self, dt: f32, _width: u32, _height: u32, _scene: &Scene) {
        self.time += dt;
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        let height = buffer.height() as f32;
        let bar_height = (BASE_BAR_HEIGHT * height / 480.0).round().max(4.0) as i32;

        // Clear to dark background
        buffer.clear(16, 8, 32);

        // Draw bars back-to-front for proper blending
        for bar in &self.bars {
            // Sine wave vertical position
            let wave = (self.time * bar.speed + bar.phase).sin();
            let y_center = (height * 0.5 + wave * height * bar.amplitude) as i32;

            // Animate hue over time
            let hue = (bar.hue + self.time * 30.0) % 360.0;

            self.draw_bar(buffer, y_center, hue, 180, bar_height);
        }
    }

    fn name(&self) -> &str {
        "Copper Bars"
    }

    fn region_color(&self) -> (u8, u8, u8) {
        (16, 8, 32) // Match background
    }
}
