//! Text Effect Primitives
//!
//! Pure functions for composing text effects. These are the building blocks
//! for demoscene-style animated text.
//!
//! # Categories
//! - `offset` - Position modifiers (wave, wobble, bounce)
//! - `color` - Color modifiers (rainbow, gradient, fade)
//! - `visibility` - Alpha/visibility modifiers (blink, pulse, strobe)
//! - `transform` - Geometric transforms (flip for reflection)

use super::font::{get_glyph, GLYPH_HEIGHT, GLYPH_WIDTH};
use super::PixelBuffer;

/// Offset functions - return (dx, dy) position modifiers
pub mod offset {
    /// Sine wave vertical offset (classic demo scroller)
    ///
    /// - `index`: character index in string
    /// - `time`: current time in seconds
    /// - `amplitude`: wave height in pixels
    /// - `frequency`: wave speed multiplier
    #[inline]
    pub fn wave(index: usize, time: f32, amplitude: f32, frequency: f32) -> (i32, i32) {
        let phase = index as f32 * 0.5 + time * frequency;
        let dy = (phase.sin() * amplitude) as i32;
        (0, dy)
    }

    /// Random jitter offset (glitchy effect)
    #[inline]
    pub fn wobble(index: usize, time: f32, amount: f32) -> (i32, i32) {
        // Use a pseudo-random function based on index and time
        let seed = (index as f32 * 127.1 + time * 43.7).sin() * 43758.5453;
        let dx = ((seed.fract() - 0.5) * 2.0 * amount) as i32;
        let seed2 = (index as f32 * 269.5 + time * 183.3).sin() * 43758.5453;
        let dy = ((seed2.fract() - 0.5) * 2.0 * amount) as i32;
        (dx, dy)
    }

    /// Bouncing offset (characters bounce up and down)
    #[inline]
    pub fn bounce(index: usize, time: f32, height: f32, speed: f32) -> (i32, i32) {
        let phase = index as f32 * 0.3 + time * speed;
        // Absolute sine for bounce effect (always positive, like a ball)
        let dy = -(phase.sin().abs() * height) as i32;
        (0, dy)
    }

    /// Spread characters apart over time
    #[inline]
    pub fn spread(index: usize, amount: f32, time: f32) -> (i32, i32) {
        let extra = (amount * time.sin().abs()) as i32;
        let dx = index as i32 * extra;
        (dx, 0)
    }

    /// Circular motion per character
    #[inline]
    pub fn circle(index: usize, time: f32, radius: f32, speed: f32) -> (i32, i32) {
        let phase = index as f32 * 0.4 + time * speed;
        let dx = (phase.cos() * radius) as i32;
        let dy = (phase.sin() * radius) as i32;
        (dx, dy)
    }
}

/// Color functions - return (r, g, b) color values
pub mod color {
    /// Rainbow color cycling per character
    #[inline]
    pub fn rainbow(index: usize, time: f32, speed: f32) -> (u8, u8, u8) {
        let hue = ((index as f32 * 30.0 + time * speed * 100.0) % 360.0).abs();
        crate::util::hsv_to_rgb(hue, 1.0, 1.0)
    }

    /// Gradient between two colors based on position
    #[inline]
    pub fn gradient(
        index: usize,
        total: usize,
        c1: (u8, u8, u8),
        c2: (u8, u8, u8),
    ) -> (u8, u8, u8) {
        if total <= 1 {
            return c1;
        }
        let t = index as f32 / (total - 1) as f32;
        (
            (c1.0 as f32 + (c2.0 as f32 - c1.0 as f32) * t) as u8,
            (c1.1 as f32 + (c2.1 as f32 - c1.1 as f32) * t) as u8,
            (c1.2 as f32 + (c2.2 as f32 - c1.2 as f32) * t) as u8,
        )
    }

    /// Fade color toward black
    #[inline]
    pub fn fade(color: (u8, u8, u8), amount: f32) -> (u8, u8, u8) {
        let amount = amount.clamp(0.0, 1.0);
        (
            (color.0 as f32 * amount) as u8,
            (color.1 as f32 * amount) as u8,
            (color.2 as f32 * amount) as u8,
        )
    }

    /// Pulse color brightness over time
    #[inline]
    pub fn pulse_brightness(color: (u8, u8, u8), time: f32, speed: f32, min: f32) -> (u8, u8, u8) {
        let t = (time * speed).sin() * 0.5 + 0.5; // 0 to 1
        let brightness = min + (1.0 - min) * t;
        fade(color, brightness)
    }

    /// Interpolate between two colors over time
    #[inline]
    pub fn lerp(c1: (u8, u8, u8), c2: (u8, u8, u8), t: f32) -> (u8, u8, u8) {
        let t = t.clamp(0.0, 1.0);
        (
            (c1.0 as f32 + (c2.0 as f32 - c1.0 as f32) * t) as u8,
            (c1.1 as f32 + (c2.1 as f32 - c1.1 as f32) * t) as u8,
            (c1.2 as f32 + (c2.2 as f32 - c1.2 as f32) * t) as u8,
        )
    }
}

/// Visibility functions - return 0.0 to 1.0 for alpha/brightness
pub mod visibility {
    /// Square wave blink (on/off)
    #[inline]
    pub fn blink(time: f32, rate: f32) -> f32 {
        if (time * rate) % 1.0 < 0.5 {
            1.0
        } else {
            0.0
        }
    }

    /// Smooth sine pulse (breathing effect)
    #[inline]
    pub fn pulse(time: f32, rate: f32) -> f32 {
        (time * rate * std::f32::consts::TAU).sin() * 0.5 + 0.5
    }

    /// Fast strobe (rapid blinking)
    #[inline]
    pub fn strobe(time: f32, rate: f32) -> f32 {
        blink(time, rate * 4.0)
    }

    /// One-shot flash then fade
    #[inline]
    pub fn flash(time: f32, start_time: f32, duration: f32) -> f32 {
        let elapsed = time - start_time;
        if elapsed < 0.0 {
            0.0
        } else if elapsed > duration {
            0.0
        } else {
            1.0 - (elapsed / duration)
        }
    }

    /// Sequential per-character blink (wave of visibility)
    #[inline]
    pub fn blink_seq(index: usize, time: f32, rate: f32, char_delay: f32) -> f32 {
        let offset_time = time - (index as f32 * char_delay);
        if offset_time < 0.0 {
            0.0
        } else {
            blink(offset_time, rate)
        }
    }

    /// Random per-character visibility (glitch effect)
    #[inline]
    pub fn blink_rand(index: usize, time: f32, rate: f32) -> f32 {
        let seed = (index as f32 * 127.1 + (time * rate).floor() * 311.7).sin() * 43758.5453;
        if seed.fract() > 0.3 {
            1.0
        } else {
            0.0
        }
    }

    /// Fade in over time (for reveals)
    #[inline]
    pub fn fade_in(time: f32, start: f32, duration: f32) -> f32 {
        ((time - start) / duration).clamp(0.0, 1.0)
    }

    /// Fade out over time
    #[inline]
    pub fn fade_out(time: f32, start: f32, duration: f32) -> f32 {
        1.0 - fade_in(time, start, duration)
    }
}

/// Transform functions for layer effects
pub mod transform {
    use super::*;

    /// Draw a single character vertically flipped (for reflections)
    pub fn draw_char_flipped(
        buffer: &mut PixelBuffer,
        x: i32,
        y: i32,
        ch: char,
        r: u8,
        g: u8,
        b: u8,
        scale: u32,
    ) {
        if let Some(glyph) = get_glyph(ch) {
            draw_glyph_flipped(buffer, x, y, glyph, r, g, b, scale);
        }
    }

    /// Draw a glyph vertically flipped
    pub fn draw_glyph_flipped(
        buffer: &mut PixelBuffer,
        x: i32,
        y: i32,
        glyph: &[u8; 8],
        r: u8,
        g: u8,
        b: u8,
        scale: u32,
    ) {
        // Draw rows in reverse order
        for (row, &bits) in glyph.iter().rev().enumerate() {
            for col in 0..8 {
                if bits & (1 << col) != 0 {
                    for sy in 0..scale as i32 {
                        for sx in 0..scale as i32 {
                            buffer.set_pixel(
                                x + col * scale as i32 + sx,
                                y + row as i32 * scale as i32 + sy,
                                r,
                                g,
                                b,
                            );
                        }
                    }
                }
            }
        }
    }

    /// Draw text vertically flipped (for reflections)
    pub fn draw_text_flipped(
        buffer: &mut PixelBuffer,
        x: i32,
        y: i32,
        text: &str,
        r: u8,
        g: u8,
        b: u8,
        scale: u32,
    ) {
        let char_width = (GLYPH_WIDTH * scale) as i32;
        let mut cursor_x = x;
        for ch in text.chars() {
            draw_char_flipped(buffer, cursor_x, y, ch, r, g, b, scale);
            cursor_x += char_width;
        }
    }

    /// Draw text with reflection below (lake effect)
    pub fn draw_text_reflected(
        buffer: &mut PixelBuffer,
        x: i32,
        y: i32,
        text: &str,
        color: (u8, u8, u8),
        scale: u32,
        gap: i32,
        reflection_fade: f32,
    ) {
        use super::super::font::draw_text_scaled;

        let text_height = (GLYPH_HEIGHT * scale) as i32;

        // Draw main text
        draw_text_scaled(buffer, x, y, text, color.0, color.1, color.2, scale);

        // Draw reflection (flipped and faded)
        let ref_y = y + text_height + gap;
        let faded = super::color::fade(color, reflection_fade);
        draw_text_flipped(buffer, x, ref_y, text, faded.0, faded.1, faded.2, scale);
    }

    /// Draw text with drop shadow
    pub fn draw_text_shadowed(
        buffer: &mut PixelBuffer,
        x: i32,
        y: i32,
        text: &str,
        color: (u8, u8, u8),
        shadow_color: (u8, u8, u8),
        scale: u32,
        offset_x: i32,
        offset_y: i32,
    ) {
        use super::super::font::draw_text_scaled;

        // Draw shadow first (behind)
        draw_text_scaled(
            buffer,
            x + offset_x,
            y + offset_y,
            text,
            shadow_color.0,
            shadow_color.1,
            shadow_color.2,
            scale,
        );

        // Draw main text on top
        draw_text_scaled(buffer, x, y, text, color.0, color.1, color.2, scale);
    }

    /// Draw text with outline
    pub fn draw_text_outlined(
        buffer: &mut PixelBuffer,
        x: i32,
        y: i32,
        text: &str,
        color: (u8, u8, u8),
        outline_color: (u8, u8, u8),
        scale: u32,
    ) {
        use super::super::font::draw_text_scaled;

        // Draw outline in 8 directions
        let offsets = [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ];

        for (dx, dy) in offsets {
            draw_text_scaled(
                buffer,
                x + dx,
                y + dy,
                text,
                outline_color.0,
                outline_color.1,
                outline_color.2,
                scale,
            );
        }

        // Draw main text on top
        draw_text_scaled(buffer, x, y, text, color.0, color.1, color.2, scale);
    }
}

// Re-export commonly used items at module level for external use
#[allow(unused_imports)]
pub use color::{fade, gradient, lerp, pulse_brightness, rainbow};
#[allow(unused_imports)]
pub use offset::{bounce, circle, spread, wave, wobble};
#[allow(unused_imports)]
pub use transform::{
    draw_char_flipped, draw_text_flipped, draw_text_outlined, draw_text_reflected,
    draw_text_shadowed,
};
#[allow(unused_imports)]
pub use visibility::{blink, blink_rand, blink_seq, fade_in, fade_out, flash, pulse, strobe};
