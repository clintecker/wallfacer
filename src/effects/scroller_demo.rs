use super::Effect;
use crate::display::{
    draw_text, ColorEffect, LayerEffect, OffsetEffect, PixelBuffer, ScrollDirection,
    StyledScroller, Typewriter, VisibilityEffect, GLYPH_HEIGHT,
};
use crate::regions::Scene;

/// Demo effect showcasing styled scrollers with combined effects
pub struct ScrollerDemo {
    // Combined effect scrollers
    rainbow_wave: StyledScroller,
    shadow_bounce: StyledScroller,
    outlined_wobble: StyledScroller,
    reflected_pulse: StyledScroller,
    glitch_strobe: StyledScroller,
    gradient_circle: StyledScroller,

    // Typewriter for comparison
    typewriter: Typewriter,
    typewriter_pause: f32,
}

impl ScrollerDemo {
    pub fn new() -> Self {
        Self {
            // Rainbow colors + sine wave motion
            rainbow_wave: StyledScroller::new("RAINBOW WAVE --- Classic demoscene combo ---")
                .speed(100.0)
                .direction(ScrollDirection::Leftward)
                .scale(2)
                .color(255, 255, 255)
                .offset(OffsetEffect::Wave {
                    amplitude: 12.0,
                    frequency: 4.0,
                })
                .color_fx(ColorEffect::Rainbow { speed: 2.0 }),

            // Drop shadow + bouncing letters
            shadow_bounce: StyledScroller::new("BOUNCY SHADOW")
                .speed(80.0)
                .direction(ScrollDirection::Rightward)
                .scale(2)
                .color(255, 200, 100)
                .offset(OffsetEffect::Bounce {
                    height: 15.0,
                    speed: 6.0,
                })
                .layer(LayerEffect::Shadow {
                    offset_x: 3,
                    offset_y: 3,
                    color: (40, 30, 20),
                }),

            // Outlined text + wobble
            outlined_wobble: StyledScroller::new("GLITCHY OUTLINE")
                .speed(60.0)
                .direction(ScrollDirection::Leftward)
                .scale(2)
                .color(255, 255, 100)
                .offset(OffsetEffect::Wobble { amount: 2.0 })
                .layer(LayerEffect::Outline {
                    color: (100, 80, 0),
                }),

            // Reflection + pulsing brightness
            reflected_pulse: StyledScroller::new("MIRROR PULSE")
                .speed(50.0)
                .direction(ScrollDirection::Rightward)
                .scale(3)
                .color(100, 200, 255)
                .color_fx(ColorEffect::Pulse {
                    speed: 2.0,
                    min_brightness: 0.4,
                })
                .layer(LayerEffect::Reflection { gap: 4, fade: 0.3 }),

            // Glitch visibility + strobe + gradient
            glitch_strobe: StyledScroller::new("CYBER GLITCH --- SYSTEM ERROR ---")
                .speed(120.0)
                .direction(ScrollDirection::Leftward)
                .scale(2)
                .color(50, 255, 150)
                .color_fx(ColorEffect::Gradient {
                    start: (255, 50, 50),
                    end: (50, 255, 50),
                })
                .visibility(VisibilityEffect::BlinkRandom { rate: 6.0 }),

            // Circle motion + gradient
            gradient_circle: StyledScroller::new("ORBITAL GRADIENT")
                .speed(70.0)
                .direction(ScrollDirection::Leftward)
                .scale(2)
                .offset(OffsetEffect::Circle {
                    radius: 8.0,
                    speed: 3.0,
                })
                .color_fx(ColorEffect::Gradient {
                    start: (255, 100, 255),
                    end: (100, 255, 255),
                }),

            typewriter: Typewriter::new("Typewriter with styled scrollers demo...")
                .speed(15.0)
                .scale(1)
                .color(150, 150, 150),

            typewriter_pause: 0.0,
        }
    }
}

impl Default for ScrollerDemo {
    fn default() -> Self {
        Self::new()
    }
}

impl Effect for ScrollerDemo {
    fn update(&mut self, dt: f32, width: u32, _height: u32, _scene: &Scene) {
        // Update screen width for all scrollers
        self.rainbow_wave.set_screen_width(width);
        self.shadow_bounce.set_screen_width(width);
        self.outlined_wobble.set_screen_width(width);
        self.reflected_pulse.set_screen_width(width);
        self.glitch_strobe.set_screen_width(width);
        self.gradient_circle.set_screen_width(width);

        self.rainbow_wave.update(dt);
        self.shadow_bounce.update(dt);
        self.outlined_wobble.update(dt);
        self.reflected_pulse.update(dt);
        self.glitch_strobe.update(dt);
        self.gradient_circle.update(dt);

        if self.typewriter.is_complete() {
            self.typewriter_pause += dt;
            if self.typewriter_pause > 3.0 {
                self.typewriter.reset();
                self.typewriter_pause = 0.0;
            }
        } else {
            self.typewriter.update(dt);
        }
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        buffer.clear(15, 15, 25);

        let scale2_height = (GLYPH_HEIGHT * 2) as i32;
        let scale3_height = (GLYPH_HEIGHT * 3) as i32;
        let label_height = GLYPH_HEIGHT as i32;
        let padding = 6_i32;
        let gap = 4; // gap between label and scroller background

        let mut y = 8;

        // Rainbow wave (wave amplitude 12)
        draw_label(buffer, 10, y, "Rainbow+Wave:");
        y += label_height + gap + padding + 12; // +12 for wave amplitude
        self.rainbow_wave
            .render_with_background(buffer, y, (30, 20, 40), padding as u32);
        y += scale2_height + padding + 12 + gap; // text + padding below + amplitude + gap

        // Shadow bounce (bounce height 15)
        draw_label(buffer, 10, y, "Shadow+Bounce:");
        y += label_height + gap + padding + 15; // +15 for bounce height
        self.shadow_bounce
            .render_with_background(buffer, y, (40, 35, 25), padding as u32);
        y += scale2_height + padding + 3 + gap; // shadow adds 3 below

        // Outlined wobble (wobble amount 2)
        draw_label(buffer, 10, y, "Outline+Wobble:");
        y += label_height + gap + padding + 2;
        self.outlined_wobble
            .render_with_background(buffer, y, (35, 35, 20), padding as u32);
        y += scale2_height + padding + 2 + gap;

        // Reflected pulse (scale 3 + reflection)
        draw_label(buffer, 10, y, "Reflect+Pulse:");
        y += label_height + gap + padding;
        self.reflected_pulse
            .render_with_background(buffer, y, (20, 30, 40), padding as u32);
        y += scale3_height + 4 + scale3_height + padding + gap; // text + gap + reflection + padding

        // Glitch strobe
        draw_label(buffer, 10, y, "Glitch+Gradient:");
        y += label_height + gap + padding;
        self.glitch_strobe
            .render_with_background(buffer, y, (20, 35, 30), padding as u32);
        y += scale2_height + padding + gap;

        // Gradient circle (circle radius 8)
        draw_label(buffer, 10, y, "Circle+Gradient:");
        y += label_height + gap + padding + 8; // +8 for circle radius
        self.gradient_circle
            .render_with_background(buffer, y, (35, 25, 35), padding as u32);
    }

    fn name(&self) -> &str {
        "Scroller Demo"
    }
}

fn draw_label(buffer: &mut PixelBuffer, x: i32, y: i32, text: &str) {
    draw_text(buffer, x, y, text, 100, 100, 100);
}
