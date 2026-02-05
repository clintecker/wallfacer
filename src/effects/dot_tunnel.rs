use super::Effect;
use crate::display::PixelBuffer;
use crate::math3d::{project_with_depth, Vec3};
use crate::regions::Scene;
use crate::util::hsv_to_rgb;

use std::f32::consts::PI;

const NUM_RINGS: usize = 40;
const DOTS_PER_RING: usize = 16;
const NEAR_Z: f32 = 50.0;
const FAR_Z: f32 = 800.0;

struct Ring {
    z: f32,
}

pub struct DotTunnel {
    time: f32,
    rings: Vec<Ring>,
    speed: f32,
}

impl DotTunnel {
    pub fn new() -> Self {
        let spacing = (FAR_Z - NEAR_Z) / NUM_RINGS as f32;
        let rings = (0..NUM_RINGS)
            .map(|i| Ring {
                z: NEAR_Z + i as f32 * spacing,
            })
            .collect();

        Self {
            time: 0.0,
            rings,
            speed: 200.0,
        }
    }
}

impl Default for DotTunnel {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for DotTunnel {
    fn update(&mut self, dt: f32, _width: u32, _height: u32, _scene: &Scene) {
        self.time += dt;

        // Advance rings toward camera
        for ring in &mut self.rings {
            ring.z -= self.speed * dt;
            // Recycle rings that pass behind the camera
            if ring.z < 1.0 {
                ring.z += FAR_Z - NEAR_Z;
            }
        }
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        let width = buffer.width() as f32;
        let height = buffer.height() as f32;
        let cx = width / 2.0;
        let cy = height / 2.0;
        let scale = width.min(height) / 480.0;
        let fov = 300.0 * scale;

        buffer.clear(0, 0, 0);

        for ring in &self.rings {
            let z = ring.z;

            // Twist: rotation increases with depth
            let base_angle = z * 0.008 + self.time * 0.5;

            // Wavy tunnel walls: radius oscillates with z and time
            let radius = 120.0
                + 40.0 * (z * 0.015 + self.time * 1.5).sin()
                + 20.0 * (z * 0.025 - self.time * 0.8).cos();

            for d in 0..DOTS_PER_RING {
                let angle = base_angle + d as f32 * PI * 2.0 / DOTS_PER_RING as f32;
                let pos = Vec3::new(radius * angle.cos(), radius * angle.sin(), z);

                if let Some((sx, sy, proximity)) = project_with_depth(pos, fov, cx, cy, FAR_Z) {
                    // Color: hue from depth, brightness from proximity
                    let hue = (z * 0.5 + self.time * 40.0).rem_euclid(360.0);
                    let brightness = 0.3 + proximity * 0.7;
                    let (r, g, b) = hsv_to_rgb(hue, 0.8, brightness);

                    // Dot size: bigger when closer
                    let dot_size = (2.0 + proximity * 6.0) * scale;

                    if dot_size > 1.5 {
                        buffer.fill_circle(sx as i32, sy as i32, dot_size as i32, r, g, b);
                    } else {
                        buffer.splat_pixel(sx, sy, r, g, b, proximity);
                    }
                }
            }
        }
    }

    fn name(&self) -> &str {
        "Dot Tunnel"
    }
}
