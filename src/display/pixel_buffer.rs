use super::{DEFAULT_HEIGHT, DEFAULT_WIDTH};

// ============================================================================
// Blend Mode
// ============================================================================

/// Compositing blend mode for `composite()` / `composite_full()`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    /// Standard source-over alpha blending
    Alpha,
    /// Additive: dst += src * (src_alpha / 255), saturating
    Additive,
    /// Multiply: dst = lerp(dst, dst * src / 255, src_alpha)
    Multiply,
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Alpha blend a single color channel
/// Uses fast approximation: (x + 1 + (x >> 8)) >> 8 instead of x / 255
#[inline]
fn blend_channel(src: u8, dst: u8, alpha: u16) -> u8 {
    let result = src as u16 * alpha + dst as u16 * (255 - alpha);
    ((result + 1 + (result >> 8)) >> 8) as u8
}

/// Write ABGR pixel to slice (RGBA8888 little-endian byte order)
#[inline]
fn write_pixel(dest: &mut [u8], r: u8, g: u8, b: u8) {
    dest[0] = 255; // A
    dest[1] = b; // B
    dest[2] = g; // G
    dest[3] = r; // R
}

/// Write ABGR pixel with custom alpha (for scratch buffers used with `composite`)
#[inline]
fn write_pixel_rgba(dest: &mut [u8], r: u8, g: u8, b: u8, a: u8) {
    dest[0] = a; // A
    dest[1] = b; // B
    dest[2] = g; // G
    dest[3] = r; // R
}

// ============================================================================
// PixelBuffer
// ============================================================================

/// RGBA8888 pixel buffer for software rendering
/// This is our canvas - all demoscene effects render to this
pub struct PixelBuffer {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
    depth: Option<Vec<f32>>,
}

impl PixelBuffer {
    /// Create a new pixel buffer with default resolution (640x480)
    pub fn new() -> Self {
        Self::with_size(DEFAULT_WIDTH, DEFAULT_HEIGHT)
    }

    /// Create a new pixel buffer with custom resolution
    pub fn with_size(width: u32, height: u32) -> Self {
        Self {
            pixels: vec![0; (width * height * 4) as usize],
            width,
            height,
            depth: None,
        }
    }

    /// Create a pixel buffer with an attached depth buffer (initialized to infinity)
    pub fn with_depth(width: u32, height: u32) -> Self {
        let pixel_count = (width * height) as usize;
        Self {
            pixels: vec![0; pixel_count * 4],
            width,
            height,
            depth: Some(vec![f32::INFINITY; pixel_count]),
        }
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Check if coordinates are within bounds
    #[inline]
    fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && x < self.width as i32 && y >= 0 && y < self.height as i32
    }

    /// Calculate byte offset for pixel at (x, y)
    #[inline]
    fn pixel_index(&self, x: u32, y: u32) -> usize {
        ((y * self.width + x) * 4) as usize
    }

    /// Clear to a solid color
    /// Optimized: uses u32 fill for maximum speed
    pub fn clear(&mut self, r: u8, g: u8, b: u8) {
        // Create ABGR u32 pattern
        let pixel = u32::from_ne_bytes([255, b, g, r]);

        // Safety: pixels.len() is always divisible by 4 (width * height * 4).
        // We use write_unaligned to avoid assuming alignment of Vec<u8>.
        let ptr = self.pixels.as_mut_ptr() as *mut u32;
        let len = self.pixels.len() / 4;

        // Fill using u32 writes (4x faster than byte-by-byte)
        for i in 0..len {
            // Safety: i < len ensures we stay within bounds, and we use
            // write_unaligned for portability across platforms with different
            // alignment requirements.
            unsafe {
                ptr.add(i).write_unaligned(pixel);
            }
        }
    }

    /// Clear to a solid color with custom alpha (for scratch buffers)
    pub fn clear_rgba(&mut self, r: u8, g: u8, b: u8, a: u8) {
        let pixel = u32::from_ne_bytes([a, b, g, r]);
        let ptr = self.pixels.as_mut_ptr() as *mut u32;
        let len = self.pixels.len() / 4;
        for i in 0..len {
            // Safety: same as clear() - bounds checked and unaligned write
            unsafe {
                ptr.add(i).write_unaligned(pixel);
            }
        }
    }

    /// Set a single pixel with custom alpha (bounds checked)
    #[inline]
    pub fn set_pixel_rgba(&mut self, x: i32, y: i32, r: u8, g: u8, b: u8, a: u8) {
        if self.in_bounds(x, y) {
            let idx = self.pixel_index(x as u32, y as u32);
            write_pixel_rgba(&mut self.pixels[idx..idx + 4], r, g, b, a);
        }
    }

    /// Read all 4 channels of a pixel (bounds checked)
    /// Returns (r, g, b, a) or None if out of bounds
    #[inline]
    pub fn get_pixel_rgba(&self, x: i32, y: i32) -> Option<(u8, u8, u8, u8)> {
        if self.in_bounds(x, y) {
            let idx = self.pixel_index(x as u32, y as u32);
            Some((
                self.pixels[idx + 3], // R
                self.pixels[idx + 2], // G
                self.pixels[idx + 1], // B
                self.pixels[idx],     // A
            ))
        } else {
            None
        }
    }

    // ========================================================================
    // Depth Buffer
    // ========================================================================

    /// Returns true if this buffer has a depth buffer attached
    #[inline]
    pub fn has_depth(&self) -> bool {
        self.depth.is_some()
    }

    /// Reset depth buffer to infinity. No-op if no depth buffer.
    pub fn clear_depth(&mut self) {
        if let Some(ref mut d) = self.depth {
            d.fill(f32::INFINITY);
        }
    }

    /// Clear color (A=255) and depth in one call
    pub fn clear_all(&mut self, r: u8, g: u8, b: u8) {
        self.clear(r, g, b);
        self.clear_depth();
    }

    /// Read depth value at (x, y). Returns None if out of bounds or no depth buffer.
    #[inline]
    pub fn depth_at(&self, x: i32, y: i32) -> Option<f32> {
        if !self.in_bounds(x, y) {
            return None;
        }
        self.depth
            .as_ref()
            .map(|d| d[(y as u32 * self.width + x as u32) as usize])
    }

    /// Depth-tested pixel write: writes only if z < current depth, then updates depth.
    /// No-op if no depth buffer is attached (always writes color).
    #[inline]
    pub fn set_pixel_z(&mut self, x: i32, y: i32, z: f32, r: u8, g: u8, b: u8) {
        if !self.in_bounds(x, y) {
            return;
        }
        let pi = (y as u32 * self.width + x as u32) as usize;
        if let Some(ref mut d) = self.depth {
            if z >= d[pi] {
                return;
            }
            d[pi] = z;
        }
        let idx = pi * 4;
        write_pixel(&mut self.pixels[idx..idx + 4], r, g, b);
    }

    /// Set a single pixel (bounds checked)
    #[inline]
    pub fn set_pixel(&mut self, x: i32, y: i32, r: u8, g: u8, b: u8) {
        if self.in_bounds(x, y) {
            let idx = self.pixel_index(x as u32, y as u32);
            write_pixel(&mut self.pixels[idx..idx + 4], r, g, b);
        }
    }

    /// Set pixel with alpha blending
    #[inline]
    pub fn blend_pixel(&mut self, x: i32, y: i32, r: u8, g: u8, b: u8, a: u8) {
        if self.in_bounds(x, y) {
            let idx = self.pixel_index(x as u32, y as u32);
            let alpha = a as u16;
            self.pixels[idx] = 255; // A - always opaque
            self.pixels[idx + 1] = blend_channel(b, self.pixels[idx + 1], alpha);
            self.pixels[idx + 2] = blend_channel(g, self.pixels[idx + 2], alpha);
            self.pixels[idx + 3] = blend_channel(r, self.pixels[idx + 3], alpha);
        }
    }

    /// Fast unchecked pixel set - use when you've already bounds-checked
    #[inline]
    pub unsafe fn set_pixel_unchecked(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8) {
        let idx = self.pixel_index(x, y);
        *self.pixels.get_unchecked_mut(idx) = 255;
        *self.pixels.get_unchecked_mut(idx + 1) = b;
        *self.pixels.get_unchecked_mut(idx + 2) = g;
        *self.pixels.get_unchecked_mut(idx + 3) = r;
    }

    /// Read a pixel from the buffer (bounds checked)
    /// Returns None if coordinates are out of bounds
    #[inline]
    pub fn get_pixel(&self, x: i32, y: i32) -> Option<(u8, u8, u8)> {
        if self.in_bounds(x, y) {
            let idx = self.pixel_index(x as u32, y as u32);
            Some((
                self.pixels[idx + 3], // R
                self.pixels[idx + 2], // G
                self.pixels[idx + 1], // B
            ))
        } else {
            None
        }
    }

    /// Fast unchecked pixel read - use when bounds already verified
    #[inline]
    pub unsafe fn get_pixel_unchecked(&self, x: u32, y: u32) -> (u8, u8, u8) {
        let idx = self.pixel_index(x, y);
        (
            *self.pixels.get_unchecked(idx + 3), // R
            *self.pixels.get_unchecked(idx + 2), // G
            *self.pixels.get_unchecked(idx + 1), // B
        )
    }

    /// Additive blend a pixel (colors saturate at 255)
    /// Used for glow effects, glenz vectors, and shadebobs
    #[inline]
    pub fn blend_pixel_additive(&mut self, x: i32, y: i32, r: u8, g: u8, b: u8) {
        if self.in_bounds(x, y) {
            let idx = self.pixel_index(x as u32, y as u32);
            self.pixels[idx + 1] = self.pixels[idx + 1].saturating_add(b);
            self.pixels[idx + 2] = self.pixels[idx + 2].saturating_add(g);
            self.pixels[idx + 3] = self.pixels[idx + 3].saturating_add(r);
        }
    }

    /// Fast unchecked additive blend
    #[inline]
    pub unsafe fn blend_pixel_additive_unchecked(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8) {
        let idx = self.pixel_index(x, y);
        let pixels = &mut self.pixels;
        *pixels.get_unchecked_mut(idx + 1) = pixels.get_unchecked(idx + 1).saturating_add(b);
        *pixels.get_unchecked_mut(idx + 2) = pixels.get_unchecked(idx + 2).saturating_add(g);
        *pixels.get_unchecked_mut(idx + 3) = pixels.get_unchecked(idx + 3).saturating_add(r);
    }

    /// Draw a horizontal line (classic demoscene primitive)
    /// Optimized: computes starting index once, then increments by 4
    pub fn hline(&mut self, x1: i32, x2: i32, y: i32, r: u8, g: u8, b: u8) {
        if y < 0 || y >= self.height as i32 {
            return;
        }
        let (x1, x2) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
        let start = x1.max(0);
        let end = x2.min(self.width as i32 - 1);
        if start > end {
            return;
        }

        // Compute starting index once, then increment by 4 per pixel
        let mut idx = self.pixel_index(start as u32, y as u32);
        let count = (end - start + 1) as usize;
        for _ in 0..count {
            write_pixel(&mut self.pixels[idx..idx + 4], r, g, b);
            idx += 4;
        }
    }

    /// Draw a horizontal line with alpha blending
    pub fn hline_blend(&mut self, x1: i32, x2: i32, y: i32, r: u8, g: u8, b: u8, a: u8) {
        if y < 0 || y >= self.height as i32 {
            return;
        }
        let (x1, x2) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
        let start = x1.max(0);
        let end = x2.min(self.width as i32 - 1);
        if start > end {
            return;
        }

        let alpha = a as u16;
        let mut idx = self.pixel_index(start as u32, y as u32);
        let count = (end - start + 1) as usize;
        for _ in 0..count {
            self.pixels[idx] = 255;
            self.pixels[idx + 1] = blend_channel(b, self.pixels[idx + 1], alpha);
            self.pixels[idx + 2] = blend_channel(g, self.pixels[idx + 2], alpha);
            self.pixels[idx + 3] = blend_channel(r, self.pixels[idx + 3], alpha);
            idx += 4;
        }
    }

    /// Draw a horizontal line with additive blending (for glow/raster effects)
    pub fn hline_additive(&mut self, x1: i32, x2: i32, y: i32, r: u8, g: u8, b: u8) {
        if y < 0 || y >= self.height as i32 {
            return;
        }
        let (x1, x2) = if x1 <= x2 { (x1, x2) } else { (x2, x1) };
        let start = x1.max(0);
        let end = x2.min(self.width as i32 - 1);
        if start > end {
            return;
        }

        let mut idx = self.pixel_index(start as u32, y as u32);
        let count = (end - start + 1) as usize;
        for _ in 0..count {
            self.pixels[idx + 1] = self.pixels[idx + 1].saturating_add(b);
            self.pixels[idx + 2] = self.pixels[idx + 2].saturating_add(g);
            self.pixels[idx + 3] = self.pixels[idx + 3].saturating_add(r);
            idx += 4;
        }
    }

    /// Draw a vertical line
    pub fn vline(&mut self, x: i32, y1: i32, y2: i32, r: u8, g: u8, b: u8) {
        if x < 0 || x >= self.width as i32 {
            return;
        }
        let (y1, y2) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };
        let start = y1.max(0);
        let end = y2.min(self.height as i32 - 1);
        if start > end {
            return;
        }

        // Stride is width * 4 bytes per row
        let stride = (self.width * 4) as usize;
        let mut idx = self.pixel_index(x as u32, start as u32);
        let count = (end - start + 1) as usize;
        for _ in 0..count {
            write_pixel(&mut self.pixels[idx..idx + 4], r, g, b);
            idx += stride;
        }
    }

    /// Draw a vertical line with alpha blending
    pub fn vline_blend(&mut self, x: i32, y1: i32, y2: i32, r: u8, g: u8, b: u8, a: u8) {
        if x < 0 || x >= self.width as i32 {
            return;
        }
        let (y1, y2) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };
        let start = y1.max(0);
        let end = y2.min(self.height as i32 - 1);
        if start > end {
            return;
        }

        let alpha = a as u16;
        let stride = (self.width * 4) as usize;
        let mut idx = self.pixel_index(x as u32, start as u32);
        let count = (end - start + 1) as usize;
        for _ in 0..count {
            self.pixels[idx] = 255;
            self.pixels[idx + 1] = blend_channel(b, self.pixels[idx + 1], alpha);
            self.pixels[idx + 2] = blend_channel(g, self.pixels[idx + 2], alpha);
            self.pixels[idx + 3] = blend_channel(r, self.pixels[idx + 3], alpha);
            idx += stride;
        }
    }

    /// Draw a vertical line with additive blending (for glow effects)
    pub fn vline_additive(&mut self, x: i32, y1: i32, y2: i32, r: u8, g: u8, b: u8) {
        if x < 0 || x >= self.width as i32 {
            return;
        }
        let (y1, y2) = if y1 <= y2 { (y1, y2) } else { (y2, y1) };
        let start = y1.max(0);
        let end = y2.min(self.height as i32 - 1);
        if start > end {
            return;
        }

        let stride = (self.width * 4) as usize;
        let mut idx = self.pixel_index(x as u32, start as u32);
        let count = (end - start + 1) as usize;
        for _ in 0..count {
            self.pixels[idx + 1] = self.pixels[idx + 1].saturating_add(b);
            self.pixels[idx + 2] = self.pixels[idx + 2].saturating_add(g);
            self.pixels[idx + 3] = self.pixels[idx + 3].saturating_add(r);
            idx += stride;
        }
    }

    /// Draw a line using Bresenham's algorithm with Cohen-Sutherland clipping
    ///
    /// Clips to screen bounds first, then draws without per-pixel bounds checks.
    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, r: u8, g: u8, b: u8) {
        // Cohen-Sutherland clipping
        let (clipped, cx0, cy0, cx1, cy1) = self.clip_line(x0, y0, x1, y1);
        if !clipped {
            return;
        }

        // Now draw with unchecked access since we've clipped to bounds
        let dx = (cx1 - cx0).abs();
        let dy = -((cy1 - cy0).abs());
        let sx = if cx0 < cx1 { 1i32 } else { -1i32 };
        let sy = if cy0 < cy1 { 1i32 } else { -1i32 };
        let mut err = dx + dy;
        let mut x = cx0;
        let mut y = cy0;

        loop {
            // Safety: coordinates are clipped to valid range
            unsafe {
                self.set_pixel_unchecked(x as u32, y as u32, r, g, b);
            }
            if x == cx1 && y == cy1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// Cohen-Sutherland line clipping algorithm
    /// Returns (visible, x0, y0, x1, y1) with clipped coordinates
    fn clip_line(
        &self,
        mut x0: i32,
        mut y0: i32,
        mut x1: i32,
        mut y1: i32,
    ) -> (bool, i32, i32, i32, i32) {
        const INSIDE: u8 = 0;
        const LEFT: u8 = 1;
        const RIGHT: u8 = 2;
        const BOTTOM: u8 = 4;
        const TOP: u8 = 8;
        // Max iterations to prevent infinite loops with degenerate input (NaN, etc.)
        // Algorithm should converge in at most 4 iterations for valid input
        const MAX_ITERATIONS: u32 = 16;

        let w = self.width as i32;
        let h = self.height as i32;

        let outcode = |x: i32, y: i32| -> u8 {
            let mut code = INSIDE;
            if x < 0 {
                code |= LEFT;
            } else if x >= w {
                code |= RIGHT;
            }
            if y < 0 {
                code |= TOP;
            } else if y >= h {
                code |= BOTTOM;
            }
            code
        };

        let mut code0 = outcode(x0, y0);
        let mut code1 = outcode(x1, y1);

        for _ in 0..MAX_ITERATIONS {
            if (code0 | code1) == 0 {
                // Both inside
                return (true, x0, y0, x1, y1);
            }
            if (code0 & code1) != 0 {
                // Both outside same region
                return (false, 0, 0, 0, 0);
            }

            // Pick endpoint outside
            let code_out = if code0 != 0 { code0 } else { code1 };
            let (x, y);

            // Guard against division by zero
            let dy = y1 - y0;
            let dx = x1 - x0;

            if (code_out & BOTTOM) != 0 {
                if dy == 0 {
                    return (false, 0, 0, 0, 0);
                }
                x = x0 + dx * (h - 1 - y0) / dy;
                y = h - 1;
            } else if (code_out & TOP) != 0 {
                if dy == 0 {
                    return (false, 0, 0, 0, 0);
                }
                x = x0 + dx * (0 - y0) / dy;
                y = 0;
            } else if (code_out & RIGHT) != 0 {
                if dx == 0 {
                    return (false, 0, 0, 0, 0);
                }
                y = y0 + dy * (w - 1 - x0) / dx;
                x = w - 1;
            } else {
                // LEFT
                if dx == 0 {
                    return (false, 0, 0, 0, 0);
                }
                y = y0 + dy * (0 - x0) / dx;
                x = 0;
            }

            if code_out == code0 {
                x0 = x;
                y0 = y;
                code0 = outcode(x0, y0);
            } else {
                x1 = x;
                y1 = y;
                code1 = outcode(x1, y1);
            }
        }

        // Max iterations exceeded (degenerate input) - reject line
        (false, 0, 0, 0, 0)
    }

    /// Draw a line with variable thickness
    ///
    /// Note: For steep angles (> ~45 degrees) with thickness > 5, the parallel-line
    /// approach may produce slight visual gaps due to integer rounding. This is
    /// acceptable for most demoscene effects; use fill_polygon for exact results.
    pub fn line_thick(
        &mut self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        thickness: i32,
        r: u8,
        g: u8,
        b: u8,
    ) {
        if thickness <= 1 {
            self.line(x0, y0, x1, y1, r, g, b);
            return;
        }

        // Calculate perpendicular direction
        let dx = (x1 - x0) as f32;
        let dy = (y1 - y0) as f32;
        let len = (dx * dx + dy * dy).sqrt();

        if len < 0.001 {
            // Degenerate line (single point) - draw filled circle
            self.fill_circle(x0, y0, thickness / 2, r, g, b);
            return;
        }

        // Perpendicular unit vector
        let px = -dy / len;
        let py = dx / len;

        // Draw parallel lines for each offset
        let half = (thickness - 1) as f32 / 2.0;
        for i in 0..thickness {
            let offset = (i as f32) - half;
            let ox = (px * offset) as i32;
            let oy = (py * offset) as i32;
            self.line(x0 + ox, y0 + oy, x1 + ox, y1 + oy, r, g, b);
        }
    }

    /// Draw a thick line with rounded ends (better for glow effects)
    pub fn line_thick_rounded(
        &mut self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        thickness: i32,
        r: u8,
        g: u8,
        b: u8,
    ) {
        self.line_thick(x0, y0, x1, y1, thickness, r, g, b);
        // Add rounded caps
        let radius = thickness / 2;
        self.fill_circle(x0, y0, radius, r, g, b);
        self.fill_circle(x1, y1, radius, r, g, b);
    }

    /// Fill a rectangle
    pub fn fill_rect(&mut self, x: i32, y: i32, w: u32, h: u32, r: u8, g: u8, b: u8) {
        for row in 0..h as i32 {
            self.hline(x, x + w as i32 - 1, y + row, r, g, b);
        }
    }

    /// Draw a filled circle using horizontal spans (much faster than pixel-by-pixel)
    pub fn fill_circle(&mut self, cx: i32, cy: i32, radius: i32, r: u8, g: u8, b: u8) {
        if radius <= 0 {
            if radius == 0 {
                self.set_pixel(cx, cy, r, g, b);
            }
            return;
        }

        // Midpoint circle algorithm with span filling
        let mut x = radius;
        let mut y = 0;
        let mut err = 1 - radius;

        while x >= y {
            // Fill horizontal spans for 4 quadrants, avoiding duplicates
            self.hline(cx - x, cx + x, cy + y, r, g, b);
            if y != 0 {
                self.hline(cx - x, cx + x, cy - y, r, g, b);
            }
            if x != y {
                self.hline(cx - y, cx + y, cy + x, r, g, b);
                if y != 0 {
                    self.hline(cx - y, cx + y, cy - x, r, g, b);
                }
            }

            y += 1;
            if err < 0 {
                err += 2 * y + 1;
            } else {
                x -= 1;
                err += 2 * (y - x) + 1;
            }
        }
    }

    /// Fill a circle with alpha blending (for soft particles, glow effects)
    pub fn fill_circle_blend(&mut self, cx: i32, cy: i32, radius: i32, r: u8, g: u8, b: u8, a: u8) {
        if radius <= 0 {
            if radius == 0 {
                self.blend_pixel(cx, cy, r, g, b, a);
            }
            return;
        }

        let height = self.height as i32;

        let mut xi = radius;
        let mut y = 0;
        let mut err = 1 - radius;

        while xi >= y {
            // Inline helper to fill a horizontal span with blending
            macro_rules! fill_span {
                ($x1:expr, $x2:expr, $line_y:expr) => {{
                    let line_y = $line_y;
                    if line_y >= 0 && line_y < height {
                        let (x1, x2) = if $x1 <= $x2 { ($x1, $x2) } else { ($x2, $x1) };
                        // Use hline_blend for efficient span rendering
                        self.hline_blend(x1, x2, line_y, r, g, b, a);
                    }
                }};
            }

            fill_span!(cx - xi, cx + xi, cy + y);
            if y != 0 {
                fill_span!(cx - xi, cx + xi, cy - y);
            }
            if xi != y {
                fill_span!(cx - y, cx + y, cy + xi);
                if y != 0 {
                    fill_span!(cx - y, cx + y, cy - xi);
                }
            }

            y += 1;
            if err < 0 {
                err += 2 * y + 1;
            } else {
                xi -= 1;
                err += 2 * (y - xi) + 1;
            }
        }
    }

    /// Draw a circle outline (1px thick)
    pub fn draw_circle(&mut self, cx: i32, cy: i32, radius: i32, r: u8, g: u8, b: u8) {
        // Midpoint circle algorithm
        let mut x = radius;
        let mut y = 0;
        let mut err = 0;

        while x >= y {
            self.set_pixel(cx + x, cy + y, r, g, b);
            self.set_pixel(cx + y, cy + x, r, g, b);
            self.set_pixel(cx - y, cy + x, r, g, b);
            self.set_pixel(cx - x, cy + y, r, g, b);
            self.set_pixel(cx - x, cy - y, r, g, b);
            self.set_pixel(cx - y, cy - x, r, g, b);
            self.set_pixel(cx + y, cy - x, r, g, b);
            self.set_pixel(cx + x, cy - y, r, g, b);

            y += 1;
            err += 1 + 2 * y;
            if 2 * (err - x) + 1 > 0 {
                x -= 1;
                err += 1 - 2 * x;
            }
        }
    }

    /// Fill a polygon using scanline algorithm
    /// Optimized: preallocates intersection buffer outside loop
    pub fn fill_polygon(&mut self, vertices: &[(f32, f32)], r: u8, g: u8, b: u8) {
        if vertices.len() < 3 {
            return;
        }

        // Find bounding box
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        for (_, y) in vertices {
            min_y = min_y.min(*y);
            max_y = max_y.max(*y);
        }

        let min_y = (min_y as i32).max(0);
        let max_y = (max_y as i32).min(self.height as i32 - 1);

        // Preallocate intersection buffer (reused per scanline)
        let mut intersections = Vec::with_capacity(vertices.len());
        let n = vertices.len();

        // Scanline fill
        for y in min_y..=max_y {
            intersections.clear(); // Reuse allocation
            let yf = y as f32 + 0.5;

            // Find all edge intersections with this scanline
            for i in 0..n {
                let (x1, y1) = vertices[i];
                let (x2, y2) = vertices[(i + 1) % n];

                // Check if edge crosses this scanline
                if (y1 <= yf && y2 > yf) || (y2 <= yf && y1 > yf) {
                    // Calculate x intersection
                    let x = x1 + (yf - y1) / (y2 - y1) * (x2 - x1);
                    intersections.push(x as i32);
                }
            }

            // Sort intersections and fill between pairs
            intersections.sort_unstable();
            for pair in intersections.chunks_exact(2) {
                self.hline(pair[0], pair[1], y, r, g, b);
            }
        }
    }

    /// Fill a polygon with alpha blending
    /// Optimized: preallocates intersection buffer outside loop
    pub fn fill_polygon_blend(&mut self, vertices: &[(f32, f32)], r: u8, g: u8, b: u8, a: u8) {
        if vertices.len() < 3 {
            return;
        }

        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        for (_, y) in vertices {
            min_y = min_y.min(*y);
            max_y = max_y.max(*y);
        }

        let min_y = (min_y as i32).max(0);
        let max_y = (max_y as i32).min(self.height as i32 - 1);

        // Preallocate intersection buffer (reused per scanline)
        let mut intersections = Vec::with_capacity(vertices.len());
        let n = vertices.len();

        for y in min_y..=max_y {
            intersections.clear(); // Reuse allocation
            let yf = y as f32 + 0.5;

            for i in 0..n {
                let (x1, y1) = vertices[i];
                let (x2, y2) = vertices[(i + 1) % n];

                if (y1 <= yf && y2 > yf) || (y2 <= yf && y1 > yf) {
                    let x = x1 + (yf - y1) / (y2 - y1) * (x2 - x1);
                    intersections.push(x as i32);
                }
            }

            intersections.sort_unstable();
            for pair in intersections.chunks_exact(2) {
                // Use hline_blend for efficient span rendering
                self.hline_blend(pair[0], pair[1], y, r, g, b, a);
            }
        }
    }

    /// Fill a polygon with additive blending (for glenz effect)
    pub fn fill_polygon_additive(&mut self, vertices: &[(f32, f32)], r: u8, g: u8, b: u8) {
        if vertices.len() < 3 {
            return;
        }

        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        for (_, y) in vertices {
            min_y = min_y.min(*y);
            max_y = max_y.max(*y);
        }

        let min_y = (min_y as i32).max(0);
        let max_y = (max_y as i32).min(self.height as i32 - 1);

        let mut intersections = Vec::with_capacity(vertices.len());
        let n = vertices.len();

        for y in min_y..=max_y {
            intersections.clear();
            let yf = y as f32 + 0.5;

            for i in 0..n {
                let (x1, y1) = vertices[i];
                let (x2, y2) = vertices[(i + 1) % n];

                if (y1 <= yf && y2 > yf) || (y2 <= yf && y1 > yf) {
                    let x = x1 + (yf - y1) / (y2 - y1) * (x2 - x1);
                    intersections.push(x as i32);
                }
            }

            intersections.sort_unstable();
            for pair in intersections.chunks_exact(2) {
                // Use hline_additive for efficient span rendering
                self.hline_additive(pair[0], pair[1], y, r, g, b);
            }
        }
    }

    // ========================================================================
    // Buffer Operations
    // ========================================================================

    /// Copy another buffer onto this one at position (x, y)
    pub fn blit(&mut self, src: &PixelBuffer, x: i32, y: i32) {
        let src_w = src.width() as i32;
        let src_h = src.height() as i32;
        let dst_w = self.width as i32;
        let dst_h = self.height as i32;

        for sy in 0..src_h {
            let dy = y + sy;
            if dy < 0 || dy >= dst_h {
                continue;
            }

            for sx in 0..src_w {
                let dx = x + sx;
                if dx < 0 || dx >= dst_w {
                    continue;
                }

                if let Some((r, g, b)) = src.get_pixel(sx, sy) {
                    self.set_pixel(dx, dy, r, g, b);
                }
            }
        }
    }

    /// Blit with alpha blending
    pub fn blit_blend(&mut self, src: &PixelBuffer, x: i32, y: i32, alpha: u8) {
        let src_w = src.width() as i32;
        let src_h = src.height() as i32;
        let dst_w = self.width as i32;
        let dst_h = self.height as i32;

        for sy in 0..src_h {
            let dy = y + sy;
            if dy < 0 || dy >= dst_h {
                continue;
            }

            for sx in 0..src_w {
                let dx = x + sx;
                if dx < 0 || dx >= dst_w {
                    continue;
                }

                if let Some((r, g, b)) = src.get_pixel(sx, sy) {
                    self.blend_pixel(dx, dy, r, g, b, alpha);
                }
            }
        }
    }

    /// Composite a source buffer onto this one using per-pixel source alpha.
    /// Supports Alpha (src-over), Additive, and Multiply blend modes.
    /// Skips fully transparent pixels; fast-copies fully opaque ones in Alpha mode.
    pub fn composite(&mut self, src: &PixelBuffer, dst_x: i32, dst_y: i32, mode: BlendMode) {
        let src_w = src.width() as i32;
        let src_h = src.height() as i32;
        let dst_w = self.width as i32;
        let dst_h = self.height as i32;

        for sy in 0..src_h {
            let dy = dst_y + sy;
            if dy < 0 || dy >= dst_h {
                continue;
            }

            for sx in 0..src_w {
                let dx = dst_x + sx;
                if dx < 0 || dx >= dst_w {
                    continue;
                }

                let si = src.pixel_index(sx as u32, sy as u32);
                let sa = src.pixels[si]; // alpha channel (ABGR[0])
                if sa == 0 {
                    continue;
                }

                let sr = src.pixels[si + 3];
                let sg = src.pixels[si + 2];
                let sb = src.pixels[si + 1];

                let di = self.pixel_index(dx as u32, dy as u32);

                match mode {
                    BlendMode::Alpha => {
                        if sa == 255 {
                            // Fully opaque — direct copy
                            write_pixel(&mut self.pixels[di..di + 4], sr, sg, sb);
                        } else {
                            let alpha = sa as u16;
                            self.pixels[di] = 255;
                            self.pixels[di + 1] = blend_channel(sb, self.pixels[di + 1], alpha);
                            self.pixels[di + 2] = blend_channel(sg, self.pixels[di + 2], alpha);
                            self.pixels[di + 3] = blend_channel(sr, self.pixels[di + 3], alpha);
                        }
                    },
                    BlendMode::Additive => {
                        // dst += src * (src_alpha / 255), saturating
                        let a = sa as u16;
                        let add_r = ((sr as u16 * a + 127) / 255) as u8;
                        let add_g = ((sg as u16 * a + 127) / 255) as u8;
                        let add_b = ((sb as u16 * a + 127) / 255) as u8;
                        self.pixels[di + 1] = self.pixels[di + 1].saturating_add(add_b);
                        self.pixels[di + 2] = self.pixels[di + 2].saturating_add(add_g);
                        self.pixels[di + 3] = self.pixels[di + 3].saturating_add(add_r);
                    },
                    BlendMode::Multiply => {
                        // dst = lerp(dst, dst * src / 255, src_alpha)
                        let a = sa as u16;
                        let dr = self.pixels[di + 3];
                        let dg = self.pixels[di + 2];
                        let db = self.pixels[di + 1];
                        let mr = (dr as u16 * sr as u16 / 255) as u8;
                        let mg = (dg as u16 * sg as u16 / 255) as u8;
                        let mb = (db as u16 * sb as u16 / 255) as u8;
                        self.pixels[di + 3] = blend_channel(mr, dr, a);
                        self.pixels[di + 2] = blend_channel(mg, dg, a);
                        self.pixels[di + 1] = blend_channel(mb, db, a);
                    },
                }
            }
        }
    }

    /// Convenience: composite at (0, 0) — buffers should be the same size
    pub fn composite_full(&mut self, src: &PixelBuffer, mode: BlendMode) {
        self.composite(src, 0, 0, mode);
    }

    /// Fade the entire buffer (multiply all colors by factor)
    /// factor: 0.0 = black, 1.0 = unchanged
    pub fn fade(&mut self, factor: f32) {
        let factor = factor.clamp(0.0, 1.0);
        let factor_u16 = (factor * 256.0) as u16;

        for chunk in self.pixels.chunks_exact_mut(4) {
            // Skip alpha (chunk[0]), fade RGB using bit shift instead of division
            chunk[1] = ((chunk[1] as u16 * factor_u16) >> 8) as u8;
            chunk[2] = ((chunk[2] as u16 * factor_u16) >> 8) as u8;
            chunk[3] = ((chunk[3] as u16 * factor_u16) >> 8) as u8;
        }
    }

    /// Copy contents from another buffer (must be same size)
    pub fn copy_from(&mut self, src: &PixelBuffer) {
        if self.pixels.len() == src.pixels.len() {
            self.pixels.copy_from_slice(&src.pixels);
        }
    }

    /// Scroll buffer contents using double-buffering (avoids allocation)
    /// Positive dx scrolls right, positive dy scrolls down
    pub fn scroll_from(&mut self, src: &PixelBuffer, dx: i32, dy: i32) {
        let w = self.width as i32;
        let h = self.height as i32;

        self.clear(0, 0, 0);

        for y in 0..h {
            let src_y = y - dy;
            if src_y < 0 || src_y >= h {
                continue;
            }

            let x_start = 0.max(-dx);
            let x_end = w.min(w - dx);
            if x_start >= x_end {
                continue;
            }

            let src_x_start = (x_start - dx) as u32;
            let src_row_start = src.pixel_index(src_x_start, src_y as u32);
            let dst_row_start = self.pixel_index(x_start as u32, y as u32);
            let row_bytes = ((x_end - x_start) * 4) as usize;

            self.pixels[dst_row_start..dst_row_start + row_bytes]
                .copy_from_slice(&src.pixels[src_row_start..src_row_start + row_bytes]);
        }
    }

    /// Scroll in place (allocates temporary buffer)
    ///
    /// **Warning**: This function clones the entire buffer (~1.2MB at 640x480).
    /// For feedback effects at 60fps, use `scroll_from` with double-buffering instead.
    #[deprecated(
        since = "0.1.0",
        note = "Allocates full buffer clone per call. Use scroll_from() with double-buffering for hot loops."
    )]
    pub fn scroll(&mut self, dx: i32, dy: i32) {
        let old_pixels = self.pixels.clone();
        let w = self.width as i32;
        let h = self.height as i32;

        self.clear(0, 0, 0);

        for y in 0..h {
            let src_y = y - dy;
            if src_y < 0 || src_y >= h {
                continue;
            }

            let x_start = 0.max(-dx);
            let x_end = w.min(w - dx);
            if x_start >= x_end {
                continue;
            }

            let src_x_start = (x_start - dx) as u32;
            let src_row_start = self.pixel_index(src_x_start, src_y as u32);
            let dst_row_start = self.pixel_index(x_start as u32, y as u32);
            let row_bytes = ((x_end - x_start) * 4) as usize;

            self.pixels[dst_row_start..dst_row_start + row_bytes]
                .copy_from_slice(&old_pixels[src_row_start..src_row_start + row_bytes]);
        }
    }

    /// Scale and blit centered (for zoom feedback effects)
    /// scale > 1.0 zooms in, scale < 1.0 zooms out
    pub fn blit_scaled_centered(&mut self, src: &PixelBuffer, scale: f32) {
        let w = self.width as i32;
        let h = self.height as i32;
        let cx = w as f32 / 2.0;
        let cy = h as f32 / 2.0;

        for y in 0..h {
            for x in 0..w {
                // Map destination to source coordinates
                let sx = ((x as f32 - cx) / scale + cx) as i32;
                let sy = ((y as f32 - cy) / scale + cy) as i32;

                if let Some((r, g, b)) = src.get_pixel(sx, sy) {
                    self.set_pixel(x, y, r, g, b);
                }
            }
        }
    }

    /// Raw bytes for SDL texture upload
    pub fn as_bytes(&self) -> &[u8] {
        &self.pixels
    }

    /// Mutable access to raw pixels for advanced effects
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        &mut self.pixels
    }

    /// Create a new buffer rotated by 90 degrees clockwise.
    /// Output dimensions are swapped (width becomes height, height becomes width).
    pub fn rotated_90(&self) -> Self {
        let new_width = self.height;
        let new_height = self.width;
        let mut rotated = Self::with_size(new_width, new_height);

        for y in 0..self.height {
            for x in 0..self.width {
                // (x, y) -> (height - 1 - y, x) with swapped dimensions
                let new_x = self.height - 1 - y;
                let new_y = x;

                let src_idx = self.pixel_index(x, y);
                let dst_idx = rotated.pixel_index(new_x, new_y);

                rotated.pixels[dst_idx..dst_idx + 4]
                    .copy_from_slice(&self.pixels[src_idx..src_idx + 4]);
            }
        }

        rotated
    }

    /// Create a new buffer rotated by 180 degrees.
    /// Output dimensions remain the same.
    pub fn rotated_180(&self) -> Self {
        let mut rotated = Self::with_size(self.width, self.height);

        for y in 0..self.height {
            for x in 0..self.width {
                // (x, y) -> (width - 1 - x, height - 1 - y)
                let new_x = self.width - 1 - x;
                let new_y = self.height - 1 - y;

                let src_idx = self.pixel_index(x, y);
                let dst_idx = rotated.pixel_index(new_x, new_y);

                rotated.pixels[dst_idx..dst_idx + 4]
                    .copy_from_slice(&self.pixels[src_idx..src_idx + 4]);
            }
        }

        rotated
    }

    /// Create a new buffer rotated by 270 degrees clockwise (90 degrees counter-clockwise).
    /// Output dimensions are swapped (width becomes height, height becomes width).
    pub fn rotated_270(&self) -> Self {
        let new_width = self.height;
        let new_height = self.width;
        let mut rotated = Self::with_size(new_width, new_height);

        for y in 0..self.height {
            for x in 0..self.width {
                // (x, y) -> (y, width - 1 - x) with swapped dimensions
                let new_x = y;
                let new_y = self.width - 1 - x;

                let src_idx = self.pixel_index(x, y);
                let dst_idx = rotated.pixel_index(new_x, new_y);

                rotated.pixels[dst_idx..dst_idx + 4]
                    .copy_from_slice(&self.pixels[src_idx..src_idx + 4]);
            }
        }

        rotated
    }

    // ========================================================================
    // Anti-aliased Drawing (Level 1)
    // Composes: blend_pixel
    // ========================================================================

    /// Anti-aliased line using Xiaolin Wu's algorithm.
    /// Accepts f32 endpoints for subpixel precision.
    pub fn line_aa(
        &mut self,
        mut x0: f32,
        mut y0: f32,
        mut x1: f32,
        mut y1: f32,
        r: u8,
        g: u8,
        b: u8,
    ) {
        let steep = (y1 - y0).abs() > (x1 - x0).abs();
        if steep {
            std::mem::swap(&mut x0, &mut y0);
            std::mem::swap(&mut x1, &mut y1);
        }
        if x0 > x1 {
            std::mem::swap(&mut x0, &mut x1);
            std::mem::swap(&mut y0, &mut y1);
        }

        let dx = x1 - x0;
        let dy = y1 - y0;
        let gradient = if dx.abs() < 0.001 { 1.0 } else { dy / dx };

        // First endpoint
        let xend = x0.round();
        let yend = y0 + gradient * (xend - x0);
        let xgap = 1.0 - (x0 + 0.5).fract();
        let xpxl1 = xend as i32;
        let ypxl1 = yend.floor() as i32;
        let fpart = yend.fract();

        if steep {
            self.blend_pixel(ypxl1, xpxl1, r, g, b, ((1.0 - fpart) * xgap * 255.0) as u8);
            self.blend_pixel(ypxl1 + 1, xpxl1, r, g, b, (fpart * xgap * 255.0) as u8);
        } else {
            self.blend_pixel(xpxl1, ypxl1, r, g, b, ((1.0 - fpart) * xgap * 255.0) as u8);
            self.blend_pixel(xpxl1, ypxl1 + 1, r, g, b, (fpart * xgap * 255.0) as u8);
        }
        let mut intery = yend + gradient;

        // Second endpoint
        let xend = x1.round();
        let yend = y1 + gradient * (xend - x1);
        let xgap = (x1 + 0.5).fract();
        let xpxl2 = xend as i32;
        let ypxl2 = yend.floor() as i32;
        let fpart = yend.fract();

        if steep {
            self.blend_pixel(ypxl2, xpxl2, r, g, b, ((1.0 - fpart) * xgap * 255.0) as u8);
            self.blend_pixel(ypxl2 + 1, xpxl2, r, g, b, (fpart * xgap * 255.0) as u8);
        } else {
            self.blend_pixel(xpxl2, ypxl2, r, g, b, ((1.0 - fpart) * xgap * 255.0) as u8);
            self.blend_pixel(xpxl2, ypxl2 + 1, r, g, b, (fpart * xgap * 255.0) as u8);
        }

        // Main line body
        for x in (xpxl1 + 1)..xpxl2 {
            let fpart = intery.fract();
            let ipart = intery.floor() as i32;
            if steep {
                self.blend_pixel(ipart, x, r, g, b, ((1.0 - fpart) * 255.0) as u8);
                self.blend_pixel(ipart + 1, x, r, g, b, (fpart * 255.0) as u8);
            } else {
                self.blend_pixel(x, ipart, r, g, b, ((1.0 - fpart) * 255.0) as u8);
                self.blend_pixel(x, ipart + 1, r, g, b, (fpart * 255.0) as u8);
            }
            intery += gradient;
        }
    }

    /// Anti-aliased line with additive blending (for glowing trails, shooting stars).
    /// Same algorithm as line_aa but uses saturating add instead of alpha blend.
    pub fn line_aa_additive(
        &mut self,
        mut x0: f32,
        mut y0: f32,
        mut x1: f32,
        mut y1: f32,
        r: u8,
        g: u8,
        b: u8,
    ) {
        let steep = (y1 - y0).abs() > (x1 - x0).abs();
        if steep {
            std::mem::swap(&mut x0, &mut y0);
            std::mem::swap(&mut x1, &mut y1);
        }
        if x0 > x1 {
            std::mem::swap(&mut x0, &mut x1);
            std::mem::swap(&mut y0, &mut y1);
        }

        let dx = x1 - x0;
        let dy = y1 - y0;
        let gradient = if dx.abs() < 0.001 { 1.0 } else { dy / dx };

        let scale = |c: u8, f: f32| -> u8 { (c as f32 * f).min(255.0) as u8 };

        // First endpoint
        let xend = x0.round();
        let yend = y0 + gradient * (xend - x0);
        let xgap = 1.0 - (x0 + 0.5).fract();
        let xpxl1 = xend as i32;
        let ypxl1 = yend.floor() as i32;
        let f = yend.fract();

        if steep {
            self.blend_pixel_additive(
                ypxl1,
                xpxl1,
                scale(r, (1.0 - f) * xgap),
                scale(g, (1.0 - f) * xgap),
                scale(b, (1.0 - f) * xgap),
            );
            self.blend_pixel_additive(
                ypxl1 + 1,
                xpxl1,
                scale(r, f * xgap),
                scale(g, f * xgap),
                scale(b, f * xgap),
            );
        } else {
            self.blend_pixel_additive(
                xpxl1,
                ypxl1,
                scale(r, (1.0 - f) * xgap),
                scale(g, (1.0 - f) * xgap),
                scale(b, (1.0 - f) * xgap),
            );
            self.blend_pixel_additive(
                xpxl1,
                ypxl1 + 1,
                scale(r, f * xgap),
                scale(g, f * xgap),
                scale(b, f * xgap),
            );
        }
        let mut intery = yend + gradient;

        // Second endpoint
        let xend = x1.round();
        let yend = y1 + gradient * (xend - x1);
        let xgap = (x1 + 0.5).fract();
        let xpxl2 = xend as i32;
        let ypxl2 = yend.floor() as i32;
        let f = yend.fract();

        if steep {
            self.blend_pixel_additive(
                ypxl2,
                xpxl2,
                scale(r, (1.0 - f) * xgap),
                scale(g, (1.0 - f) * xgap),
                scale(b, (1.0 - f) * xgap),
            );
            self.blend_pixel_additive(
                ypxl2 + 1,
                xpxl2,
                scale(r, f * xgap),
                scale(g, f * xgap),
                scale(b, f * xgap),
            );
        } else {
            self.blend_pixel_additive(
                xpxl2,
                ypxl2,
                scale(r, (1.0 - f) * xgap),
                scale(g, (1.0 - f) * xgap),
                scale(b, (1.0 - f) * xgap),
            );
            self.blend_pixel_additive(
                xpxl2,
                ypxl2 + 1,
                scale(r, f * xgap),
                scale(g, f * xgap),
                scale(b, f * xgap),
            );
        }

        for x in (xpxl1 + 1)..xpxl2 {
            let f = intery.fract();
            let ipart = intery.floor() as i32;
            if steep {
                self.blend_pixel_additive(
                    ipart,
                    x,
                    scale(r, 1.0 - f),
                    scale(g, 1.0 - f),
                    scale(b, 1.0 - f),
                );
                self.blend_pixel_additive(ipart + 1, x, scale(r, f), scale(g, f), scale(b, f));
            } else {
                self.blend_pixel_additive(
                    x,
                    ipart,
                    scale(r, 1.0 - f),
                    scale(g, 1.0 - f),
                    scale(b, 1.0 - f),
                );
                self.blend_pixel_additive(x, ipart + 1, scale(r, f), scale(g, f), scale(b, f));
            }
            intery += gradient;
        }
    }

    // ========================================================================
    // Subpixel & Gradient Primitives (Level 1)
    // Composes: blend_pixel_additive
    // ========================================================================

    /// Subpixel particle splat — distributes a point's brightness across 4 pixels
    /// using bilinear weighting from the fractional position. Use for particles,
    /// stars, and any point that needs smooth subpixel positioning.
    pub fn splat_pixel(&mut self, x: f32, y: f32, r: u8, g: u8, b: u8, intensity: f32) {
        let ix = x.floor() as i32;
        let iy = y.floor() as i32;
        let fx = x - ix as f32;
        let fy = y - iy as f32;

        let scale = |c: u8, w: f32| -> u8 { (c as f32 * w).min(255.0) as u8 };

        let w00 = (1.0 - fx) * (1.0 - fy) * intensity;
        let w10 = fx * (1.0 - fy) * intensity;
        let w01 = (1.0 - fx) * fy * intensity;
        let w11 = fx * fy * intensity;

        self.blend_pixel_additive(ix, iy, scale(r, w00), scale(g, w00), scale(b, w00));
        self.blend_pixel_additive(ix + 1, iy, scale(r, w10), scale(g, w10), scale(b, w10));
        self.blend_pixel_additive(ix, iy + 1, scale(r, w01), scale(g, w01), scale(b, w01));
        self.blend_pixel_additive(ix + 1, iy + 1, scale(r, w11), scale(g, w11), scale(b, w11));
    }

    /// Filled circle with radial gradient falloff (for glows, light sources, lens flares).
    /// `falloff` controls the curve: 1.0=linear, 2.0=quadratic (natural glow), 0.5=wide glow.
    /// Uses additive blending so multiple glows accumulate naturally.
    pub fn fill_circle_gradient(
        &mut self,
        cx: i32,
        cy: i32,
        radius: i32,
        r: u8,
        g: u8,
        b: u8,
        falloff: f32,
    ) {
        if radius <= 0 {
            return;
        }
        let r_sq = (radius * radius) as f32;
        let r_f = radius as f32;

        let y_start = (cy - radius).max(0);
        let y_end = (cy + radius).min(self.height as i32 - 1);
        let x_start = (cx - radius).max(0);
        let x_end = (cx + radius).min(self.width as i32 - 1);

        for y in y_start..=y_end {
            let dy = (y - cy) as f32;
            let dy_sq = dy * dy;
            for x in x_start..=x_end {
                let dx = (x - cx) as f32;
                let dist_sq = dx * dx + dy_sq;
                if dist_sq > r_sq {
                    continue;
                }

                let dist = dist_sq.sqrt();
                let t = (1.0 - dist / r_f).powf(falloff);
                self.blend_pixel_additive(
                    x,
                    y,
                    (r as f32 * t) as u8,
                    (g as f32 * t) as u8,
                    (b as f32 * t) as u8,
                );
            }
        }
    }

    // ========================================================================
    // Gouraud Shaded Primitives (Level 1 + 2)
    // Composes: write_pixel (Level 0) → hline_gouraud → fill_polygon_gouraud
    // ========================================================================

    /// Horizontal line with per-pixel color interpolation between two endpoints.
    /// Colors are passed as f32 for precision during interpolation.
    pub fn hline_gouraud(
        &mut self,
        x1: i32,
        x2: i32,
        y: i32,
        r1: f32,
        g1: f32,
        b1: f32,
        r2: f32,
        g2: f32,
        b2: f32,
    ) {
        if y < 0 || y >= self.height as i32 {
            return;
        }
        // Ensure x1 <= x2, swapping colors to match
        let (x1, x2, r1, g1, b1, r2, g2, b2) = if x1 <= x2 {
            (x1, x2, r1, g1, b1, r2, g2, b2)
        } else {
            (x2, x1, r2, g2, b2, r1, g1, b1)
        };

        let start = x1.max(0);
        let end = x2.min(self.width as i32 - 1);
        if start > end {
            return;
        }

        let span = (x2 - x1) as f32;
        if span < 1.0 {
            // Degenerate: single pixel
            self.hline(start, end, y, r1 as u8, g1 as u8, b1 as u8);
            return;
        }

        let inv_span = 1.0 / span;
        let dr = (r2 - r1) * inv_span;
        let dg = (g2 - g1) * inv_span;
        let db = (b2 - b1) * inv_span;

        // Adjust for clipping on the left
        let offset = (start - x1) as f32;
        let mut cr = r1 + dr * offset;
        let mut cg = g1 + dg * offset;
        let mut cb = b1 + db * offset;

        let mut idx = self.pixel_index(start as u32, y as u32);
        for _ in start..=end {
            write_pixel(
                &mut self.pixels[idx..idx + 4],
                cr.clamp(0.0, 255.0) as u8,
                cg.clamp(0.0, 255.0) as u8,
                cb.clamp(0.0, 255.0) as u8,
            );
            cr += dr;
            cg += dg;
            cb += db;
            idx += 4;
        }
    }

    /// Gouraud-shaded polygon fill with per-vertex colors.
    /// Each vertex is (screen_x, screen_y, r, g, b).
    /// Colors are interpolated smoothly across the triangle using scanline rasterization.
    pub fn fill_polygon_gouraud(&mut self, vertices: &[(f32, f32, u8, u8, u8)]) {
        if vertices.len() < 3 {
            return;
        }

        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        for &(_, y, _, _, _) in vertices {
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }

        let min_y = (min_y as i32).max(0);
        let max_y = (max_y as i32).min(self.height as i32 - 1);

        // Intersections carry (x, r, g, b) as f32 for interpolation precision
        let mut intersections: Vec<(f32, f32, f32, f32)> = Vec::with_capacity(vertices.len());
        let n = vertices.len();

        for y in min_y..=max_y {
            intersections.clear();
            let yf = y as f32 + 0.5;

            for i in 0..n {
                let (x1, y1, r1, g1, b1) = vertices[i];
                let (x2, y2, r2, g2, b2) = vertices[(i + 1) % n];

                if (y1 <= yf && y2 > yf) || (y2 <= yf && y1 > yf) {
                    let t = (yf - y1) / (y2 - y1);
                    let x = x1 + t * (x2 - x1);
                    let r = r1 as f32 + t * (r2 as f32 - r1 as f32);
                    let g = g1 as f32 + t * (g2 as f32 - g1 as f32);
                    let b = b1 as f32 + t * (b2 as f32 - b1 as f32);
                    intersections.push((x, r, g, b));
                }
            }

            intersections.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

            for pair in intersections.chunks_exact(2) {
                let (x1, r1, g1, b1) = pair[0];
                let (x2, r2, g2, b2) = pair[1];
                self.hline_gouraud(x1 as i32, x2 as i32, y, r1, g1, b1, r2, g2, b2);
            }
        }
    }

    // ========================================================================
    // Alpha-blended Gouraud Primitives (Level 1 + 2)
    // Composes: blend_channel (Level 0) → hline_gouraud_blend → fill_polygon_gouraud_blend
    // ========================================================================

    /// Horizontal line with per-pixel color+alpha interpolation, alpha-blended onto buffer.
    pub fn hline_gouraud_blend(
        &mut self,
        x1: i32,
        x2: i32,
        y: i32,
        r1: f32,
        g1: f32,
        b1: f32,
        a1: f32,
        r2: f32,
        g2: f32,
        b2: f32,
        a2: f32,
    ) {
        if y < 0 || y >= self.height as i32 {
            return;
        }
        let (x1, x2, r1, g1, b1, a1, r2, g2, b2, a2) = if x1 <= x2 {
            (x1, x2, r1, g1, b1, a1, r2, g2, b2, a2)
        } else {
            (x2, x1, r2, g2, b2, a2, r1, g1, b1, a1)
        };

        let start = x1.max(0);
        let end = x2.min(self.width as i32 - 1);
        if start > end {
            return;
        }

        let span = (x2 - x1) as f32;
        if span < 1.0 {
            let alpha = a1.clamp(0.0, 255.0) as u16;
            if alpha == 0 {
                return;
            }
            let idx = self.pixel_index(start as u32, y as u32);
            self.pixels[idx] = 255;
            self.pixels[idx + 1] = blend_channel(b1 as u8, self.pixels[idx + 1], alpha);
            self.pixels[idx + 2] = blend_channel(g1 as u8, self.pixels[idx + 2], alpha);
            self.pixels[idx + 3] = blend_channel(r1 as u8, self.pixels[idx + 3], alpha);
            return;
        }

        let inv_span = 1.0 / span;
        let dr = (r2 - r1) * inv_span;
        let dg = (g2 - g1) * inv_span;
        let db = (b2 - b1) * inv_span;
        let da = (a2 - a1) * inv_span;

        let offset = (start - x1) as f32;
        let mut cr = r1 + dr * offset;
        let mut cg = g1 + dg * offset;
        let mut cb = b1 + db * offset;
        let mut ca = a1 + da * offset;

        let mut idx = self.pixel_index(start as u32, y as u32);
        for _ in start..=end {
            let alpha = ca.clamp(0.0, 255.0) as u16;
            if alpha > 0 {
                self.pixels[idx] = 255;
                self.pixels[idx + 1] =
                    blend_channel(cb.clamp(0.0, 255.0) as u8, self.pixels[idx + 1], alpha);
                self.pixels[idx + 2] =
                    blend_channel(cg.clamp(0.0, 255.0) as u8, self.pixels[idx + 2], alpha);
                self.pixels[idx + 3] =
                    blend_channel(cr.clamp(0.0, 255.0) as u8, self.pixels[idx + 3], alpha);
            }
            cr += dr;
            cg += dg;
            cb += db;
            ca += da;
            idx += 4;
        }
    }

    /// Gouraud-shaded polygon fill with per-vertex colors and alpha, blended onto buffer.
    /// Each vertex is (screen_x, screen_y, r, g, b, a).
    pub fn fill_polygon_gouraud_blend(&mut self, vertices: &[(f32, f32, u8, u8, u8, u8)]) {
        if vertices.len() < 3 {
            return;
        }

        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        for &(_, y, _, _, _, _) in vertices {
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }

        let min_y = (min_y as i32).max(0);
        let max_y = (max_y as i32).min(self.height as i32 - 1);

        // Intersections carry (x, r, g, b, a) as f32
        let mut intersections: Vec<(f32, f32, f32, f32, f32)> = Vec::with_capacity(vertices.len());
        let n = vertices.len();

        for y in min_y..=max_y {
            intersections.clear();
            let yf = y as f32 + 0.5;

            for i in 0..n {
                let (x1, y1, r1, g1, b1, a1) = vertices[i];
                let (x2, y2, r2, g2, b2, a2) = vertices[(i + 1) % n];

                if (y1 <= yf && y2 > yf) || (y2 <= yf && y1 > yf) {
                    let t = (yf - y1) / (y2 - y1);
                    let x = x1 + t * (x2 - x1);
                    let r = r1 as f32 + t * (r2 as f32 - r1 as f32);
                    let g = g1 as f32 + t * (g2 as f32 - g1 as f32);
                    let b = b1 as f32 + t * (b2 as f32 - b1 as f32);
                    let a = a1 as f32 + t * (a2 as f32 - a1 as f32);
                    intersections.push((x, r, g, b, a));
                }
            }

            intersections.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

            for pair in intersections.chunks_exact(2) {
                let (x1, r1, g1, b1, a1) = pair[0];
                let (x2, r2, g2, b2, a2) = pair[1];
                self.hline_gouraud_blend(x1 as i32, x2 as i32, y, r1, g1, b1, a1, r2, g2, b2, a2);
            }
        }
    }

    // ========================================================================
    // Post-processing (Level 2)
    // Composes: raw pixel access → box_blur → bloom
    // ========================================================================

    /// Separable box blur using a sliding window. O(width*height) regardless of radius.
    /// Clamps at edges (repeats border pixels). Allocates one temporary buffer internally.
    pub fn box_blur(&mut self, radius: u32) {
        if radius == 0 {
            return;
        }
        let w = self.width as i32;
        let h = self.height as i32;
        let r = radius as i32;
        let div = (2 * radius + 1) as u32;

        let mut temp = vec![0u8; self.pixels.len()];

        // --- Horizontal pass: self.pixels → temp ---
        for y in 0..h {
            let (mut sr, mut sg, mut sb) = (0u32, 0u32, 0u32);

            // Build initial sum for x=0: window [-r..r] clamped
            for i in -r..=r {
                let sx = i.clamp(0, w - 1) as u32;
                let idx = (y as u32 * self.width + sx) as usize * 4;
                sr += self.pixels[idx + 3] as u32; // R (ABGR layout)
                sg += self.pixels[idx + 2] as u32;
                sb += self.pixels[idx + 1] as u32;
            }

            // Write x=0
            let idx = (y as u32 * self.width) as usize * 4;
            temp[idx] = 255;
            temp[idx + 3] = (sr / div) as u8;
            temp[idx + 2] = (sg / div) as u8;
            temp[idx + 1] = (sb / div) as u8;

            // Slide for x=1..w
            for x in 1..w {
                let leave_x = (x - 1 - r).clamp(0, w - 1) as u32;
                let enter_x = (x + r).clamp(0, w - 1) as u32;
                let li = (y as u32 * self.width + leave_x) as usize * 4;
                let ei = (y as u32 * self.width + enter_x) as usize * 4;

                sr = sr - self.pixels[li + 3] as u32 + self.pixels[ei + 3] as u32;
                sg = sg - self.pixels[li + 2] as u32 + self.pixels[ei + 2] as u32;
                sb = sb - self.pixels[li + 1] as u32 + self.pixels[ei + 1] as u32;

                let idx = (y as u32 * self.width + x as u32) as usize * 4;
                temp[idx] = 255;
                temp[idx + 3] = (sr / div) as u8;
                temp[idx + 2] = (sg / div) as u8;
                temp[idx + 1] = (sb / div) as u8;
            }
        }

        // --- Vertical pass: temp → self.pixels ---
        for x in 0..w {
            let (mut sr, mut sg, mut sb) = (0u32, 0u32, 0u32);

            for i in -r..=r {
                let sy = i.clamp(0, h - 1) as u32;
                let idx = (sy * self.width + x as u32) as usize * 4;
                sr += temp[idx + 3] as u32;
                sg += temp[idx + 2] as u32;
                sb += temp[idx + 1] as u32;
            }

            let idx = (x as u32) as usize * 4;
            self.pixels[idx] = 255;
            self.pixels[idx + 3] = (sr / div) as u8;
            self.pixels[idx + 2] = (sg / div) as u8;
            self.pixels[idx + 1] = (sb / div) as u8;

            for y in 1..h {
                let leave_y = (y - 1 - r).clamp(0, h - 1) as u32;
                let enter_y = (y + r).clamp(0, h - 1) as u32;
                let li = (leave_y * self.width + x as u32) as usize * 4;
                let ei = (enter_y * self.width + x as u32) as usize * 4;

                sr = sr - temp[li + 3] as u32 + temp[ei + 3] as u32;
                sg = sg - temp[li + 2] as u32 + temp[ei + 2] as u32;
                sb = sb - temp[li + 1] as u32 + temp[ei + 1] as u32;

                let idx = (y as u32 * self.width + x as u32) as usize * 4;
                self.pixels[idx] = 255;
                self.pixels[idx + 3] = (sr / div) as u8;
                self.pixels[idx + 2] = (sg / div) as u8;
                self.pixels[idx + 1] = (sb / div) as u8;
            }
        }
    }

    /// Bloom post-processing: extract bright pixels, blur them, and additively composite back.
    /// - `threshold`: minimum pixel brightness (0-255) to contribute to bloom
    /// - `blur_radius`: size of the blur kernel (2-5 typical)
    /// - `intensity`: strength of the bloom effect (0.0-2.0 typical, >1.0 for strong glow)
    pub fn bloom(&mut self, threshold: u8, blur_radius: u32, intensity: f32) {
        let w = self.width;
        let h = self.height;

        // Step 1: Extract bright pixels into scratch buffer
        let mut bright = PixelBuffer::with_size(w, h);
        let threshold_u16 = threshold as u16;

        for i in (0..self.pixels.len()).step_by(4) {
            let pr = self.pixels[i + 3]; // R in ABGR
            let pg = self.pixels[i + 2];
            let pb = self.pixels[i + 1];
            let luma = (pr as u16 + pg as u16 + pb as u16) / 3;
            if luma > threshold_u16 {
                bright.pixels[i] = 255;
                bright.pixels[i + 3] = pr;
                bright.pixels[i + 2] = pg;
                bright.pixels[i + 1] = pb;
            }
        }

        // Step 2: Blur (two passes approximate a Gaussian)
        bright.box_blur(blur_radius);
        bright.box_blur(blur_radius);

        // Step 3: Additively composite back with intensity scaling
        let scale = (intensity * 256.0).min(512.0) as u32;
        for i in (0..self.pixels.len()).step_by(4) {
            let br = ((bright.pixels[i + 3] as u32 * scale) >> 8).min(255) as u8;
            let bg = ((bright.pixels[i + 2] as u32 * scale) >> 8).min(255) as u8;
            let bb = ((bright.pixels[i + 1] as u32 * scale) >> 8).min(255) as u8;
            self.pixels[i + 3] = self.pixels[i + 3].saturating_add(br);
            self.pixels[i + 2] = self.pixels[i + 2].saturating_add(bg);
            self.pixels[i + 1] = self.pixels[i + 1].saturating_add(bb);
        }
    }

    // ========================================================================
    // Shade Map (Level 2)
    // ========================================================================

    /// Multiply each pixel's RGB by shade[i] >> 8.
    /// shades.len() must equal width * height. Values: 0=black, 256=unchanged.
    /// Use for fog, depth shading, vignette, or any per-pixel brightness modulation.
    pub fn apply_shade_map(&mut self, shades: &[u16]) {
        debug_assert_eq!(shades.len(), (self.width * self.height) as usize);

        let mut idx = 0usize;
        for &shade in shades {
            let s = shade as u32;
            // ABGR layout: [0]=A, [1]=B, [2]=G, [3]=R
            self.pixels[idx + 1] = ((self.pixels[idx + 1] as u32 * s) >> 8) as u8;
            self.pixels[idx + 2] = ((self.pixels[idx + 2] as u32 * s) >> 8) as u8;
            self.pixels[idx + 3] = ((self.pixels[idx + 3] as u32 * s) >> 8) as u8;
            idx += 4;
        }
    }
}

impl Default for PixelBuffer {
    fn default() -> Self {
        Self::new()
    }
}
