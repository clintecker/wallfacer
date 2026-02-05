//! Text Scroller System
//!
//! Classic demoscene-style scrolling text with various modes and effects.
//!
//! Use `Scroller` for basic scrolling, or `StyledScroller` to combine
//! scrolling with text_fx effects (rainbow, wave, shadow, etc.)

use super::font::{
    draw_char_scaled, draw_text_scaled, text_width_scaled, GLYPH_HEIGHT, GLYPH_WIDTH,
};
use super::text_fx::{
    color as fx_color, offset as fx_offset, transform as fx_transform, visibility as fx_vis,
};
use super::{PixelBuffer, DEFAULT_WIDTH};

/// Scroll direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    /// Text moves from right to left (classic news ticker)
    Leftward,
    /// Text moves from left to right
    Rightward,
}

/// Scroller mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollMode {
    /// Continuous loop - wraps around when off-screen
    Loop,
    /// Bounces back and forth between edges
    PingPong,
}

/// A scrolling text display
pub struct Scroller {
    text: String,
    x: f32,
    speed: f32, // pixels per second (always positive)
    direction: ScrollDirection,
    mode: ScrollMode,
    scale: u32,
    color: (u8, u8, u8),
    // For ping-pong mode
    ping_pong_dir: f32, // 1.0 or -1.0
    // Screen width (updated dynamically)
    screen_width: u32,
}

impl Scroller {
    /// Create a new scroller with default settings
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            text,
            x: DEFAULT_WIDTH as f32, // Start off-screen right
            speed: 100.0,
            direction: ScrollDirection::Leftward,
            mode: ScrollMode::Loop,
            scale: 1,
            color: (255, 255, 255),
            ping_pong_dir: -1.0,
            screen_width: DEFAULT_WIDTH,
        }
    }

    /// Set scroll speed in pixels per second
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed.abs();
        self
    }

    /// Set scroll direction
    pub fn direction(mut self, direction: ScrollDirection) -> Self {
        self.direction = direction;
        // Reset position based on direction
        match direction {
            ScrollDirection::Leftward => self.x = self.screen_width as f32,
            ScrollDirection::Rightward => self.x = -(self.text_pixel_width() as f32),
        }
        self.ping_pong_dir = match direction {
            ScrollDirection::Leftward => -1.0,
            ScrollDirection::Rightward => 1.0,
        };
        self
    }

    /// Set scroll mode
    pub fn mode(mut self, mode: ScrollMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set text scale
    pub fn scale(mut self, scale: u32) -> Self {
        self.scale = scale.max(1);
        self
    }

    /// Set text color
    pub fn color(mut self, r: u8, g: u8, b: u8) -> Self {
        self.color = (r, g, b);
        self
    }

    /// Get the pixel width of the text
    pub fn text_pixel_width(&self) -> u32 {
        text_width_scaled(&self.text, self.scale)
    }

    /// Get the pixel height of the text
    pub fn text_pixel_height(&self) -> u32 {
        GLYPH_HEIGHT * self.scale
    }

    /// Get current X position
    pub fn x(&self) -> f32 {
        self.x
    }

    /// Get text content
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Get scale
    pub fn get_scale(&self) -> u32 {
        self.scale
    }

    /// Get color
    pub fn get_color(&self) -> (u8, u8, u8) {
        self.color
    }

    /// Update scroller position
    pub fn update(&mut self, dt: f32) {
        let text_w = self.text_pixel_width() as f32;
        let width = self.screen_width as f32;

        match self.mode {
            ScrollMode::Loop => {
                match self.direction {
                    ScrollDirection::Leftward => {
                        self.x -= self.speed * dt;
                        // Seamless wrap using modulo
                        if self.x < -text_w {
                            self.x = self.x % text_w;
                        }
                    },
                    ScrollDirection::Rightward => {
                        self.x += self.speed * dt;
                        // Seamless wrap using modulo
                        if self.x > width {
                            self.x = self.x % text_w - text_w;
                        }
                    },
                }
            },
            ScrollMode::PingPong => {
                self.x += self.speed * dt * self.ping_pong_dir;

                // Bounce at edges
                let min_x = 0.0;
                let max_x = (width - text_w).max(0.0);

                if self.x <= min_x {
                    self.x = min_x;
                    self.ping_pong_dir = 1.0;
                } else if self.x >= max_x {
                    self.x = max_x;
                    self.ping_pong_dir = -1.0;
                }
            },
        }
    }

    /// Set screen width (call this when buffer size changes)
    pub fn set_screen_width(&mut self, width: u32) {
        self.screen_width = width;
        // Clamp position so scrollers don't start beyond the actual screen edge
        match self.direction {
            ScrollDirection::Leftward => {
                if self.x > width as f32 {
                    self.x = width as f32;
                }
            },
            ScrollDirection::Rightward => {
                let text_w = self.text_pixel_width() as f32;
                if self.x < -text_w {
                    self.x = -text_w;
                }
            },
        }
    }

    /// Render the scroller at the given Y position
    pub fn render(&self, buffer: &mut PixelBuffer, y: i32) {
        draw_text_scaled(
            buffer,
            self.x as i32,
            y,
            &self.text,
            self.color.0,
            self.color.1,
            self.color.2,
            self.scale,
        );
    }

    /// Render with a background strip
    pub fn render_with_background(
        &self,
        buffer: &mut PixelBuffer,
        y: i32,
        bg: (u8, u8, u8),
        padding: u32,
    ) {
        let strip_y = y - padding as i32;
        let strip_h = self.text_pixel_height() + padding * 2;

        // Draw background strip across full width
        buffer.fill_rect(0, strip_y, buffer.width(), strip_h, bg.0, bg.1, bg.2);

        // Draw text
        self.render(buffer, y);
    }
}

/// A typewriter-style text reveal
pub struct Typewriter {
    text: String,
    revealed: usize, // characters revealed
    timer: f32,
    chars_per_second: f32,
    scale: u32,
    color: (u8, u8, u8),
    complete: bool,
}

impl Typewriter {
    /// Create a new typewriter effect
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            revealed: 0,
            timer: 0.0,
            chars_per_second: 10.0,
            scale: 1,
            color: (255, 255, 255),
            complete: false,
        }
    }

    /// Set typing speed in characters per second
    pub fn speed(mut self, chars_per_second: f32) -> Self {
        self.chars_per_second = chars_per_second.max(0.1);
        self
    }

    /// Set text scale
    pub fn scale(mut self, scale: u32) -> Self {
        self.scale = scale.max(1);
        self
    }

    /// Set text color
    pub fn color(mut self, r: u8, g: u8, b: u8) -> Self {
        self.color = (r, g, b);
        self
    }

    /// Check if typing is complete
    pub fn is_complete(&self) -> bool {
        self.complete
    }

    /// Reset to beginning
    pub fn reset(&mut self) {
        self.revealed = 0;
        self.timer = 0.0;
        self.complete = false;
    }

    /// Update typewriter state
    pub fn update(&mut self, dt: f32) {
        if self.complete {
            return;
        }

        self.timer += dt;
        let chars_to_show = (self.timer * self.chars_per_second) as usize;
        self.revealed = chars_to_show.min(self.text.chars().count());

        if self.revealed >= self.text.chars().count() {
            self.complete = true;
        }
    }

    /// Render at position
    pub fn render(&self, buffer: &mut PixelBuffer, x: i32, y: i32) {
        let visible: String = self.text.chars().take(self.revealed).collect();
        draw_text_scaled(
            buffer,
            x,
            y,
            &visible,
            self.color.0,
            self.color.1,
            self.color.2,
            self.scale,
        );
    }

    /// Render centered
    pub fn render_centered(&self, buffer: &mut PixelBuffer, y: i32) {
        let visible: String = self.text.chars().take(self.revealed).collect();
        let text_width = text_width_scaled(&visible, self.scale) as i32;
        let x = (buffer.width() as i32 - text_width) / 2;
        self.render(buffer, x, y);
    }
}

/// A sine-wave scrolling text effect
pub struct SineScroller {
    scroller: Scroller,
    amplitude: f32, // wave height in pixels
    frequency: f32, // wave frequency
    time: f32,
}

impl SineScroller {
    /// Create a new sine-wave scroller
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            scroller: Scroller::new(text),
            amplitude: 20.0,
            frequency: 3.0,
            time: 0.0,
        }
    }

    /// Set wave amplitude (height in pixels)
    pub fn amplitude(mut self, amplitude: f32) -> Self {
        self.amplitude = amplitude;
        self
    }

    /// Set wave frequency
    pub fn frequency(mut self, frequency: f32) -> Self {
        self.frequency = frequency;
        self
    }

    /// Set scroll speed
    pub fn speed(mut self, speed: f32) -> Self {
        self.scroller = self.scroller.speed(speed);
        self
    }

    /// Set scroll direction
    pub fn direction(mut self, direction: ScrollDirection) -> Self {
        self.scroller = self.scroller.direction(direction);
        self
    }

    /// Set text scale
    pub fn scale(mut self, scale: u32) -> Self {
        self.scroller = self.scroller.scale(scale);
        self
    }

    /// Set text color
    pub fn color(mut self, r: u8, g: u8, b: u8) -> Self {
        self.scroller = self.scroller.color(r, g, b);
        self
    }

    /// Update scroller
    pub fn update(&mut self, dt: f32) {
        self.scroller.update(dt);
        self.time += dt;
    }

    /// Set screen width for proper wrapping
    pub fn set_screen_width(&mut self, width: u32) {
        self.scroller.set_screen_width(width);
    }

    /// Render with sine wave effect
    pub fn render(&self, buffer: &mut PixelBuffer, base_y: i32) {
        use super::font::{draw_char_scaled, GLYPH_WIDTH};

        let char_w = (GLYPH_WIDTH * self.scroller.scale) as i32;
        let text_width = self.scroller.text_pixel_width() as f32;
        let screen_w = self.scroller.screen_width as i32;

        // Draw multiple copies to seamlessly fill the screen
        let mut base_x = self.scroller.x;
        while (base_x as i32) < screen_w {
            for (i, ch) in self.scroller.text.chars().enumerate() {
                let x = base_x as i32 + i as i32 * char_w;

                // Skip if off screen
                if x + char_w < 0 {
                    continue;
                }
                if x > screen_w {
                    break;
                }

                // Calculate sine offset for this character
                let phase = (x as f32 * 0.02) + (self.time * self.frequency);
                let y_offset = (phase.sin() * self.amplitude) as i32;

                draw_char_scaled(
                    buffer,
                    x,
                    base_y + y_offset,
                    ch,
                    self.scroller.color.0,
                    self.scroller.color.1,
                    self.scroller.color.2,
                    self.scroller.scale,
                );
            }
            base_x += text_width;
        }
    }
}

// ============================================================================
// Styled Scroller - Combines scrolling with text_fx effects
// ============================================================================

/// Offset effect type for styled scrollers
#[derive(Debug, Clone, Copy)]
pub enum OffsetEffect {
    None,
    Wave { amplitude: f32, frequency: f32 },
    Wobble { amount: f32 },
    Bounce { height: f32, speed: f32 },
    Circle { radius: f32, speed: f32 },
}

/// Color effect type for styled scrollers
#[derive(Debug, Clone, Copy)]
pub enum ColorEffect {
    None,
    Rainbow {
        speed: f32,
    },
    Gradient {
        start: (u8, u8, u8),
        end: (u8, u8, u8),
    },
    Pulse {
        speed: f32,
        min_brightness: f32,
    },
}

/// Visibility effect type for styled scrollers
#[derive(Debug, Clone, Copy)]
pub enum VisibilityEffect {
    None,
    Blink { rate: f32 },
    Strobe { rate: f32 },
    BlinkSequential { rate: f32, delay: f32 },
    BlinkRandom { rate: f32 },
}

/// Layer effect type for styled scrollers
#[derive(Debug, Clone, Copy)]
pub enum LayerEffect {
    None,
    Shadow {
        offset_x: i32,
        offset_y: i32,
        color: (u8, u8, u8),
    },
    Outline {
        color: (u8, u8, u8),
    },
    Reflection {
        gap: i32,
        fade: f32,
    },
}

/// A scroller with composable text effects
pub struct StyledScroller {
    scroller: Scroller,
    time: f32,
    offset_effect: OffsetEffect,
    color_effect: ColorEffect,
    visibility_effect: VisibilityEffect,
    layer_effect: LayerEffect,
}

impl StyledScroller {
    /// Create a new styled scroller
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            scroller: Scroller::new(text),
            time: 0.0,
            offset_effect: OffsetEffect::None,
            color_effect: ColorEffect::None,
            visibility_effect: VisibilityEffect::None,
            layer_effect: LayerEffect::None,
        }
    }

    /// Set scroll speed
    pub fn speed(mut self, speed: f32) -> Self {
        self.scroller = self.scroller.speed(speed);
        self
    }

    /// Set scroll direction
    pub fn direction(mut self, direction: ScrollDirection) -> Self {
        self.scroller = self.scroller.direction(direction);
        self
    }

    /// Set scroll mode
    pub fn mode(mut self, mode: ScrollMode) -> Self {
        self.scroller = self.scroller.mode(mode);
        self
    }

    /// Set text scale
    pub fn scale(mut self, scale: u32) -> Self {
        self.scroller = self.scroller.scale(scale);
        self
    }

    /// Set base text color
    pub fn color(mut self, r: u8, g: u8, b: u8) -> Self {
        self.scroller = self.scroller.color(r, g, b);
        self
    }

    /// Set offset effect (wave, wobble, bounce, etc.)
    pub fn offset(mut self, effect: OffsetEffect) -> Self {
        self.offset_effect = effect;
        self
    }

    /// Set color effect (rainbow, gradient, pulse)
    pub fn color_fx(mut self, effect: ColorEffect) -> Self {
        self.color_effect = effect;
        self
    }

    /// Set visibility effect (blink, strobe, etc.)
    pub fn visibility(mut self, effect: VisibilityEffect) -> Self {
        self.visibility_effect = effect;
        self
    }

    /// Set layer effect (shadow, outline, reflection)
    pub fn layer(mut self, effect: LayerEffect) -> Self {
        self.layer_effect = effect;
        self
    }

    /// Update scroller
    pub fn update(&mut self, dt: f32) {
        self.scroller.update(dt);
        self.time += dt;
    }

    /// Set screen width for proper wrapping
    pub fn set_screen_width(&mut self, width: u32) {
        self.scroller.set_screen_width(width);
    }

    /// Calculate offset for a character
    fn char_offset(&self, index: usize) -> (i32, i32) {
        match self.offset_effect {
            OffsetEffect::None => (0, 0),
            OffsetEffect::Wave {
                amplitude,
                frequency,
            } => fx_offset::wave(index, self.time, amplitude, frequency),
            OffsetEffect::Wobble { amount } => fx_offset::wobble(index, self.time, amount),
            OffsetEffect::Bounce { height, speed } => {
                fx_offset::bounce(index, self.time, height, speed)
            },
            OffsetEffect::Circle { radius, speed } => {
                fx_offset::circle(index, self.time, radius, speed)
            },
        }
    }

    /// Calculate color for a character
    fn char_color(&self, index: usize, total: usize) -> (u8, u8, u8) {
        match self.color_effect {
            ColorEffect::None => self.scroller.get_color(),
            ColorEffect::Rainbow { speed } => fx_color::rainbow(index, self.time, speed),
            ColorEffect::Gradient { start, end } => fx_color::gradient(index, total, start, end),
            ColorEffect::Pulse {
                speed,
                min_brightness,
            } => fx_color::pulse_brightness(
                self.scroller.get_color(),
                self.time,
                speed,
                min_brightness,
            ),
        }
    }

    /// Calculate visibility for a character
    fn char_visibility(&self, index: usize) -> f32 {
        match self.visibility_effect {
            VisibilityEffect::None => 1.0,
            VisibilityEffect::Blink { rate } => fx_vis::blink(self.time, rate),
            VisibilityEffect::Strobe { rate } => fx_vis::strobe(self.time, rate),
            VisibilityEffect::BlinkSequential { rate, delay } => {
                fx_vis::blink_seq(index, self.time, rate, delay)
            },
            VisibilityEffect::BlinkRandom { rate } => fx_vis::blink_rand(index, self.time, rate),
        }
    }

    /// Render the styled scroller
    pub fn render(&self, buffer: &mut PixelBuffer, base_y: i32) {
        let scale = self.scroller.get_scale();
        let _char_w = (GLYPH_WIDTH * scale) as i32;
        let text = self.scroller.text();
        let _char_count = text.chars().count();

        // Handle layer effects that need pre-pass
        match self.layer_effect {
            LayerEffect::Shadow {
                offset_x,
                offset_y,
                color,
            } => {
                // Draw shadow first
                self.render_chars(buffer, base_y, Some((offset_x, offset_y)), Some(color));
            },
            LayerEffect::Outline { color } => {
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
                for (ox, oy) in offsets {
                    self.render_chars(buffer, base_y, Some((ox, oy)), Some(color));
                }
            },
            LayerEffect::Reflection { gap, fade } => {
                // Draw main text first, then reflection below
                self.render_chars(buffer, base_y, None, None);
                self.render_reflection(buffer, base_y, gap, fade);
                return; // Already drew main text
            },
            LayerEffect::None => {},
        }

        // Draw main text
        self.render_chars(buffer, base_y, None, None);
    }

    /// Render characters with optional offset and color override
    fn render_chars(
        &self,
        buffer: &mut PixelBuffer,
        base_y: i32,
        pos_offset: Option<(i32, i32)>,
        color_override: Option<(u8, u8, u8)>,
    ) {
        let scale = self.scroller.get_scale();
        let char_w = (GLYPH_WIDTH * scale) as i32;
        let text = self.scroller.text();
        let char_count = text.chars().count();
        let mut x = self.scroller.x() as i32;

        let (ox, oy) = pos_offset.unwrap_or((0, 0));

        for (i, ch) in text.chars().enumerate() {
            let vis = self.char_visibility(i);
            if vis < 0.01 {
                x += char_w;
                continue;
            }

            let (dx, dy) = self.char_offset(i);
            let (mut r, mut g, mut b) =
                color_override.unwrap_or_else(|| self.char_color(i, char_count));

            // Apply visibility as brightness
            if vis < 1.0 {
                r = (r as f32 * vis) as u8;
                g = (g as f32 * vis) as u8;
                b = (b as f32 * vis) as u8;
            }

            draw_char_scaled(buffer, x + dx + ox, base_y + dy + oy, ch, r, g, b, scale);

            x += char_w;
        }
    }

    /// Render reflection (flipped text below)
    fn render_reflection(&self, buffer: &mut PixelBuffer, base_y: i32, gap: i32, fade: f32) {
        let scale = self.scroller.get_scale();
        let char_w = (GLYPH_WIDTH * scale) as i32;
        let text_height = (GLYPH_HEIGHT * scale) as i32;
        let text = self.scroller.text();
        let char_count = text.chars().count();
        let mut x = self.scroller.x() as i32;

        let reflect_y = base_y + text_height + gap;

        for (i, ch) in text.chars().enumerate() {
            let vis = self.char_visibility(i) * fade;
            if vis < 0.01 {
                x += char_w;
                continue;
            }

            let (dx, dy) = self.char_offset(i);
            let (mut r, mut g, mut b) = self.char_color(i, char_count);

            // Apply fade
            r = (r as f32 * vis) as u8;
            g = (g as f32 * vis) as u8;
            b = (b as f32 * vis) as u8;

            fx_transform::draw_char_flipped(
                buffer,
                x + dx,
                reflect_y - dy, // Invert dy for reflection
                ch,
                r,
                g,
                b,
                scale,
            );

            x += char_w;
        }
    }

    /// Render with a background strip
    pub fn render_with_background(
        &self,
        buffer: &mut PixelBuffer,
        y: i32,
        bg: (u8, u8, u8),
        padding: u32,
    ) {
        let strip_y = y - padding as i32;
        let mut strip_h = (GLYPH_HEIGHT * self.scroller.get_scale()) + padding * 2;

        // Extra space for reflection
        if let LayerEffect::Reflection { gap, .. } = self.layer_effect {
            strip_h += (GLYPH_HEIGHT * self.scroller.get_scale()) + gap as u32;
        }

        // Extra space for wave amplitude
        if let OffsetEffect::Wave { amplitude, .. } = self.offset_effect {
            strip_h += (amplitude * 2.0) as u32;
        }

        buffer.fill_rect(0, strip_y, buffer.width(), strip_h, bg.0, bg.1, bg.2);
        self.render(buffer, y);
    }
}
