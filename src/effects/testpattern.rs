use super::Effect;
use crate::display::{PixelBuffer, SineScroller, GLYPH_HEIGHT};
use crate::regions::Scene;

/// Classic SMPTE color bars test pattern
pub struct TestPattern {
    time: f32,
    scroller: SineScroller,
    scanline_offset: f32,
    glitch_timer: f32,
}

impl TestPattern {
    pub fn new() -> Self {
        Self {
            time: 0.0,
            scroller: SineScroller::new("2389 RESEARCH     ")
                .speed(80.0)
                .scale(5)
                .amplitude(20.0)
                .frequency(3.0)
                .color(255, 255, 255),
            scanline_offset: 0.0,
            glitch_timer: 0.0,
        }
    }
}

impl Default for TestPattern {
    fn default() -> Self {
        Self::new()
    }
}

// SMPTE color bar colors (75% intensity)
const WHITE: (u8, u8, u8) = (191, 191, 191);
const YELLOW: (u8, u8, u8) = (191, 191, 0);
const CYAN: (u8, u8, u8) = (0, 191, 191);
const GREEN: (u8, u8, u8) = (0, 191, 0);
const MAGENTA: (u8, u8, u8) = (191, 0, 191);
const RED: (u8, u8, u8) = (191, 0, 0);
const BLUE: (u8, u8, u8) = (0, 0, 191);
const BLACK: (u8, u8, u8) = (0, 0, 0);

// Pluge bar colors for black level calibration
const SUPERBLACK: (u8, u8, u8) = (0, 0, 0); // -4% (clipped to 0)
const BLACK_PLUS: (u8, u8, u8) = (10, 10, 10); // +4%

// -I and +Q colors for NTSC calibration
const NEG_I: (u8, u8, u8) = (0, 68, 130);
const POS_Q: (u8, u8, u8) = (67, 0, 130);

impl Effect for TestPattern {
    fn update(&mut self, dt: f32, width: u32, height: u32, _scene: &Scene) {
        self.time += dt;

        // Update scroller
        self.scroller.set_screen_width(width);
        self.scroller.update(dt);

        // Animate scanlines with layered sine waves for organic motion
        let wave1 = (self.time * 0.3).sin() * 40.0; // slow, large wave
        let wave2 = (self.time * 0.7).sin() * 20.0; // medium wave
        let wave3 = (self.time * 1.9).sin() * 8.0; // faster ripple
        self.scanline_offset = wave1 + wave2 + wave3 + (height as f32 / 2.0);

        // Occasional glitch - jump forward
        self.glitch_timer += dt;
        if self.glitch_timer > 2.5 + (self.time * 1.7).sin().abs() * 4.0 {
            self.glitch_timer = 0.0;
            self.scanline_offset += 20.0 + (self.time * 3.3).sin().abs() * 30.0;
        }
    }

    fn render(&self, buffer: &mut PixelBuffer) {
        let width = buffer.width() as i32;
        let height = buffer.height() as i32;

        // Main color bars take top 2/3
        let main_height = height * 2 / 3;
        // Middle strip is 1/12 of height
        let mid_height = height / 12;
        // Bottom pluge section is remaining
        let bottom_y = main_height + mid_height;

        let bar_width = width / 7;

        // Top section: 7 color bars
        let main_colors = [WHITE, YELLOW, CYAN, GREEN, MAGENTA, RED, BLUE];
        for (i, color) in main_colors.iter().enumerate() {
            let x = i as i32 * bar_width;
            let w = if i == 6 { width - x } else { bar_width }; // Last bar fills remainder
            buffer.fill_rect(
                x,
                0,
                w as u32,
                main_height as u32,
                color.0,
                color.1,
                color.2,
            );
        }

        // Middle section: reverse mini-bars
        // Pattern: blue, black, magenta, black, cyan, black, white
        let mid_colors = [BLUE, BLACK, MAGENTA, BLACK, CYAN, BLACK, WHITE];
        for (i, color) in mid_colors.iter().enumerate() {
            let x = i as i32 * bar_width;
            let w = if i == 6 { width - x } else { bar_width };
            buffer.fill_rect(
                x,
                main_height,
                w as u32,
                mid_height as u32,
                color.0,
                color.1,
                color.2,
            );
        }

        // Bottom section: pluge bars for black level calibration
        // Layout: -I (1 bar), white (1 bar), +Q (1 bar), black (3.5 bars), pluge (0.5 bar)
        let bottom_height = (height - bottom_y) as u32;

        // -I signal
        buffer.fill_rect(
            0,
            bottom_y,
            bar_width as u32,
            bottom_height,
            NEG_I.0,
            NEG_I.1,
            NEG_I.2,
        );

        // White (100%)
        buffer.fill_rect(
            bar_width,
            bottom_y,
            bar_width as u32,
            bottom_height,
            255,
            255,
            255,
        );

        // +Q signal
        buffer.fill_rect(
            bar_width * 2,
            bottom_y,
            bar_width as u32,
            bottom_height,
            POS_Q.0,
            POS_Q.1,
            POS_Q.2,
        );

        // Black section (fills most of remaining width)
        let pluge_start = width - bar_width;
        let black_width = pluge_start - bar_width * 3;
        buffer.fill_rect(
            bar_width * 3,
            bottom_y,
            black_width as u32,
            bottom_height,
            BLACK.0,
            BLACK.1,
            BLACK.2,
        );

        // Pluge bars: superblack | black | black+4%
        // Each is 1/3 of remaining bar width
        let pluge_width = bar_width / 3;

        buffer.fill_rect(
            pluge_start,
            bottom_y,
            pluge_width as u32,
            bottom_height,
            SUPERBLACK.0,
            SUPERBLACK.1,
            SUPERBLACK.2,
        );

        buffer.fill_rect(
            pluge_start + pluge_width,
            bottom_y,
            pluge_width as u32,
            bottom_height,
            BLACK.0,
            BLACK.1,
            BLACK.2,
        );

        buffer.fill_rect(
            pluge_start + pluge_width * 2,
            bottom_y,
            (width - (pluge_start + pluge_width * 2)) as u32,
            bottom_height,
            BLACK_PLUS.0,
            BLACK_PLUS.1,
            BLACK_PLUS.2,
        );

        // Scrolling sine wave text marquee
        let text_height = (GLYPH_HEIGHT * 5) as i32; // scale 5
        let center_y = height / 2 - text_height / 2;
        let amplitude = 20;

        // Draw background strip for text
        let strip_padding = 8;
        let strip_y = center_y - strip_padding - amplitude;
        let strip_h = text_height + strip_padding * 2 + (amplitude * 2);
        buffer.fill_rect(0, strip_y, width as u32, strip_h as u32, 0, 0, 0);

        // Render scroller
        self.scroller.render(buffer, center_y);

        // CRT scanlines effect - fatter lines, animated
        let buf_width = buffer.width() as usize;
        let buf_height = buffer.height() as usize;
        let pixels = buffer.as_bytes_mut();

        let scanline_thickness = 3; // dark lines are 3 pixels thick
        let scanline_gap = 3; // bright lines are 3 pixels thick
        let scanline_period = scanline_thickness + scanline_gap;
        let offset = self.scanline_offset as usize;

        for y in 0..buf_height {
            // Calculate position in scanline pattern with offset
            let pattern_pos = (y + offset) % scanline_period;

            // Dark scanline bands (subtle transparency) - ABGR format
            if pattern_pos < scanline_thickness {
                let row_start = y * buf_width * 4;
                for x in 0..buf_width {
                    let i = row_start + x * 4;
                    // Skip [0] (alpha), darken [1]=B, [2]=G, [3]=R
                    pixels[i + 1] = (pixels[i + 1] as u32 * 82 / 100) as u8;
                    pixels[i + 2] = (pixels[i + 2] as u32 * 82 / 100) as u8;
                    pixels[i + 3] = (pixels[i + 3] as u32 * 82 / 100) as u8;
                }
            }
        }
    }

    fn name(&self) -> &str {
        "Test Pattern"
    }
}
