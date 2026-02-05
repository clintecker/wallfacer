//! Tunnel Effect
//!
//! Flying through an infinite curving tunnel with parallax occlusion.
//!
//! Techniques:
//! - Precomputed polar LUTs (atan2/sqrt at init, table lookup at render)
//! - Depth-dependent parallax: screen-edge pixels shift by the curve amount,
//!   center pixels stay still — walls occlude the vanishing point on sharp turns
//! - Brick wall texture mapped in tunnel space (angle=U, depth=V)
//! - IndexedMipTexture for anti-aliased texture at distance
//! - 2D palette: pre-baked luminance × hue lookup (no per-pixel RGB muls)
//! - Single distance fade as sole darkening stage (sqrt curve with floor)
//! - Additive ceiling/lamp highlights for dramatic lighting without dimming

use super::Effect;
use crate::display::PixelBuffer;
use crate::regions::Scene;
use crate::texture::{IndexedMipTexture, Texture};
use crate::util::hsv_to_rgb;
use std::f32::consts::TAU;

const PALETTE_SIZE: usize = 256;
const LUT_PAD: i32 = 450;
const SINE_TABLE_SIZE: usize = 256;

fn build_sine_table() -> [u8; SINE_TABLE_SIZE] {
    let mut table = [0u8; SINE_TABLE_SIZE];
    for (i, entry) in table.iter_mut().enumerate() {
        let angle = (i as f32 / SINE_TABLE_SIZE as f32) * TAU;
        *entry = ((angle.sin() * 0.5 + 0.5) * 255.0) as u8;
    }
    table
}

/// Brick wall texture — offset mortar rows with per-brick variation
fn build_wall_texture() -> Texture {
    let size = 256u32;
    let mut tex = Texture::new(size, size);

    let brick_w = 64u32;
    let brick_h = 32u32;

    for y in 0..size {
        for x in 0..size {
            let row = y / brick_h;
            let offset = if row % 2 == 0 { 0 } else { brick_w / 2 };
            let bx = (x + offset) % brick_w;
            let by = y % brick_h;

            let mortar = bx < 4 || by < 4;

            let v = if mortar {
                40u8
            } else {
                let brick_id = ((y / brick_h) * 17 + ((x + offset) / brick_w) * 31) & 0xFF;
                let base = 130u16 + (brick_id & 0x3F) as u16;
                let cx = (bx as i32 - brick_w as i32 / 2).unsigned_abs().min(brick_w);
                let cy = (by as i32 - brick_h as i32 / 2).unsigned_abs().min(brick_h);
                let edge_dim = (cx / 8 + cy / 4) as u16;
                (base - edge_dim.min(base)).min(255) as u8
            };

            tex.set_pixel(x, y, v, v, v, 255);
        }
    }
    tex
}

/// Build 2D palette: palette_2d[hue * 256 + luminance] = pre-baked RGB
fn build_palette_2d() -> Vec<(u8, u8, u8)> {
    let mut palette_2d = Vec::with_capacity(PALETTE_SIZE * 256);
    for h in 0..PALETTE_SIZE {
        let hue = (h as f32 / PALETTE_SIZE as f32) * 360.0;
        let (pr, pg, pb) = hsv_to_rgb(hue, 0.7, 0.95);
        for l in 0..256u16 {
            palette_2d.push((
                ((l * pr as u16) >> 8) as u8,
                ((l * pg as u16) >> 8) as u8,
                ((l * pb as u16) >> 8) as u8,
            ));
        }
    }
    palette_2d
}

pub struct Tunnel {
    time: f32,
    speed: f32,
    rotation: f32,
    color_cycle: f32,
    lut_distance: Vec<f32>,
    lut_angle: Vec<f32>,
    lut_shade: Vec<u16>,
    lut_w: i32,
    lut_h: i32,
    screen_w: u32,
    screen_h: u32,
    lut_proximity: Vec<u8>,
    curve_x: f32,
    curve_y: f32,
    sine_lut: [u8; SINE_TABLE_SIZE],
    wall_mip: IndexedMipTexture,
    palette_2d: Vec<(u8, u8, u8)>,
}

impl Tunnel {
    pub fn new() -> Self {
        let wall_texture = build_wall_texture();
        let wall_mip = IndexedMipTexture::from_grayscale(&wall_texture);
        let palette_2d = build_palette_2d();

        Self {
            time: 0.0,
            speed: 0.7,
            rotation: 0.12,
            color_cycle: 0.12,
            lut_distance: Vec::new(),
            lut_angle: Vec::new(),
            lut_shade: Vec::new(),
            lut_w: 0,
            lut_h: 0,
            screen_w: 0,
            screen_h: 0,
            lut_proximity: Vec::new(),
            curve_x: 0.0,
            curve_y: 0.0,
            sine_lut: build_sine_table(),
            wall_mip,
            palette_2d,
        }
    }

    fn build_luts(&mut self, width: u32, height: u32) {
        if self.screen_w == width && self.screen_h == height {
            return;
        }

        let lut_w = width as i32 + LUT_PAD * 2;
        let lut_h = height as i32 + LUT_PAD * 2;
        let size = (lut_w * lut_h) as usize;

        self.lut_distance = Vec::with_capacity(size);
        self.lut_angle = Vec::with_capacity(size);
        self.lut_shade = Vec::with_capacity(size);

        let cx = lut_w as f32 / 2.0;
        let cy = lut_h as f32 / 2.0;
        let half_dim = cx.min(cy);

        for y in 0..lut_h {
            let dy = y as f32 - cy;
            for x in 0..lut_w {
                let dx = x as f32 - cx;
                let distance = (dx * dx + dy * dy).sqrt();
                let angle = dy.atan2(dx);

                let depth = if distance < 1.0 {
                    0.0
                } else {
                    200.0 / distance
                };
                self.lut_distance.push(depth);
                self.lut_angle.push(angle / TAU);

                // Fog: sqrt curve with floor — nearby walls at full brightness,
                // vanishing point dim but never fully black
                let t = (distance / half_dim).min(1.0);
                let shade = (30.0 + t.sqrt() * 226.0) as u16;
                self.lut_shade.push(shade);
            }
        }

        // Quadratic proximity — more of the screen participates in curve shifts
        let scx = width as f32 / 2.0;
        let scy = height as f32 / 2.0;
        let max_d = (scx * scx + scy * scy).sqrt();

        self.lut_proximity = Vec::with_capacity((width * height) as usize);
        for y in 0..height as i32 {
            let dy = y as f32 - scy;
            for x in 0..width as i32 {
                let dx = x as f32 - scx;
                let d = (dx * dx + dy * dy).sqrt() / max_d;
                let proximity = (d * d * 255.0).min(255.0) as u8;
                self.lut_proximity.push(proximity);
            }
        }

        self.lut_w = lut_w;
        self.lut_h = lut_h;
        self.screen_w = width;
        self.screen_h = height;
    }

    fn compute_curve(&mut self) {
        let t = self.time;
        // Scale curve amplitudes proportionally to screen size
        // (designed for 640x480 — at 320x240 amplitudes halve, etc.)
        let sx = self.screen_w as f32 / 640.0;
        let sy = self.screen_h as f32 / 480.0;
        self.curve_x =
            ((t * 0.23).sin() * 220.0 + (t * 0.51).sin() * 100.0 + (t * 0.89).sin() * 40.0) * sx;
        self.curve_y =
            ((t * 0.19).cos() * 180.0 + (t * 0.43).cos() * 80.0 + (t * 0.71).cos() * 30.0) * sy;
    }
}

impl Default for Tunnel {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Tunnel {
    fn update(&mut self, dt: f32, width: u32, height: u32, _scene: &Scene) {
        self.time += dt;
        self.build_luts(width, height);
        self.compute_curve();
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        let pixels = buffer.as_bytes_mut();
        let sw = self.screen_w as i32;
        let sh = self.screen_h as i32;
        let lut_w = self.lut_w;
        let lut_h = self.lut_h;

        let depth_offset = self.time * self.speed;
        let angle_offset = self.time * self.rotation / TAU;
        let color_offset = self.time * self.color_cycle * 100.0;
        let palette_mask = PALETTE_SIZE - 1;

        let curve_xi = self.curve_x as i32;
        let curve_yi = self.curve_y as i32;

        // Lamp animation: pools of light sweep past
        let lamp_phase = (self.time * self.speed * 3.0 * 256.0) as i32;

        let mut pixel_idx = 0;
        let mut screen_idx = 0;

        for sy in 0..sh {
            for sx in 0..sw {
                let proximity = self.lut_proximity[screen_idx] as i32;
                let shift_x = (curve_xi * proximity) >> 8;
                let shift_y = (curve_yi * proximity) >> 8;

                let lx = sx + LUT_PAD - shift_x;
                let ly = sy + LUT_PAD - shift_y;
                screen_idx += 1;

                if lx < 0 || lx >= lut_w || ly < 0 || ly >= lut_h {
                    pixels[pixel_idx] = 255;
                    pixels[pixel_idx + 1] = 0;
                    pixels[pixel_idx + 2] = 0;
                    pixels[pixel_idx + 3] = 0;
                    pixel_idx += 4;
                    continue;
                }

                let li = (ly * lut_w + lx) as usize;
                let depth = self.lut_distance[li];

                if depth == 0.0 {
                    pixels[pixel_idx] = 255;
                    pixels[pixel_idx + 1] = 0;
                    pixels[pixel_idx + 2] = 0;
                    pixels[pixel_idx + 3] = 0;
                    pixel_idx += 4;
                    continue;
                }

                let angle = self.lut_angle[li];
                let d = depth + depth_offset;
                let a = angle + angle_offset;

                // --- Wall texture via mipped index lookup ---
                let tex_x = (a * 256.0) as i32;
                let tex_y = (d * 48.0) as i32;
                let mip = u32::from(depth > 3.0);
                let lum = self.wall_mip.sample_index_mipped(tex_x, tex_y, mip);

                // --- 2D palette: pre-baked luminance × hue color ---
                let hue_idx = (d * 12.0 + color_offset) as usize & palette_mask;
                let (pr, pg, pb) = self.palette_2d[hue_idx * 256 + lum as usize];

                // --- Distance fog: single darkening stage ---
                let fog = self.lut_shade[li];
                let mut r = ((pr as u16 * fog) >> 8) as u8;
                let mut g = ((pg as u16 * fog) >> 8) as u8;
                let mut b = ((pb as u16 * fog) >> 8) as u8;

                // --- Additive lighting highlights ---
                // Overhead strip: bright at ceiling (a ≈ 0.25), dim at floor
                let overhead_idx = (a * 256.0) as usize & 255;
                let overhead_raw = self.sine_lut[overhead_idx] as u16;
                let overhead = (overhead_raw * overhead_raw) >> 8;

                // Periodic lamp pools along ceiling
                let lamp_idx = ((d * 256.0) as i32 + lamp_phase) as usize & 255;
                let lamp_raw = self.sine_lut[lamp_idx] as u16;
                let lamp = (lamp_raw * lamp_raw) >> 8;
                let lamp = (lamp * lamp) >> 8;

                // Lamps only glow on the ceiling
                let lamp_on_ceiling = (lamp * overhead) >> 8;

                // Additive highlight: overhead glow + lamp pools (warm light)
                let highlight = ((overhead * 35 + lamp_on_ceiling * 55) >> 8) as u8;

                r = r.saturating_add(highlight);
                g = g.saturating_add(highlight);
                b = b.saturating_add(highlight);

                pixels[pixel_idx] = 255;
                pixels[pixel_idx + 1] = b;
                pixels[pixel_idx + 2] = g;
                pixels[pixel_idx + 3] = r;
                pixel_idx += 4;
            }
        }
    }

    fn name(&self) -> &str {
        "Tunnel"
    }
}
