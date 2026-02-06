//! Gravity Balls Effect
//!
//! Colorful balls bouncing with realistic gravity physics.
//! Balls bounce off walls and defined regions.

use super::Effect;
use crate::display::PixelBuffer;
use crate::geometry::circle_polygon_collision;
use crate::regions::{Scene, Shape};
use crate::util::Rng;

const NUM_BALLS: usize = 12;
const GRAVITY: f32 = 400.0;
const BOUNCE_DAMPING: f32 = 0.85;
const TRAIL_LENGTH: usize = 20;

/// A single bouncing ball
struct Ball {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    radius: f32,
    hue: f32,
    trail: Vec<(f32, f32)>,
}

/// Bouncing balls with gravity
pub struct GravityBalls {
    balls: Vec<Ball>,
    rng: Rng,
    width: u32,
    height: u32,
}

impl GravityBalls {
    pub fn new() -> Self {
        Self {
            balls: Vec::new(),
            rng: Rng::new(0xBA11_5678),
            width: 0,
            height: 0,
        }
    }

    fn init_balls(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.balls.clear();

        let min_dim = width.min(height) as f32;

        for i in 0..NUM_BALLS {
            let radius = min_dim * (0.02 + (self.rng.next_u8() as f32 / 255.0) * 0.03);
            self.balls.push(Ball {
                x: radius + self.rng.next_f32() * (width as f32 - radius * 2.0),
                y: radius + self.rng.next_f32() * (height as f32 * 0.5),
                vx: (self.rng.next_f32() - 0.5) * 200.0,
                vy: (self.rng.next_f32() - 0.5) * 100.0,
                radius,
                hue: (i as f32 / NUM_BALLS as f32) * 360.0,
                trail: Vec::with_capacity(TRAIL_LENGTH),
            });
        }
    }

    fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
        let h = h % 360.0;
        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;

        let (r, g, b) = match (h / 60.0) as i32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };

        (
            ((r + m) * 255.0) as u8,
            ((g + m) * 255.0) as u8,
            ((b + m) * 255.0) as u8,
        )
    }
}

impl Default for GravityBalls {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for GravityBalls {
    fn update(&mut self, dt: f32, width: u32, height: u32, scene: &Scene) {
        if width != self.width || height != self.height || self.balls.is_empty() {
            self.init_balls(width, height);
        }

        let w = width as f32;
        let h = height as f32;

        for ball in &mut self.balls {
            // Store trail position
            ball.trail.push((ball.x, ball.y));
            if ball.trail.len() > TRAIL_LENGTH {
                ball.trail.remove(0);
            }

            // Apply gravity
            ball.vy += GRAVITY * dt;

            // Update position
            ball.x += ball.vx * dt;
            ball.y += ball.vy * dt;

            // Bounce off walls
            if ball.x < ball.radius {
                ball.x = ball.radius;
                ball.vx = -ball.vx * BOUNCE_DAMPING;
            } else if ball.x > w - ball.radius {
                ball.x = w - ball.radius;
                ball.vx = -ball.vx * BOUNCE_DAMPING;
            }

            if ball.y < ball.radius {
                ball.y = ball.radius;
                ball.vy = -ball.vy * BOUNCE_DAMPING;
            } else if ball.y > h - ball.radius {
                ball.y = h - ball.radius;
                ball.vy = -ball.vy * BOUNCE_DAMPING;
                // Add a tiny bit of random horizontal motion on floor bounce
                ball.vx += (self.rng.next_f32() - 0.5) * 20.0;
            }

            // Bounce off regions
            for region in &scene.regions {
                match region.get_shape() {
                    Shape::Polygon(poly) => {
                        // Check circle-polygon collision with proper normal
                        let verts = poly.as_tuples();
                        if let Some((nx, ny, penetration)) =
                            circle_polygon_collision(ball.x, ball.y, ball.radius, &verts)
                        {
                            // Push ball out along normal
                            ball.x += nx * (penetration + 1.0);
                            ball.y += ny * (penetration + 1.0);

                            // Reflect velocity off the surface normal
                            let dot = ball.vx * nx + ball.vy * ny;
                            ball.vx = (ball.vx - 2.0 * dot * nx) * BOUNCE_DAMPING;
                            ball.vy = (ball.vy - 2.0 * dot * ny) * BOUNCE_DAMPING;
                        }
                    }
                    Shape::Circle(circle) => {
                        let dx = ball.x - circle.center.x;
                        let dy = ball.y - circle.center.y;
                        let dist = (dx * dx + dy * dy).sqrt();
                        let min_dist = ball.radius + circle.radius;

                        if dist < min_dist && dist > 0.0 {
                            // Push ball out
                            let nx = dx / dist;
                            let ny = dy / dist;
                            ball.x = circle.center.x + nx * min_dist;
                            ball.y = circle.center.y + ny * min_dist;

                            // Reflect velocity
                            let dot = ball.vx * nx + ball.vy * ny;
                            ball.vx = (ball.vx - 2.0 * dot * nx) * BOUNCE_DAMPING;
                            ball.vy = (ball.vy - 2.0 * dot * ny) * BOUNCE_DAMPING;
                        }
                    }
                }
            }

            // Slowly rotate hue
            ball.hue = (ball.hue + dt * 30.0) % 360.0;
        }

        // Reset individual balls that are too slow (stuck or settled)
        const MIN_BALL_SPEED: f32 = 25.0;
        for ball in &mut self.balls {
            let speed = (ball.vx * ball.vx + ball.vy * ball.vy).sqrt();
            if speed < MIN_BALL_SPEED {
                // This ball is stuck - reset it with fresh energy
                ball.x = self.rng.next_f32() * w;
                ball.y = self.rng.next_f32() * h * 0.3; // Top 30%
                ball.vx = (self.rng.next_f32() - 0.5) * 300.0;
                ball.vy = self.rng.next_f32() * 100.0; // Downward
                ball.trail.clear();
            }
        }
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        buffer.clear(10, 10, 20);

        // Draw trails
        for ball in &self.balls {
            let (r, g, b) = Self::hsv_to_rgb(ball.hue, 0.8, 1.0);

            for (i, &(tx, ty)) in ball.trail.iter().enumerate() {
                let alpha = i as f32 / TRAIL_LENGTH as f32;
                let trail_radius = (ball.radius * 0.5 * alpha) as i32;
                let tr = (r as f32 * alpha * 0.5) as u8;
                let tg = (g as f32 * alpha * 0.5) as u8;
                let tb = (b as f32 * alpha * 0.5) as u8;

                // Draw small trail circle
                for dy in -trail_radius..=trail_radius {
                    for dx in -trail_radius..=trail_radius {
                        if dx * dx + dy * dy <= trail_radius * trail_radius {
                            buffer.set_pixel(tx as i32 + dx, ty as i32 + dy, tr, tg, tb);
                        }
                    }
                }
            }
        }

        // Draw balls
        for ball in &self.balls {
            let (r, g, b) = Self::hsv_to_rgb(ball.hue, 0.8, 1.0);
            let cx = ball.x as i32;
            let cy = ball.y as i32;
            let radius = ball.radius as i32;

            // Draw filled circle with gradient
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    let dist_sq = dx * dx + dy * dy;
                    if dist_sq <= radius * radius {
                        // Gradient from center (bright) to edge (darker)
                        let dist = (dist_sq as f32).sqrt();
                        let t = 1.0 - (dist / ball.radius);
                        let brightness = 0.6 + t * 0.4;

                        let pr = (r as f32 * brightness) as u8;
                        let pg = (g as f32 * brightness) as u8;
                        let pb = (b as f32 * brightness) as u8;

                        buffer.set_pixel(cx + dx, cy + dy, pr, pg, pb);
                    }
                }
            }

            // Highlight
            let hx = cx - radius / 3;
            let hy = cy - radius / 3;
            let hr = radius / 4;
            for dy in -hr..=hr {
                for dx in -hr..=hr {
                    if dx * dx + dy * dy <= hr * hr {
                        buffer.set_pixel(hx + dx, hy + dy, 255, 255, 255);
                    }
                }
            }
        }
    }

    fn name(&self) -> &str {
        "Gravity Balls"
    }

    fn region_color(&self) -> (u8, u8, u8) {
        (40, 40, 60)
    }
}
