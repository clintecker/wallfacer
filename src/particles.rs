//! Particle System for Demoscene Effects
//!
//! Provides particle emission, physics, and rendering.

use crate::display::PixelBuffer;
use crate::math3d::Vec2;

/// A single particle
#[derive(Clone)]
pub struct Particle {
    pub pos: Vec2,
    pub vel: Vec2,
    pub life: f32,
    pub max_life: f32,
    pub color: (u8, u8, u8),
    pub size: f32,
}

impl Particle {
    pub fn new(pos: Vec2, vel: Vec2, life: f32, color: (u8, u8, u8)) -> Self {
        Self {
            pos,
            vel,
            life,
            max_life: life,
            color,
            size: 1.0,
        }
    }

    /// Create a particle with custom size
    pub fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Get normalized age (0 = just born, 1 = about to die)
    #[inline]
    pub fn age(&self) -> f32 {
        1.0 - (self.life / self.max_life)
    }

    /// Check if particle is still alive
    #[inline]
    pub fn is_alive(&self) -> bool {
        self.life > 0.0
    }
}

/// Particle system manager
pub struct ParticleSystem {
    particles: Vec<Particle>,
    gravity: Vec2,
    drag: f32,
}

impl ParticleSystem {
    pub fn new() -> Self {
        Self {
            particles: Vec::with_capacity(1000),
            gravity: Vec2::new(0.0, 0.0),
            drag: 0.0,
        }
    }

    /// Set gravity (applied each frame)
    pub fn with_gravity(mut self, gravity: Vec2) -> Self {
        self.gravity = gravity;
        self
    }

    /// Set drag (velocity reduction per second, 0.99 = slight drag)
    pub fn with_drag(mut self, drag: f32) -> Self {
        self.drag = drag;
        self
    }

    /// Emit a single particle
    pub fn emit(&mut self, particle: Particle) {
        self.particles.push(particle);
    }

    /// Emit particles in a burst from a point
    pub fn emit_burst(
        &mut self,
        pos: Vec2,
        count: usize,
        speed: f32,
        life: f32,
        color: (u8, u8, u8),
    ) {
        use std::f32::consts::TAU;

        for i in 0..count {
            let angle = (i as f32 / count as f32) * TAU;
            let vel = Vec2::new(angle.cos() * speed, angle.sin() * speed);
            self.emit(Particle::new(pos, vel, life, color));
        }
    }

    /// Emit particles in a random burst
    pub fn emit_burst_random(
        &mut self,
        pos: Vec2,
        count: usize,
        speed: f32,
        spread: f32,
        life: f32,
        color: (u8, u8, u8),
        rng: &mut impl FnMut() -> f32,
    ) {
        use std::f32::consts::TAU;

        for _ in 0..count {
            let angle = rng() * TAU;
            let s = speed * (1.0 - spread + spread * rng());
            let vel = Vec2::new(angle.cos() * s, angle.sin() * s);
            let l = life * (0.5 + 0.5 * rng());
            self.emit(Particle::new(pos, vel, l, color));
        }
    }

    /// Update all particles
    pub fn update(&mut self, dt: f32) {
        for p in &mut self.particles {
            if !p.is_alive() {
                continue;
            }

            // Apply velocity
            p.pos = p.pos + p.vel * dt;

            // Apply gravity
            p.vel = p.vel + self.gravity * dt;

            // Apply drag (clamped to prevent velocity reversal)
            if self.drag > 0.0 {
                let drag_factor = (1.0 - self.drag * dt).max(0.0);
                p.vel = p.vel * drag_factor;
            }

            // Age
            p.life -= dt;
        }

        // Remove dead particles using swap-remove for better performance
        // (avoids shifting memory when removing from middle of Vec)
        let mut i = 0;
        while i < self.particles.len() {
            if self.particles[i].is_alive() {
                i += 1;
            } else {
                self.particles.swap_remove(i);
            }
        }
    }

    /// Render all particles as single pixels with alpha fade
    ///
    /// Note: This always renders 1-pixel particles regardless of `Particle::size`.
    /// Use `render_circles()` for size-aware rendering.
    pub fn render(&self, buffer: &mut PixelBuffer) {
        for p in &self.particles {
            let alpha = (p.life / p.max_life * 255.0) as u8;
            buffer.blend_pixel(
                p.pos.x as i32,
                p.pos.y as i32,
                p.color.0,
                p.color.1,
                p.color.2,
                alpha,
            );
        }
    }

    /// Render particles as circles (for larger effects)
    pub fn render_circles(&self, buffer: &mut PixelBuffer) {
        for p in &self.particles {
            let alpha = (p.life / p.max_life * 255.0) as u8;
            let radius = (p.size * (1.0 - p.age() * 0.5)) as i32;
            if radius > 0 {
                buffer.fill_circle_blend(
                    p.pos.x as i32,
                    p.pos.y as i32,
                    radius,
                    p.color.0,
                    p.color.1,
                    p.color.2,
                    alpha,
                );
            }
        }
    }

    /// Render with additive blending (for sparks/glow)
    pub fn render_additive(&self, buffer: &mut PixelBuffer) {
        for p in &self.particles {
            let intensity = p.life / p.max_life;
            buffer.blend_pixel_additive(
                p.pos.x as i32,
                p.pos.y as i32,
                (p.color.0 as f32 * intensity) as u8,
                (p.color.1 as f32 * intensity) as u8,
                (p.color.2 as f32 * intensity) as u8,
            );
        }
    }

    /// Get particle count
    pub fn count(&self) -> usize {
        self.particles.len()
    }

    /// Clear all particles
    pub fn clear(&mut self) {
        self.particles.clear();
    }
}

impl Default for ParticleSystem {
    fn default() -> Self {
        Self::new()
    }
}
