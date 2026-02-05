//! Worms Effect
//!
//! Chaotic worms that grow, slither, rotate through colors, and eventually die.
//! Worms bounce off screen edges and user-defined polygon regions.

use super::Effect;
use crate::display::PixelBuffer;
use crate::geometry::{rect_polygon_collision, reflect};
use crate::regions::Scene;
use crate::util::{hsv_to_rgb, Rng};

const MAX_WORMS: usize = 24;
const MAX_SEGMENTS: usize = 800;
const BASE_SEGMENT_SIZE: f32 = 4.0;

/// A single worm with head, body segments, and lifecycle
struct Worm {
    /// Body segments (head is segments[0])
    segments: Vec<(f32, f32)>,
    /// Current head position (may differ from segments[0] between segment commits)
    head_x: f32,
    head_y: f32,
    /// Distance traveled since last segment was added
    distance_accum: f32,
    /// Current movement direction in radians
    direction: f32,
    /// Angular velocity (how fast it turns)
    turn_speed: f32,
    /// Movement speed in pixels per second
    speed: f32,
    /// Color hue (0-360)
    hue: f32,
    /// How fast the hue rotates
    hue_speed: f32,
    /// Current age in seconds
    age: f32,
    /// Maximum lifespan in seconds
    lifespan: f32,
    /// Growth phase: how many segments to grow to
    max_segments: usize,
    /// Is this worm alive?
    alive: bool,
}

impl Worm {
    fn new(x: f32, y: f32, rng: &mut Rng) -> Self {
        let direction = rng.next_f32() * std::f32::consts::TAU;
        let speed = 40.0 + rng.next_f32() * 60.0; // 40-100 px/s
        let lifespan = 20.0 + rng.next_f32() * 30.0; // 20-50 seconds
        let max_segments = 400 + (rng.next_f32() * (MAX_SEGMENTS - 400) as f32) as usize;
        let hue = rng.next_f32() * 360.0;
        let hue_speed = 20.0 + rng.next_f32() * 40.0; // degrees per second

        Self {
            segments: vec![(x, y)],
            head_x: x,
            head_y: y,
            distance_accum: 0.0,
            direction,
            turn_speed: 0.0,
            speed,
            hue,
            hue_speed,
            age: 0.0,
            lifespan,
            max_segments,
            alive: true,
        }
    }

    fn update(
        &mut self,
        dt: f32,
        width: u32,
        height: u32,
        scene: &Scene,
        rng: &mut Rng,
        scale: f32,
    ) {
        if !self.alive {
            // Dead worm - rapidly shrink segments until gone
            for _ in 0..3 {
                if !self.segments.is_empty() {
                    self.segments.pop();
                }
            }
            return;
        }

        self.age += dt;

        // Check if worm has died
        if self.age >= self.lifespan {
            self.alive = false;
            return;
        }

        // Rotate hue
        self.hue = (self.hue + self.hue_speed * dt) % 360.0;

        // Smooth random turning - occasionally change turn direction
        if rng.next_f32() < 0.02 {
            self.turn_speed = (rng.next_f32() - 0.5) * 4.0; // -2 to +2 rad/s
        }

        // Apply turning with some damping
        self.direction += self.turn_speed * dt;

        // Move head (scale speed proportionally to viewport)
        let seg_size = BASE_SEGMENT_SIZE * scale;
        let mut new_x = self.head_x + self.direction.cos() * self.speed * dt * scale;
        let mut new_y = self.head_y + self.direction.sin() * self.speed * dt * scale;
        let mut bounced = false;

        // Bounce off walls
        let margin = seg_size;
        let w = width as f32 - margin;
        let h = height as f32 - margin;

        if new_x < margin {
            new_x = margin + (margin - new_x);
            self.direction = std::f32::consts::PI - self.direction;
            bounced = true;
        } else if new_x > w {
            new_x = w - (new_x - w);
            self.direction = std::f32::consts::PI - self.direction;
            bounced = true;
        }

        if new_y < margin {
            new_y = margin + (margin - new_y);
            self.direction = -self.direction;
            bounced = true;
        } else if new_y > h {
            new_y = h - (new_y - h);
            self.direction = -self.direction;
            bounced = true;
        }

        // Bounce off polygon regions - treat worm head as a small bounding box
        let head_size = seg_size + 1.0;
        let half_size = head_size / 2.0;
        for region in &scene.regions {
            let verts = region.polygon.as_tuples();
            // Check collision using rectangle around head position
            if let Some((nx, ny, dist)) = rect_polygon_collision(
                new_x - half_size,
                new_y - half_size,
                head_size,
                head_size,
                &verts,
            ) {
                // Reflect direction
                let (new_dx, new_dy) = reflect(self.direction.cos(), self.direction.sin(), nx, ny);
                self.direction = new_dy.atan2(new_dx);

                // Push out of polygon: distance to edge + margin
                let push_dist = dist + 6.0;
                new_x += nx * push_dist;
                new_y += ny * push_dist;
                bounced = true;
                break;
            }
        }

        if bounced {
            self.turn_speed = (rng.next_f32() - 0.5) * 3.0;
        }

        // Calculate distance moved this frame
        let dx = new_x - self.head_x;
        let dy = new_y - self.head_y;
        let dist_moved = (dx * dx + dy * dy).sqrt();

        // Update head position
        self.head_x = new_x;
        self.head_y = new_y;

        // Accumulate distance and add segments when threshold reached
        // This makes tail length frame-rate independent
        let segment_spacing = seg_size * 0.5;
        self.distance_accum += dist_moved;

        while self.distance_accum >= segment_spacing {
            self.distance_accum -= segment_spacing;
            self.segments.insert(0, (new_x, new_y));
        }

        // Always keep head position current in first segment for smooth rendering
        if !self.segments.is_empty() {
            self.segments[0] = (new_x, new_y);
        }

        // Grow or maintain length
        let target_len = if self.age < 5.0 {
            // Growing phase - 5 seconds to reach full length
            ((self.age / 5.0) * self.max_segments as f32) as usize
        } else {
            self.max_segments
        };

        // Shrink when dying (last 20% of life)
        let death_phase = self.age / self.lifespan;
        let target_len = if death_phase > 0.8 {
            let shrink = (death_phase - 0.8) / 0.2; // 0 to 1
            (target_len as f32 * (1.0 - shrink)) as usize
        } else {
            target_len
        };

        while self.segments.len() > target_len.max(1) {
            self.segments.pop();
        }
    }

    fn render(&self, buffer: &mut PixelBuffer, seg_size: u32) {
        if !self.alive && self.segments.is_empty() {
            return;
        }

        let segment_count = self.segments.len();
        if segment_count == 0 {
            return;
        }

        // Calculate fade based on death phase
        let death_phase = self.age / self.lifespan;
        let global_alpha = if death_phase > 0.8 {
            1.0 - (death_phase - 0.8) / 0.2
        } else {
            1.0
        };

        for (i, &(x, y)) in self.segments.iter().enumerate() {
            // Color varies along body (gradient effect)
            let segment_hue = (self.hue + i as f32 * 3.0) % 360.0;
            let (r, g, b) = hsv_to_rgb(segment_hue, 0.9, 0.95);

            // Fade toward tail
            let tail_fade = 1.0 - (i as f32 / segment_count as f32) * 0.6;
            let alpha = global_alpha * tail_fade;

            let r = (r as f32 * alpha) as u8;
            let g = (g as f32 * alpha) as u8;
            let b = (b as f32 * alpha) as u8;

            // Draw segment as a filled circle
            let size = if i == 0 {
                seg_size + 1 // Head is slightly larger
            } else {
                seg_size - (i * seg_size as usize / segment_count / 2) as u32
            };
            let size = size.max(2);

            buffer.fill_circle(x as i32, y as i32, size as i32, r, g, b);
        }
    }
}

/// The worms effect
pub struct Worms {
    worms: Vec<Worm>,
    rng: Rng,
    spawn_timer: f32,
    time: f32,
    // Cache screen dimensions for spawning
    screen_width: u32,
    screen_height: u32,
    screen_scale: f32,
    // Defer initial spawn until we know screen dimensions
    needs_initial_spawn: bool,
}

impl Worms {
    pub fn new() -> Self {
        Self {
            worms: Vec::with_capacity(MAX_WORMS),
            rng: Rng::new(42),
            spawn_timer: 0.0,
            time: 0.0,
            screen_width: 640,
            screen_height: 480,
            screen_scale: 1.0,
            needs_initial_spawn: true,
        }
    }

    fn spawn_worm(&mut self) {
        if self.worms.len() >= MAX_WORMS {
            return;
        }

        let w = self.screen_width as f32;
        let h = self.screen_height as f32;

        // Spawn at random edge
        let edge = self.rng.range_i32(0, 3);
        let (x, y) = match edge {
            0 => (self.rng.range_f32(0.0, w), 10.0),     // Top
            1 => (self.rng.range_f32(0.0, w), h - 10.0), // Bottom
            2 => (10.0, self.rng.range_f32(0.0, h)),     // Left
            _ => (w - 10.0, self.rng.range_f32(0.0, h)), // Right
        };

        self.worms.push(Worm::new(x, y, &mut self.rng));
    }
}

impl Default for Worms {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Worms {
    fn update(&mut self, dt: f32, width: u32, height: u32, scene: &Scene) {
        self.time += dt;
        self.screen_width = width;
        self.screen_height = height;
        self.screen_scale = width.min(height) as f32 / 480.0;

        // Spawn initial worms distributed across the screen
        if self.needs_initial_spawn {
            self.needs_initial_spawn = false;
            let margin = 50.0;
            let w = width as f32 - margin * 2.0;
            let h = height as f32 - margin * 2.0;
            for _ in 0..8 {
                let x = margin + self.rng.next_f32() * w;
                let y = margin + self.rng.next_f32() * h;
                self.worms.push(Worm::new(x, y, &mut self.rng));
            }
        }

        // Update all worms
        let scale = self.screen_scale;
        for worm in &mut self.worms {
            worm.update(dt, width, height, scene, &mut self.rng, scale);
        }

        // Count how many worms died this frame
        let alive_before = self.worms.len();
        self.worms.retain(|w| w.alive || !w.segments.is_empty());
        let died = alive_before - self.worms.len();

        // Spawn replacements for dead worms immediately
        for _ in 0..died {
            self.spawn_worm();
        }

        // Also spawn new worms periodically to maintain population
        self.spawn_timer += dt;
        if self.spawn_timer > 1.5 && self.worms.len() < 12 {
            self.spawn_timer = 0.0;
            self.spawn_worm();
        }
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        // Dark background with slight blue tint
        buffer.clear(5, 5, 15);

        // Render all worms (back to front for proper layering)
        let seg_size = (BASE_SEGMENT_SIZE * self.screen_scale).round().max(2.0) as u32;
        for worm in &self.worms {
            worm.render(buffer, seg_size);
        }
    }

    fn name(&self) -> &str {
        "Worms"
    }

    fn region_color(&self) -> (u8, u8, u8) {
        // Toxic green glow for worm regions
        (0, 40, 10)
    }
}
