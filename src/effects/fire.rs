use super::color::fire_palette;
use super::Effect;
use crate::display::PixelBuffer;
use crate::regions::Scene;
use crate::util::Rng;

/// Target fire pixel size (keeps the chunky retro look)
const FIRE_SCALE: u32 = 4;
/// Fixed simulation rate (60 updates per second)
const SIM_STEP: f32 = 1.0 / 60.0;

/// Classic demoscene-style fire effect
pub struct Fire {
    buffer: Vec<u8>,
    palette: Vec<(u8, u8, u8)>,
    time: f32,
    sim_accum: f32, // Accumulator for fixed timestep simulation
    rng: Rng,
    // Dynamic dimensions
    fire_w: usize,
    fire_h: usize,
    scale: u32,
}

impl Fire {
    pub fn new() -> Self {
        // Start with defaults that will be resized on first update
        let fire_w = 160;
        let fire_h = 120;
        Self {
            buffer: vec![0; fire_w * fire_h],
            palette: fire_palette(),
            time: 0.0,
            sim_accum: 0.0,
            rng: Rng::new(0x1234ABCD),
            fire_w,
            fire_h,
            scale: FIRE_SCALE,
        }
    }

    /// Resize fire buffer for new screen dimensions
    fn resize(&mut self, width: u32, height: u32) {
        // Calculate fire buffer dimensions to fill screen with target scale
        let new_w = (width / self.scale) as usize;
        let new_h = (height / self.scale) as usize;

        if new_w != self.fire_w || new_h != self.fire_h {
            self.fire_w = new_w.max(1);
            self.fire_h = new_h.max(1);
            self.buffer = vec![0; self.fire_w * self.fire_h];
        }
    }
}

impl Default for Fire {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for Fire {
    fn update(&mut self, dt: f32, width: u32, height: u32, _scene: &Scene) {
        // Resize buffer if screen dimensions changed
        self.resize(width, height);

        self.time += dt;
        self.sim_accum += dt;

        let fire_w = self.fire_w;
        let fire_h = self.fire_h;

        // Fixed timestep simulation - run at consistent rate regardless of framerate
        // Cap iterations to prevent spiral of death if framerate tanks
        let max_steps = 4;
        let mut steps = 0;

        while self.sim_accum >= SIM_STEP && steps < max_steps {
            self.sim_accum -= SIM_STEP;
            steps += 1;

            // Seed the bottom row with heat
            for x in 0..fire_w {
                let flicker = self.rng.next_u8() % 80;
                let wave = ((self.time * 6.0 + x as f32 * 0.08).sin() * 40.0) as i32;
                let base = (180 + wave).clamp(120, 255) as u8;
                self.buffer[(fire_h - 1) * fire_w + x] = base.saturating_add(flicker);
            }

            // Propagate heat upwards
            let wind = (self.time * 2.5).sin() * 2.0;
            for y in 1..fire_h {
                for x in 0..fire_w {
                    let wind_x = (x as i32 + wind as i32 + fire_w as i32) % fire_w as i32;
                    let x0 = wind_x as usize;
                    let x1 = (x0 + fire_w - 1) % fire_w;
                    let x2 = (x0 + 1) % fire_w;

                    let below = self.buffer[y * fire_w + x0] as u16;
                    let below_left = self.buffer[y * fire_w + x1] as u16;
                    let below_right = self.buffer[y * fire_w + x2] as u16;
                    let below2 = if y + 1 < fire_h {
                        self.buffer[(y + 1) * fire_w + x0] as u16
                    } else {
                        below
                    };

                    let mut heat = (below + below_left + below_right + below2) / 4;
                    let cooling = 1 + (self.rng.next_u8() % 3) as u16;
                    heat = heat.saturating_sub(cooling);
                    self.buffer[(y - 1) * fire_w + x] = heat as u8;
                }
            }
        }

        // If we hit max steps, drain remaining accumulator to prevent buildup
        if steps >= max_steps {
            self.sim_accum = 0.0;
        }
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        buffer.clear(0, 0, 0);

        let fire_w = self.fire_w;
        let fire_h = self.fire_h;
        let scale = self.scale as i32;

        for y in 0..fire_h {
            let wave = (self.time * 3.0 + y as f32 * 0.1).sin() * 2.0;
            for x in 0..fire_w {
                let heat = self.buffer[y * fire_w + x] as usize;
                let (r, g, b) = self.palette[heat.min(255)];
                let px = x as i32 * scale + wave as i32;
                let py = y as i32 * scale;
                buffer.fill_rect(px, py, self.scale, self.scale, r, g, b);
            }
        }
    }

    fn name(&self) -> &str {
        "Fire"
    }

    fn region_color(&self) -> (u8, u8, u8) {
        // Deep ember glow
        (30, 8, 0)
    }
}
