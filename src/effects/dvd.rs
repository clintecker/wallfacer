//! DVD Bounce Effect
//!
//! Classic DVD screensaver logo that bounces off screen edges
//! and user-defined polygon regions. Changes color on each bounce.

use super::Effect;
use crate::display::{draw_text_scaled, text_width_scaled, PixelBuffer, GLYPH_HEIGHT};
use crate::geometry::{rect_polygon_collision, reflect};
use crate::regions::Scene;
use crate::util::hsv_to_rgb;

const LOGO_TEXT: &str = "DVD";
const LOGO_SCALE: u32 = 4;

/// The DVD bounce effect
pub struct Dvd {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    hue: f32,
    logo_width: u32,
    logo_height: u32,
    trail: Vec<(f32, f32, f32)>, // (x, y, hue) for trail effect
}

impl Dvd {
    pub fn new() -> Self {
        let logo_width = text_width_scaled(LOGO_TEXT, LOGO_SCALE);
        let logo_height = GLYPH_HEIGHT * LOGO_SCALE;

        Self {
            x: 100.0,
            y: 100.0,
            vx: 120.0, // pixels per second
            vy: 80.0,
            hue: 0.0,
            logo_width,
            logo_height,
            trail: Vec::with_capacity(20),
        }
    }

    fn change_color(&mut self) {
        // Jump to a new hue on bounce
        self.hue = (self.hue + 45.0 + (self.vx.abs() * 0.5) as f32) % 360.0;
    }

    /// Check bounding box against all polygon regions
    /// Returns (normal_x, normal_y, push_distance) if collision detected
    fn check_polygon_collision(&self, scene: &Scene) -> Option<(f32, f32, f32)> {
        for region in &scene.regions {
            let verts = region.polygon.as_tuples();
            // Use full rectangle collision - handles diagonal edges properly
            if let Some((nx, ny, dist)) = rect_polygon_collision(
                self.x,
                self.y,
                self.logo_width as f32,
                self.logo_height as f32,
                &verts,
            ) {
                return Some((nx, ny, dist));
            }
        }

        None
    }
}

impl Default for Dvd {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Dvd {
    fn update(&mut self, dt: f32, width: u32, height: u32, scene: &Scene) {
        // Store trail position - only when moved enough distance (frame-rate independent)
        let min_dist = 8.0;
        let should_add = if let Some(&(lx, ly, _)) = self.trail.last() {
            (self.x - lx).powi(2) + (self.y - ly).powi(2) >= min_dist * min_dist
        } else {
            true
        };

        if should_add {
            if self.trail.len() >= 15 {
                self.trail.remove(0);
            }
            self.trail.push((self.x, self.y, self.hue));
        }

        // Move
        let new_x = self.x + self.vx * dt;
        let new_y = self.y + self.vy * dt;

        let mut bounced = false;

        // Screen edge collision
        let screen_w = width as f32;
        let screen_h = height as f32;

        if new_x <= 0.0 {
            self.x = 0.0;
            self.vx = self.vx.abs();
            bounced = true;
        } else if new_x + self.logo_width as f32 >= screen_w {
            self.x = screen_w - self.logo_width as f32;
            self.vx = -self.vx.abs();
            bounced = true;
        } else {
            self.x = new_x;
        }

        if new_y <= 0.0 {
            self.y = 0.0;
            self.vy = self.vy.abs();
            bounced = true;
        } else if new_y + self.logo_height as f32 >= screen_h {
            self.y = screen_h - self.logo_height as f32;
            self.vy = -self.vy.abs();
            bounced = true;
        } else {
            self.y = new_y;
        }

        // Polygon collision
        if let Some((nx, ny, dist)) = self.check_polygon_collision(scene) {
            let (new_vx, new_vy) = reflect(self.vx, self.vy, nx, ny);
            self.vx = new_vx;
            self.vy = new_vy;

            // Push out of polygon: distance to edge + margin
            let push_dist = dist + 10.0;
            self.x += nx * push_dist;
            self.y += ny * push_dist;

            bounced = true;
        }

        if bounced {
            self.change_color();
        }
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        // Dark background
        buffer.clear(0, 0, 0);

        // Draw trail (fading ghosts)
        for (i, &(tx, ty, thue)) in self.trail.iter().enumerate() {
            let alpha = (i as f32 / self.trail.len() as f32) * 0.3;
            let (r, g, b) = hsv_to_rgb(thue, 0.9, alpha);
            draw_text_scaled(buffer, tx as i32, ty as i32, LOGO_TEXT, r, g, b, LOGO_SCALE);
        }

        // Draw main logo
        let (r, g, b) = hsv_to_rgb(self.hue, 0.9, 1.0);

        // Draw a subtle glow/outline
        let (gr, gg, gb) = hsv_to_rgb(self.hue, 0.5, 0.4);
        draw_text_scaled(
            buffer,
            self.x as i32 - 1,
            self.y as i32,
            LOGO_TEXT,
            gr,
            gg,
            gb,
            LOGO_SCALE,
        );
        draw_text_scaled(
            buffer,
            self.x as i32 + 1,
            self.y as i32,
            LOGO_TEXT,
            gr,
            gg,
            gb,
            LOGO_SCALE,
        );
        draw_text_scaled(
            buffer,
            self.x as i32,
            self.y as i32 - 1,
            LOGO_TEXT,
            gr,
            gg,
            gb,
            LOGO_SCALE,
        );
        draw_text_scaled(
            buffer,
            self.x as i32,
            self.y as i32 + 1,
            LOGO_TEXT,
            gr,
            gg,
            gb,
            LOGO_SCALE,
        );

        // Redraw main on top for crisp edges
        draw_text_scaled(
            buffer,
            self.x as i32,
            self.y as i32,
            LOGO_TEXT,
            r,
            g,
            b,
            LOGO_SCALE,
        );
    }

    fn region_color(&self) -> (u8, u8, u8) {
        // Regions glow with the current DVD color at low intensity
        let (r, g, b) = hsv_to_rgb(self.hue, 0.8, 0.15);
        (r, g, b)
    }

    fn name(&self) -> &str {
        "DVD Bounce"
    }
}
