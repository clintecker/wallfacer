use super::Effect;
use crate::display::text_fx::{color, offset, transform, visibility};
use crate::display::{draw_char_scaled, draw_text, PixelBuffer, GLYPH_HEIGHT, GLYPH_WIDTH};
use crate::regions::Scene;

/// Demo effect showcasing all text_fx primitives
pub struct TextFxDemo {
    time: f32,
}

impl TextFxDemo {
    pub fn new() -> Self {
        Self { time: 0.0 }
    }
}

impl Default for TextFxDemo {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for TextFxDemo {
    fn update(&mut self, dt: f32, _width: u32, _height: u32, _scene: &Scene) {
        self.time += dt;
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        buffer.clear(15, 15, 25);

        let scale: u32 = 2;
        let char_w = (GLYPH_WIDTH * scale) as i32;
        let line_h = (GLYPH_HEIGHT * scale) as i32 + 8;
        let mut y = 16;

        // === OFFSET EFFECTS ===

        // 1. Wave
        draw_label(buffer, 10, y, "Wave:");
        let text = "WAVE TEXT";
        let x = 100;
        for (i, ch) in text.chars().enumerate() {
            let (dx, dy) = offset::wave(i, self.time, 8.0, 4.0);
            draw_char_scaled(
                buffer,
                x + i as i32 * char_w + dx,
                y + dy,
                ch,
                255,
                255,
                100,
                scale,
            );
        }
        y += line_h;

        // 2. Wobble
        draw_label(buffer, 10, y, "Wobble:");
        let text = "GLITCHY";
        let x = 100;
        for (i, ch) in text.chars().enumerate() {
            let (dx, dy) = offset::wobble(i, self.time, 3.0);
            draw_char_scaled(
                buffer,
                x + i as i32 * char_w + dx,
                y + dy,
                ch,
                255,
                100,
                100,
                scale,
            );
        }
        y += line_h;

        // 3. Bounce
        draw_label(buffer, 10, y, "Bounce:");
        let text = "BOUNCY";
        let x = 100;
        for (i, ch) in text.chars().enumerate() {
            let (dx, dy) = offset::bounce(i, self.time, 10.0, 5.0);
            draw_char_scaled(
                buffer,
                x + i as i32 * char_w + dx,
                y + dy,
                ch,
                100,
                255,
                100,
                scale,
            );
        }
        y += line_h;

        // 4. Circle
        draw_label(buffer, 10, y, "Circle:");
        let text = "ORBIT";
        let x = 100;
        for (i, ch) in text.chars().enumerate() {
            let (dx, dy) = offset::circle(i, self.time, 6.0, 2.0);
            draw_char_scaled(
                buffer,
                x + i as i32 * char_w + dx,
                y + dy,
                ch,
                100,
                200,
                255,
                scale,
            );
        }
        y += line_h + 4;

        // === COLOR EFFECTS ===

        // 5. Rainbow
        draw_label(buffer, 10, y, "Rainbow:");
        let text = "RAINBOW";
        let x = 100;
        for (i, ch) in text.chars().enumerate() {
            let (r, g, b) = color::rainbow(i, self.time, 2.0);
            draw_char_scaled(buffer, x + i as i32 * char_w, y, ch, r, g, b, scale);
        }
        y += line_h;

        // 6. Gradient
        draw_label(buffer, 10, y, "Gradient:");
        let text = "GRADIENT";
        let x = 100;
        let len = text.chars().count();
        for (i, ch) in text.chars().enumerate() {
            let (r, g, b) = color::gradient(i, len, (255, 50, 50), (50, 50, 255));
            draw_char_scaled(buffer, x + i as i32 * char_w, y, ch, r, g, b, scale);
        }
        y += line_h;

        // 7. Pulse brightness
        draw_label(buffer, 10, y, "Pulse:");
        let text = "BREATHING";
        let x = 100;
        let base_color = (255, 200, 100);
        let (r, g, b) = color::pulse_brightness(base_color, self.time, 2.0, 0.3);
        for (i, ch) in text.chars().enumerate() {
            draw_char_scaled(buffer, x + i as i32 * char_w, y, ch, r, g, b, scale);
        }
        y += line_h + 4;

        // === VISIBILITY EFFECTS ===

        // 8. Blink
        draw_label(buffer, 10, y, "Blink:");
        let text = "BLINKING";
        let x = 100;
        let vis = visibility::blink(self.time, 2.0);
        if vis > 0.5 {
            for (i, ch) in text.chars().enumerate() {
                draw_char_scaled(buffer, x + i as i32 * char_w, y, ch, 255, 255, 255, scale);
            }
        }
        y += line_h;

        // 9. Strobe (fast blink)
        draw_label(buffer, 10, y, "Strobe:");
        let text = "FLASH";
        let x = 100;
        let vis = visibility::strobe(self.time, 3.0);
        if vis > 0.5 {
            for (i, ch) in text.chars().enumerate() {
                draw_char_scaled(buffer, x + i as i32 * char_w, y, ch, 255, 50, 50, scale);
            }
        }
        y += line_h;

        // 10. Sequential blink
        draw_label(buffer, 10, y, "Seq Blink:");
        let text = "SEQUENCE";
        let x = 100;
        for (i, ch) in text.chars().enumerate() {
            let vis = visibility::blink_seq(i, self.time, 4.0, 0.15);
            if vis > 0.5 {
                draw_char_scaled(buffer, x + i as i32 * char_w, y, ch, 255, 200, 50, scale);
            }
        }
        y += line_h;

        // 11. Random blink (glitch)
        draw_label(buffer, 10, y, "Glitch:");
        let text = "GLITCHING";
        let x = 100;
        for (i, ch) in text.chars().enumerate() {
            let vis = visibility::blink_rand(i, self.time, 8.0);
            if vis > 0.5 {
                draw_char_scaled(buffer, x + i as i32 * char_w, y, ch, 50, 255, 150, scale);
            }
        }
        y += line_h + 8;

        // === LAYER EFFECTS ===

        // 12. Shadow
        draw_label(buffer, 10, y, "Shadow:");
        transform::draw_text_shadowed(
            buffer,
            100,
            y,
            "SHADOW",
            (255, 255, 255),
            (40, 40, 40),
            scale,
            2,
            2,
        );
        y += line_h;

        // 13. Outline
        draw_label(buffer, 10, y, "Outline:");
        transform::draw_text_outlined(
            buffer,
            100,
            y,
            "OUTLINED",
            (255, 255, 100),
            (100, 50, 0),
            scale,
        );
        y += line_h;

        // 14. Reflection
        draw_label(buffer, 10, y, "Reflect:");
        transform::draw_text_reflected(buffer, 100, y, "MIRROR", (100, 200, 255), scale, 2, 0.4);
    }

    fn name(&self) -> &str {
        "Text FX Demo"
    }
}

fn draw_label(buffer: &mut PixelBuffer, x: i32, y: i32, text: &str) {
    draw_text(buffer, x, y + 4, text, 100, 100, 100);
}
