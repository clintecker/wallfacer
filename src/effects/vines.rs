//! Bioluminescent Vines — Glowing tendrils sprout from frame edges
//!
//! Organic vine systems grow outward from each frame, branching randomly.
//! Vines from different frames reach toward each other. When they connect,
//! a pulse of light travels the vine network.

use super::Effect;
use crate::display::PixelBuffer;
use crate::regions::Scene;
use crate::util::{hsv_to_rgb, Rng};
use std::f32::consts::TAU;

const MAX_SEGMENTS: usize = 8000;
const GROWTH_SPEED: f32 = 40.0; // pixels/sec
const BRANCH_PROBABILITY: f32 = 0.02; // per segment per frame
const MAX_BRANCHES: usize = 3; // max branches per node
const VINE_THICKNESS: f32 = 1.5;
const NODE_GLOW_RADIUS: i32 = 4;

const GROWTH_RATE: f32 = 12.0; // new tip extensions per second
const PULSE_SPEED: f32 = 200.0; // pixels/sec along vine
const PULSE_LIFETIME: f32 = 3.0;
const MAX_PULSES: usize = 20;

/// A single vine segment (one edge in the vine graph)
struct Segment {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    frame_id: u8,    // which frame this vine belongs to
    depth: u16,      // branching depth (0 = trunk)
    brightness: f32, // 1.0 for trunk, decreases with depth
}

/// An active growing tip
struct Tip {
    x: f32,
    y: f32,
    angle: f32,
    frame_id: u8,
    depth: u16,
    speed: f32,
}

/// A pulse of light traveling along the vine network
struct Pulse {
    origin_x: f32,
    origin_y: f32,
    radius: f32, // expanding circle from origin
    age: f32,
    hue: f32,
}

struct FrameInfo {
    cx: f32,
    cy: f32,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

pub struct Vines {
    segments: Vec<Segment>,
    tips: Vec<Tip>,
    pulses: Vec<Pulse>,
    frames: Vec<FrameInfo>,

    rng: Rng,
    time: f32,
    growth_accum: f32,
    scene_fingerprint: u64,
    screen_w: u32,
    screen_h: u32,
}

impl Vines {
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
            tips: Vec::new(),
            pulses: Vec::new(),
            frames: Vec::new(),
            rng: Rng::new(0x71AE),
            time: 0.0,
            growth_accum: 0.0,
            scene_fingerprint: u64::MAX,
            screen_w: 0,
            screen_h: 0,
        }
    }

    fn scene_fingerprint(scene: &Scene) -> u64 {
        let mut h: u64 = scene.regions.len() as u64;
        for region in &scene.regions {
            for v in &region.polygon.vertices {
                h = h.wrapping_mul(31).wrapping_add(v.x.to_bits() as u64);
                h = h.wrapping_mul(31).wrapping_add(v.y.to_bits() as u64);
            }
        }
        h
    }

    fn rebuild_scene(&mut self, width: u32, height: u32, scene: &Scene, fingerprint: u64) {
        self.screen_w = width;
        self.screen_h = height;
        self.scene_fingerprint = fingerprint;

        self.frames.clear();
        for region in &scene.regions {
            if let Some((min_x, min_y, max_x, max_y)) = region.polygon.bounds() {
                if let Some(c) = region.polygon.centroid() {
                    self.frames.push(FrameInfo {
                        cx: c.x,
                        cy: c.y,
                        min_x,
                        min_y,
                        max_x,
                        max_y,
                    });
                }
            }
        }
        self.frames.sort_by(|a, b| a.cx.partial_cmp(&b.cx).unwrap());

        // Spawn initial tips from frame edges
        self.segments.clear();
        self.tips.clear();
        self.pulses.clear();

        for (fi, frame) in self.frames.iter().enumerate() {
            let num_roots = 8;
            for _ in 0..num_roots {
                let t = self.rng.next_f32();
                let side = self.rng.next_u32() % 4;
                let (x, y) = match side {
                    0 => (frame.min_x + t * (frame.max_x - frame.min_x), frame.min_y),
                    1 => (frame.min_x + t * (frame.max_x - frame.min_x), frame.max_y),
                    2 => (frame.min_x, frame.min_y + t * (frame.max_y - frame.min_y)),
                    _ => (frame.max_x, frame.min_y + t * (frame.max_y - frame.min_y)),
                };

                // Angle: outward from frame center
                let dx = x - frame.cx;
                let dy = y - frame.cy;
                let angle = dy.atan2(dx) + self.rng.range_f32(-0.3, 0.3);

                self.tips.push(Tip {
                    x,
                    y,
                    angle,
                    frame_id: fi as u8,
                    depth: 0,
                    speed: GROWTH_SPEED * self.rng.range_f32(0.8, 1.2),
                });
            }
        }
    }

    fn inside_any_frame(&self, x: f32, y: f32) -> bool {
        for frame in &self.frames {
            if x >= frame.min_x && x <= frame.max_x && y >= frame.min_y && y <= frame.max_y {
                return true;
            }
        }
        false
    }

    /// Check if a point is near a vine from a different frame (for connection detection)
    fn near_other_vine(&self, x: f32, y: f32, my_frame: u8) -> bool {
        let threshold = 15.0;
        let threshold_sq = threshold * threshold;
        // Check a sample of segments for performance
        let step = (self.segments.len() / 200).max(1);
        for (i, seg) in self.segments.iter().enumerate() {
            if i % step != 0 {
                continue;
            }
            if seg.frame_id != my_frame {
                let dx = x - seg.x1;
                let dy = y - seg.y1;
                if dx * dx + dy * dy < threshold_sq {
                    return true;
                }
            }
        }
        false
    }
}

impl Default for Vines {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Vines {
    fn update(&mut self, dt: f32, width: u32, height: u32, scene: &Scene) {
        let fp = Self::scene_fingerprint(scene);
        if width != self.screen_w || height != self.screen_h || fp != self.scene_fingerprint {
            self.rebuild_scene(width, height, scene, fp);
        }

        self.time += dt;
        let w = width as f32;
        let h = height as f32;

        // Grow tips
        self.growth_accum += GROWTH_RATE * dt;
        let grow_steps = self.growth_accum as u32;
        self.growth_accum -= grow_steps as f32;

        for _ in 0..grow_steps {
            if self.segments.len() >= MAX_SEGMENTS {
                break;
            }

            let mut new_tips: Vec<Tip> = Vec::new();

            for tip in &mut self.tips {
                // Wander: organic curve with noise
                tip.angle += self.rng.range_f32(-0.15, 0.15);

                // Slight attraction toward nearest other-frame (if any)
                if self.frames.len() >= 2 {
                    let mut closest_dist = f32::MAX;
                    let mut closest_cx = tip.x;
                    let mut closest_cy = tip.y;
                    for (fi, frame) in self.frames.iter().enumerate() {
                        if fi as u8 != tip.frame_id {
                            let dx = frame.cx - tip.x;
                            let dy = frame.cy - tip.y;
                            let dist = dx * dx + dy * dy;
                            if dist < closest_dist {
                                closest_dist = dist;
                                closest_cx = frame.cx;
                                closest_cy = frame.cy;
                            }
                        }
                    }
                    // Gentle steering toward other frame
                    let target_angle = (closest_cy - tip.y).atan2(closest_cx - tip.x);
                    let mut diff = target_angle - tip.angle;
                    while diff > std::f32::consts::PI {
                        diff -= TAU;
                    }
                    while diff < -std::f32::consts::PI {
                        diff += TAU;
                    }
                    tip.angle += diff * 0.02; // very gentle pull
                }

                let old_x = tip.x;
                let old_y = tip.y;
                let step = tip.speed * (1.0 / GROWTH_RATE);
                tip.x += tip.angle.cos() * step;
                tip.y += tip.angle.sin() * step;

                // Kill if off screen or inside a frame
                if tip.x < -10.0 || tip.x > w + 10.0 || tip.y < -10.0 || tip.y > h + 10.0 {
                    tip.speed = 0.0; // mark dead
                    continue;
                }
                // Inline frame check to avoid borrow conflict
                let in_frame = self.frames.iter().any(|f| {
                    tip.x >= f.min_x && tip.x <= f.max_x && tip.y >= f.min_y && tip.y <= f.max_y
                });
                if in_frame {
                    tip.speed = 0.0;
                    continue;
                }

                let brightness = 1.0 / (1.0 + tip.depth as f32 * 0.3);
                self.segments.push(Segment {
                    x0: old_x,
                    y0: old_y,
                    x1: tip.x,
                    y1: tip.y,
                    frame_id: tip.frame_id,
                    depth: tip.depth,
                    brightness,
                });

                // Branch occasionally
                if tip.depth < 4
                    && self.rng.next_f32() < BRANCH_PROBABILITY
                    && new_tips.len() < MAX_BRANCHES
                {
                    let branch_angle = tip.angle
                        + self.rng.range_f32(0.4, 1.0)
                            * if self.rng.next_f32() < 0.5 { 1.0 } else { -1.0 };
                    new_tips.push(Tip {
                        x: tip.x,
                        y: tip.y,
                        angle: branch_angle,
                        frame_id: tip.frame_id,
                        depth: tip.depth + 1,
                        speed: tip.speed * self.rng.range_f32(0.7, 0.9),
                    });
                }
            }

            // Remove dead tips
            self.tips.retain(|t| t.speed > 0.0);
            self.tips.extend(new_tips);
        }

        // Check for vine connections and spawn pulses
        if self.frames.len() >= 2 {
            for tip in &self.tips {
                if self.near_other_vine(tip.x, tip.y, tip.frame_id)
                    && self.pulses.len() < MAX_PULSES
                {
                    self.pulses.push(Pulse {
                        origin_x: tip.x,
                        origin_y: tip.y,
                        radius: 0.0,
                        age: 0.0,
                        hue: self.rng.range_f32(150.0, 300.0),
                    });
                }
            }
        }

        // Update pulses
        for pulse in &mut self.pulses {
            pulse.age += dt;
            pulse.radius += PULSE_SPEED * dt;
        }
        self.pulses.retain(|p| p.age < PULSE_LIFETIME);
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        buffer.clear(3, 5, 3);

        let num_frames = self.frames.len();

        // Draw vine segments
        for seg in &self.segments {
            // Base hue per frame
            let hue = if num_frames > 1 {
                seg.frame_id as f32 / (num_frames - 1).max(1) as f32 * 120.0 + 100.0
            } else {
                160.0
            };

            // Check if any pulse illuminates this segment
            let mut pulse_boost: f32 = 0.0;
            for pulse in &self.pulses {
                let dx = seg.x1 - pulse.origin_x;
                let dy = seg.y1 - pulse.origin_y;
                let dist = (dx * dx + dy * dy).sqrt();
                let ring_dist = (dist - pulse.radius).abs();
                if ring_dist < 20.0 {
                    let fade = 1.0 - pulse.age / PULSE_LIFETIME;
                    let ring_fade = 1.0 - ring_dist / 20.0;
                    pulse_boost += fade * ring_fade * 0.8;
                }
            }
            pulse_boost = pulse_boost.min(1.0);

            let brightness = seg.brightness * 0.4 + pulse_boost * 0.6;
            let saturation = 0.6 + pulse_boost * 0.3;
            let (r, g, b) = hsv_to_rgb(hue, saturation, brightness.min(1.0));

            buffer.line_aa_additive(seg.x0, seg.y0, seg.x1, seg.y1, r, g, b);

            // Thicker trunk segments
            if seg.depth == 0 {
                let dx = seg.x1 - seg.x0;
                let dy = seg.y1 - seg.y0;
                let len = (dx * dx + dy * dy).sqrt().max(0.001);
                let nx = -dy / len * 0.6;
                let ny = dx / len * 0.6;
                let r2 = r / 2;
                let g2 = g / 2;
                let b2 = b / 2;
                buffer.line_aa_additive(
                    seg.x0 + nx,
                    seg.y0 + ny,
                    seg.x1 + nx,
                    seg.y1 + ny,
                    r2,
                    g2,
                    b2,
                );
                buffer.line_aa_additive(
                    seg.x0 - nx,
                    seg.y0 - ny,
                    seg.x1 - nx,
                    seg.y1 - ny,
                    r2,
                    g2,
                    b2,
                );
            }
        }

        // Glow nodes at branch points (every Nth segment tip)
        for (i, seg) in self.segments.iter().enumerate() {
            if i % 12 == 0 {
                let hue = if num_frames > 1 {
                    seg.frame_id as f32 / (num_frames - 1).max(1) as f32 * 120.0 + 100.0
                } else {
                    160.0
                };
                let (r, g, b) = hsv_to_rgb(hue, 0.5, seg.brightness * 0.3);
                buffer.fill_circle_gradient(
                    seg.x1 as i32,
                    seg.y1 as i32,
                    NODE_GLOW_RADIUS,
                    r,
                    g,
                    b,
                    2.0,
                );
            }
        }

        // Frame halos
        for (fi, frame) in self.frames.iter().enumerate() {
            let hue = if num_frames > 1 {
                fi as f32 / (num_frames - 1) as f32 * 120.0 + 100.0
            } else {
                160.0
            };
            let pulse = (self.time * 1.0 + fi as f32 * 1.5).sin() * 0.15 + 0.25;
            let size = ((frame.max_x - frame.min_x).max(frame.max_y - frame.min_y) * 0.5) as i32;
            let (r, g, b) = hsv_to_rgb(hue, 0.5, pulse);
            buffer.fill_circle_gradient(frame.cx as i32, frame.cy as i32, size, r, g, b, 2.0);
        }

        // Render pulses as expanding rings
        for pulse in &self.pulses {
            let fade = (1.0 - pulse.age / PULSE_LIFETIME).max(0.0);
            let (r, g, b) = hsv_to_rgb(pulse.hue, 0.6, fade * 0.5);
            let radius = pulse.radius as i32;
            if radius > 2 {
                // Draw ring by filling outer circle and subtracting inner
                // (approximate with additive gradient at the ring radius)
                buffer.fill_circle_gradient(
                    pulse.origin_x as i32,
                    pulse.origin_y as i32,
                    radius,
                    r / 3,
                    g / 3,
                    b / 3,
                    3.0,
                );
            }
        }

        buffer.bloom(20, 2, 0.3);
    }

    fn name(&self) -> &str {
        "Bioluminescent Vines"
    }

    fn region_color(&self) -> (u8, u8, u8) {
        (3, 5, 3)
    }
}
