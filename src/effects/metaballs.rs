//! Metaballs Effect
//!
//! Classic demoscene effect where blob-like objects merge smoothly
//! when they approach each other using an implicit surface function.

use super::Effect;
use crate::display::PixelBuffer;
use crate::regions::Scene;

const NUM_BALLS: usize = 8;

/// A single metaball
struct Ball {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    radius: f32,
}

/// Metaballs effect with smooth merging blobs
pub struct Metaballs {
    balls: Vec<Ball>,
    time: f32,
    palette: Vec<(u8, u8, u8)>,
    width: u32,
    height: u32,
}

impl Metaballs {
    pub fn new() -> Self {
        // Create a nice color palette (black -> purple -> magenta -> white)
        let mut palette = Vec::with_capacity(256);
        for i in 0..256 {
            let t = i as f32 / 255.0;
            let (r, g, b) = if t < 0.5 {
                // Black to purple/magenta
                let t2 = t * 2.0;
                (
                    (t2 * 200.0) as u8,
                    (t2 * 50.0) as u8,
                    (t2 * 255.0) as u8,
                )
            } else {
                // Purple/magenta to white
                let t2 = (t - 0.5) * 2.0;
                (
                    (200.0 + t2 * 55.0) as u8,
                    (50.0 + t2 * 205.0) as u8,
                    255,
                )
            };
            palette.push((r, g, b));
        }

        Self {
            balls: Vec::new(),
            time: 0.0,
            palette,
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
            let angle = (i as f32 / NUM_BALLS as f32) * std::f32::consts::TAU;
            let speed = 50.0 + (i as f32 * 20.0);
            self.balls.push(Ball {
                x: width as f32 / 2.0 + (angle.cos() * 100.0),
                y: height as f32 / 2.0 + (angle.sin() * 100.0),
                vx: angle.cos() * speed,
                vy: angle.sin() * speed,
                radius: min_dim * (0.08 + (i as f32 * 0.015)),
            });
        }
    }
}

impl Default for Metaballs {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Metaballs {
    fn update(&mut self, dt: f32, width: u32, height: u32, _scene: &Scene) {
        if width != self.width || height != self.height || self.balls.is_empty() {
            self.init_balls(width, height);
        }

        self.time += dt;

        let w = width as f32;
        let h = height as f32;

        // Update ball positions
        for ball in &mut self.balls {
            ball.x += ball.vx * dt;
            ball.y += ball.vy * dt;

            // Bounce off walls
            if ball.x < ball.radius {
                ball.x = ball.radius;
                ball.vx = -ball.vx;
            } else if ball.x > w - ball.radius {
                ball.x = w - ball.radius;
                ball.vx = -ball.vx;
            }

            if ball.y < ball.radius {
                ball.y = ball.radius;
                ball.vy = -ball.vy;
            } else if ball.y > h - ball.radius {
                ball.y = h - ball.radius;
                ball.vy = -ball.vy;
            }
        }
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        let width = buffer.width();
        let height = buffer.height();

        // Use 2x2 pixel blocks for performance
        let step = 2;

        for y in (0..height).step_by(step) {
            for x in (0..width).step_by(step) {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;

                // Calculate field value at this point
                let mut field: f32 = 0.0;
                for ball in &self.balls {
                    let dx = px - ball.x;
                    let dy = py - ball.y;
                    let dist_sq = dx * dx + dy * dy;
                    // Classic metaball formula: r² / d²
                    field += (ball.radius * ball.radius) / (dist_sq + 1.0);
                }

                // Map field value to color
                let intensity = ((field * 128.0).min(255.0)) as u8;
                let (r, g, b) = self.palette[intensity as usize];

                // Fill the 2x2 block
                for dy in 0..step as u32 {
                    for dx in 0..step as u32 {
                        if x + dx < width && y + dy < height {
                            buffer.set_pixel((x + dx) as i32, (y + dy) as i32, r, g, b);
                        }
                    }
                }
            }
        }
    }

    fn name(&self) -> &str {
        "Metaballs"
    }

    fn region_color(&self) -> (u8, u8, u8) {
        (0, 0, 0)
    }
}
