use super::Effect;
use crate::display::PixelBuffer;
use crate::regions::Scene;
use crate::util::{hsv_to_rgb, Rng};

const NUM_STARS: usize = 300;
const MAX_TRAIL_LEN: f32 = 40.0;

struct Star {
    x: f32,
    y: f32,
    z: f32,
    hue: f32,
}

/// Classic 3D starfield effect
pub struct Starfield {
    stars: Vec<Star>,
    center_x: f32,
    center_y: f32,
    speed: f32,
    rng: Rng,
}

impl Starfield {
    pub fn new() -> Self {
        let mut rng = Rng::new(12345);
        let mut stars = Vec::with_capacity(NUM_STARS);

        for _ in 0..NUM_STARS {
            stars.push(Self::random_star(&mut rng));
        }

        Self {
            stars,
            center_x: 320.0,
            center_y: 240.0,
            speed: 200.0,
            rng,
        }
    }

    fn random_star(rng: &mut Rng) -> Star {
        Star {
            x: (rng.next_f32() - 0.5) * 1000.0,
            y: (rng.next_f32() - 0.5) * 1000.0,
            z: rng.range_f32(100.0, 600.0),
            hue: rng.range_f32(0.0, 360.0),
        }
    }
}

impl Default for Starfield {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Starfield {
    fn update(&mut self, dt: f32, width: u32, height: u32, _scene: &Scene) {
        // Update center to current screen dimensions
        self.center_x = width as f32 / 2.0;
        self.center_y = height as f32 / 2.0;

        for star in &mut self.stars {
            star.z -= self.speed * dt;
            star.hue = (star.hue + dt * 720.0).rem_euclid(360.0);

            // Reset stars that pass the camera
            if star.z <= 1.0 {
                *star = Self::random_star(&mut self.rng);
                star.z = 500.0;
            }
        }
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        buffer.clear(0, 0, 0);

        // Scale projection and trails proportionally to viewport
        let vp_scale = buffer.width().min(buffer.height()) as f32 / 480.0;
        let fov = 256.0 * vp_scale;
        let max_trail = MAX_TRAIL_LEN * vp_scale;

        for star in &self.stars {
            // 3D projection
            let sx = (star.x / star.z) * fov + self.center_x;
            let sy = (star.y / star.z) * fov + self.center_y;

            // Brightness based on distance
            let brightness = ((1.0 - star.z / 600.0) * 255.0).clamp(0.0, 255.0) as u8;

            let (r, g, b) = hsv_to_rgb(star.hue, 1.0, brightness as f32 / 255.0);
            let trail_len = ((1.0 - (star.z / 600.0).clamp(0.0, 1.0)) * max_trail).ceil() as i32;

            if trail_len > 0 {
                let dx = (sx - self.center_x) / star.z;
                let dy = (sy - self.center_y) / star.z;

                for i in 1..=trail_len {
                    let fade = (1.0 - (i as f32 / (trail_len as f32 + 1.0))).max(0.0);
                    let tr = (r as f32 * fade) as u8;
                    let tg = (g as f32 * fade) as u8;
                    let tb = (b as f32 * fade) as u8;
                    buffer.set_pixel(
                        (sx - dx * i as f32) as i32,
                        (sy - dy * i as f32) as i32,
                        tr,
                        tg,
                        tb,
                    );
                }
            }

            // Draw star as single pixel or small cross for near stars
            if star.z < 100.0 {
                buffer.set_pixel(sx as i32, sy as i32, 255, 255, 255);
                buffer.set_pixel(sx as i32 - 1, sy as i32, r, g, b);
                buffer.set_pixel(sx as i32 + 1, sy as i32, r, g, b);
                buffer.set_pixel(sx as i32, sy as i32 - 1, r, g, b);
                buffer.set_pixel(sx as i32, sy as i32 + 1, r, g, b);
            } else {
                buffer.set_pixel(sx as i32, sy as i32, r, g, b);
            }
        }
    }

    fn name(&self) -> &str {
        "Starfield"
    }

    fn region_color(&self) -> (u8, u8, u8) {
        // Deep space blue
        (2, 2, 8)
    }
}
