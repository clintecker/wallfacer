//! 3D Pipes Effect
//!
//! Growing pipes in 3D space, inspired by the classic Windows screensaver.
//! Cylinder cross-sections tessellated into strips with cosine-falloff lighting,
//! matching what gluCylinder does in OpenGL. Sphere joints at bends.

use super::Effect;
use crate::display::PixelBuffer;
use crate::math3d::Vec3;
use crate::regions::Scene;
use crate::util::{hsv_to_rgb, Rng};
use std::f32::consts::PI;

const GRID_STEP: f32 = 50.0;
const PIPE_RADIUS: f32 = 12.0;
const MAX_SEGMENTS: usize = 40;
const GROW_INTERVAL: f32 = 0.12;
const NUM_PIPES: usize = 4;
/// Number of strips around the cylinder circumference (front-facing half)
const CYLINDER_STRIPS: usize = 8;

#[derive(Clone)]
struct Segment {
    a: Vec3,
    b: Vec3,
    hue: f32,
    dir: Vec3,
}

#[derive(Clone)]
struct Pipe {
    segments: Vec<Segment>,
    pos: Vec3,
    dir: Vec3,
    hue: f32,
}

pub struct Pipes {
    time: f32,
    pipes: Vec<Pipe>,
    grow_timer: f32,
    rotation: Vec3,
    rng: Rng,
}

const DIRECTIONS: [Vec3; 6] = [
    Vec3 {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    },
    Vec3 {
        x: -1.0,
        y: 0.0,
        z: 0.0,
    },
    Vec3 {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    },
    Vec3 {
        x: 0.0,
        y: -1.0,
        z: 0.0,
    },
    Vec3 {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    },
    Vec3 {
        x: 0.0,
        y: 0.0,
        z: -1.0,
    },
];

impl Pipes {
    pub fn new() -> Self {
        let mut rng = Rng::new(42);
        let pipes: Vec<Pipe> = (0..NUM_PIPES)
            .map(|i| {
                let pos = Vec3::new(
                    (rng.range_i32(-3, 3) as f32) * GRID_STEP,
                    (rng.range_i32(-3, 3) as f32) * GRID_STEP,
                    (rng.range_i32(-3, 3) as f32) * GRID_STEP,
                );
                Pipe {
                    segments: Vec::new(),
                    pos,
                    dir: DIRECTIONS[i % 6],
                    hue: i as f32 * 80.0,
                }
            })
            .collect();

        Self {
            time: 0.0,
            pipes,
            grow_timer: 0.0,
            rotation: Vec3::zero(),
            rng,
        }
    }

    fn grow_pipe(&mut self, pipe_idx: usize) {
        let pipe = &mut self.pipes[pipe_idx];
        let old_pos = pipe.pos;
        let current_dir = pipe.dir;
        let step = Vec3::new(
            current_dir.x * GRID_STEP,
            current_dir.y * GRID_STEP,
            current_dir.z * GRID_STEP,
        );
        pipe.pos = old_pos + step;

        pipe.segments.push(Segment {
            a: old_pos,
            b: pipe.pos,
            hue: pipe.hue,
            dir: current_dir,
        });

        if self.rng.next_f32() < 0.5 {
            let reverse = Vec3::new(-current_dir.x, -current_dir.y, -current_dir.z);
            loop {
                let idx = self.rng.next_u32() as usize % 6;
                let new_dir = DIRECTIONS[idx];
                if new_dir.x != reverse.x || new_dir.y != reverse.y || new_dir.z != reverse.z {
                    self.pipes[pipe_idx].dir = new_dir;
                    break;
                }
            }
        }

        if self.pipes[pipe_idx].segments.len() >= MAX_SEGMENTS {
            self.pipes[pipe_idx].segments.clear();
            self.pipes[pipe_idx].pos = Vec3::new(
                (self.rng.range_i32(-4, 4) as f32) * GRID_STEP,
                (self.rng.range_i32(-4, 4) as f32) * GRID_STEP,
                (self.rng.range_i32(-4, 4) as f32) * GRID_STEP,
            );
            self.pipes[pipe_idx].hue = (self.pipes[pipe_idx].hue + 60.0) % 360.0;
        }
    }
}

impl Default for Pipes {
    fn default() -> Self {
        Self::new()
    }
}

/// Projected segment data ready for rendering
struct ProjSeg {
    /// Screen positions at each endpoint
    sa: (f32, f32),
    sb: (f32, f32),
    /// Screen-space perpendicular direction
    nx: f32,
    ny: f32,
    /// Projected radii at each endpoint
    wa: f32,
    wb: f32,
    /// Base lighting intensity for this segment
    base_light: f32,
    hue: f32,
    avg_z: f32,
    is_bend: bool,
    bend_z: f32,
    bend_sx: f32,
    bend_sy: f32,
    bend_radius: f32,
}

impl Effect for Pipes {
    fn update(&mut self, dt: f32, _width: u32, _height: u32, _scene: &Scene) {
        self.time += dt;
        self.rotation.x += dt * 0.08;
        self.rotation.y += dt * 0.12;

        self.grow_timer += dt;
        while self.grow_timer >= GROW_INTERVAL {
            self.grow_timer -= GROW_INTERVAL;
            for i in 0..NUM_PIPES {
                self.grow_pipe(i);
            }
        }
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        let w = buffer.width() as f32;
        let h = buffer.height() as f32;
        let cx = w / 2.0;
        let cy = h / 2.0;
        let scale = h.min(w) / 480.0;
        let fov = 500.0 * scale;
        let camera_z = 500.0;
        let camera_offset = Vec3::new(0.0, 0.0, camera_z);
        let light_dir = Vec3::new(0.4, -0.6, -0.7).normalize();

        buffer.clear(5, 5, 12);

        // Project all segments and collect for depth sorting
        let mut projected: Vec<ProjSeg> = Vec::new();
        let rotation = &self.rotation;

        for pipe in &self.pipes {
            for (si, seg) in pipe.segments.iter().enumerate() {
                let a = seg.a.rotate_x(rotation.x).rotate_y(rotation.y) + camera_offset;
                let b = seg.b.rotate_x(rotation.x).rotate_y(rotation.y) + camera_offset;

                if a.z <= 1.0 || b.z <= 1.0 {
                    continue;
                }

                let sa = (cx + a.x * fov / a.z, cy + a.y * fov / a.z);
                let sb = (cx + b.x * fov / b.z, cy + b.y * fov / b.z);

                let dx = sb.0 - sa.0;
                let dy = sb.1 - sa.1;
                let len = (dx * dx + dy * dy).sqrt().max(0.001);
                let nx = -dy / len;
                let ny = dx / len;

                let wa = PIPE_RADIUS * fov / a.z;
                let wb = PIPE_RADIUS * fov / b.z;

                let seg_dir = (b - a).normalize();
                let base_light = 0.2 + 0.6 * seg_dir.dot(&light_dir).abs();

                // Check if this endpoint is a bend
                let is_bend = if si + 1 < pipe.segments.len() {
                    let next = &pipe.segments[si + 1];
                    (next.dir.x - seg.dir.x).abs() > 0.01
                        || (next.dir.y - seg.dir.y).abs() > 0.01
                        || (next.dir.z - seg.dir.z).abs() > 0.01
                } else {
                    true
                };

                let joint_r = PIPE_RADIUS * 1.4 * fov / b.z;

                projected.push(ProjSeg {
                    sa,
                    sb,
                    nx,
                    ny,
                    wa,
                    wb,
                    base_light,
                    hue: seg.hue,
                    avg_z: (a.z + b.z) * 0.5,
                    is_bend,
                    bend_z: b.z,
                    bend_sx: sb.0,
                    bend_sy: sb.1,
                    bend_radius: joint_r,
                });
            }
        }

        // Sort back-to-front
        projected.sort_by(|a, b| {
            b.avg_z
                .partial_cmp(&a.avg_z)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Render each segment as tessellated cylinder strips
        for ps in &projected {
            // Render cylinder: N strips around the front-facing semicircle
            // Each strip's brightness follows cosine falloff from surface normal
            for i in 0..CYLINDER_STRIPS {
                let theta0 = PI * (i as f32 / CYLINDER_STRIPS as f32);
                let theta1 = PI * ((i + 1) as f32 / CYLINDER_STRIPS as f32);

                // Position along perpendicular: cos(theta) maps [-1, 1]
                let offset0 = theta0.cos(); // +1 at left edge, -1 at right
                let offset1 = theta1.cos();

                // Surface normal for this strip (how much it faces the viewer)
                let mid_theta = (theta0 + theta1) * 0.5;
                let facing = mid_theta.sin(); // 1.0 at center, 0.0 at edges

                // Diffuse + ambient lighting
                let strip_bright = (ps.base_light * (0.3 + 0.7 * facing)).min(1.0);
                // Desaturate toward edges for a more plastic/metallic look
                let strip_sat = 0.55 + 0.15 * facing;

                let (r, g, b) = hsv_to_rgb(ps.hue, strip_sat, strip_bright);

                // Quad vertices: two points on this edge at endpoint A, two at endpoint B
                let verts: [(f32, f32); 4] = [
                    (
                        ps.sa.0 + ps.nx * ps.wa * offset0,
                        ps.sa.1 + ps.ny * ps.wa * offset0,
                    ),
                    (
                        ps.sb.0 + ps.nx * ps.wb * offset0,
                        ps.sb.1 + ps.ny * ps.wb * offset0,
                    ),
                    (
                        ps.sb.0 + ps.nx * ps.wb * offset1,
                        ps.sb.1 + ps.ny * ps.wb * offset1,
                    ),
                    (
                        ps.sa.0 + ps.nx * ps.wa * offset1,
                        ps.sa.1 + ps.ny * ps.wa * offset1,
                    ),
                ];

                let poly: Vec<(f32, f32)> = verts.to_vec();
                buffer.fill_polygon(&poly, r, g, b);
            }

            // Sphere joint at bends
            if ps.is_bend {
                let bright = (ps.base_light * 0.9).min(1.0);
                let (jr, jg, jb) = hsv_to_rgb(ps.hue, 0.5, bright);
                buffer.fill_circle_gradient(
                    ps.bend_sx as i32,
                    ps.bend_sy as i32,
                    ps.bend_radius as i32,
                    jr,
                    jg,
                    jb,
                    1.8,
                );
            }
        }
    }

    fn name(&self) -> &str {
        "3D Pipes"
    }
}
