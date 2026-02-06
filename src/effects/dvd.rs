//! DVD Bounce Effect
//!
//! Classic DVD screensaver logo that bounces off screen edges
//! and user-defined polygon regions. Changes color on each bounce.

use super::Effect;
use crate::display::{draw_text_scaled, text_width_scaled, PixelBuffer, GLYPH_HEIGHT};
use crate::geometry::{rect_polygon_collision, reflect};
use crate::regions::{Scene, Shape};
use crate::util::hsv_to_rgb;

const LOGO_TEXT: &str = "2389";
const BASE_LOGO_SCALE: u32 = 4;

/// The DVD bounce effect
pub struct Dvd {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    hue: f32,
    logo_width: u32,
    logo_height: u32,
    logo_scale: u32,
    trail: Vec<(f32, f32, f32)>, // (x, y, hue) for trail effect
    // Anti-stuck: track rapid region bounces
    region_bounce_cooldown: f32,  // Time until we allow region collision again
    rapid_bounce_count: u32,      // Count of rapid bounces (resets when cooldown expires)
}

impl Dvd {
    pub fn new() -> Self {
        let logo_scale = BASE_LOGO_SCALE;
        let logo_width = text_width_scaled(LOGO_TEXT, logo_scale);
        let logo_height = GLYPH_HEIGHT * logo_scale;

        Self {
            x: 100.0,
            y: 100.0,
            vx: 120.0, // pixels per second (base speed at 640x480)
            vy: 80.0,
            hue: 0.0,
            logo_width,
            logo_height,
            logo_scale,
            trail: Vec::with_capacity(20),
            region_bounce_cooldown: 0.0,
            rapid_bounce_count: 0,
        }
    }

    fn change_color(&mut self) {
        // Jump to a new hue on bounce
        self.hue = (self.hue + 45.0 + (self.vx.abs() * 0.5) as f32) % 360.0;
    }

    /// Check bounding box against all regions (polygons and circles)
    /// Returns (normal_x, normal_y, push_distance) if collision detected
    fn check_region_collision(&self, scene: &Scene) -> Option<(f32, f32, f32)> {
        let half_w = self.logo_width as f32 / 2.0;
        let half_h = self.logo_height as f32 / 2.0;
        let center_x = self.x + half_w;
        let center_y = self.y + half_h;

        for region in &scene.regions {
            let collision = match region.get_shape() {
                Shape::Polygon(p) => {
                    let verts = p.as_tuples();
                    rect_polygon_collision(
                        self.x,
                        self.y,
                        self.logo_width as f32,
                        self.logo_height as f32,
                        &verts,
                    )
                }
                Shape::Circle(c) => {
                    // Approximate rect-circle collision using center distance
                    let dx = center_x - c.center.x;
                    let dy = center_y - c.center.y;
                    let dist_sq = dx * dx + dy * dy;
                    let effective_radius = c.radius + half_w.max(half_h);
                    if dist_sq < effective_radius * effective_radius && dist_sq > 0.001 {
                        let dist = dist_sq.sqrt();
                        let nx = dx / dist;
                        let ny = dy / dist;
                        let penetration = effective_radius - dist;
                        Some((nx, ny, penetration))
                    } else {
                        None
                    }
                }
            };

            if collision.is_some() {
                return collision;
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
        // Recompute logo scale for current screen size
        let new_scale = (BASE_LOGO_SCALE as f32 * width.min(height) as f32 / 480.0)
            .round()
            .max(1.0) as u32;
        if new_scale != self.logo_scale {
            self.logo_scale = new_scale;
            self.logo_width = text_width_scaled(LOGO_TEXT, new_scale);
            self.logo_height = GLYPH_HEIGHT * new_scale;
        }

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

        // Move (scale speed proportionally to viewport)
        let sx = width as f32 / 640.0;
        let sy = height as f32 / 480.0;
        let new_x = self.x + self.vx * dt * sx;
        let new_y = self.y + self.vy * dt * sy;

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

        // Update region bounce cooldown
        if self.region_bounce_cooldown > 0.0 {
            self.region_bounce_cooldown -= dt;
            if self.region_bounce_cooldown <= 0.0 {
                // Cooldown expired, reset rapid bounce counter
                self.rapid_bounce_count = 0;
            }
        }

        // Region collision (polygons and circles) - skip if in cooldown
        if self.region_bounce_cooldown <= 0.0 {
            if let Some((nx, ny, dist)) = self.check_region_collision(scene) {
                let (new_vx, new_vy) = reflect(self.vx, self.vy, nx, ny);
                self.vx = new_vx;
                self.vy = new_vy;

                // Track rapid bounces - if bouncing too fast, we're probably stuck
                self.rapid_bounce_count += 1;

                // Base push distance, increases with rapid bounce count to escape corners
                let escape_multiplier = 1.0 + (self.rapid_bounce_count as f32 * 0.5);
                let push_dist = (dist + 15.0) * escape_multiplier;
                self.x += nx * push_dist;
                self.y += ny * push_dist;

                // Set cooldown - longer if we're bouncing rapidly (stuck)
                let base_cooldown = 0.05; // 50ms minimum between region bounces
                self.region_bounce_cooldown = base_cooldown * escape_multiplier;

                // If bouncing very rapidly (4+ times), add velocity boost to escape
                if self.rapid_bounce_count >= 4 {
                    let speed_boost = 1.2;
                    self.vx *= speed_boost;
                    self.vy *= speed_boost;
                }

                bounced = true;
            }
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
            draw_text_scaled(
                buffer,
                tx as i32,
                ty as i32,
                LOGO_TEXT,
                r,
                g,
                b,
                self.logo_scale,
            );
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
            self.logo_scale,
        );
        draw_text_scaled(
            buffer,
            self.x as i32 + 1,
            self.y as i32,
            LOGO_TEXT,
            gr,
            gg,
            gb,
            self.logo_scale,
        );
        draw_text_scaled(
            buffer,
            self.x as i32,
            self.y as i32 - 1,
            LOGO_TEXT,
            gr,
            gg,
            gb,
            self.logo_scale,
        );
        draw_text_scaled(
            buffer,
            self.x as i32,
            self.y as i32 + 1,
            LOGO_TEXT,
            gr,
            gg,
            gb,
            self.logo_scale,
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
            self.logo_scale,
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
