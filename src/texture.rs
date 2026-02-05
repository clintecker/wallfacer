//! Texture System for Demoscene Effects
//!
//! Provides texture storage, sampling, and procedural generation.

/// A texture stored as RGBA pixels
#[derive(Clone)]
pub struct Texture {
    width: u32,
    height: u32,
    pixels: Vec<u8>, // RGBA format, 4 bytes per pixel
}

impl Texture {
    /// Create a new empty texture
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; (width * height * 4) as usize],
        }
    }

    /// Create texture from raw RGBA data
    pub fn from_rgba(width: u32, height: u32, data: Vec<u8>) -> Option<Self> {
        if data.len() == (width * height * 4) as usize {
            Some(Self {
                width,
                height,
                pixels: data,
            })
        } else {
            None
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

    /// Set a pixel in the texture
    #[inline]
    pub fn set_pixel(&mut self, x: u32, y: u32, r: u8, g: u8, b: u8, a: u8) {
        if x < self.width && y < self.height {
            let idx = ((y * self.width + x) * 4) as usize;
            self.pixels[idx] = r;
            self.pixels[idx + 1] = g;
            self.pixels[idx + 2] = b;
            self.pixels[idx + 3] = a;
        }
    }

    /// Fast nearest-neighbor sample using texel coordinates with bitmask wrapping.
    /// Only works correctly for power-of-2 sized textures.
    /// Takes texel-space coordinates (not UV), handles negatives via two's complement.
    #[inline]
    pub fn sample_texel(&self, tx: i32, ty: i32) -> (u8, u8, u8) {
        let x = (tx as u32) & (self.width - 1);
        let y = (ty as u32) & (self.height - 1);
        let idx = ((y * self.width + x) * 4) as usize;
        (self.pixels[idx], self.pixels[idx + 1], self.pixels[idx + 2])
    }

    /// Sample texture with UV coordinates (0.0 to 1.0, wrapping)
    /// Returns (r, g, b) - alpha is discarded for simplicity
    #[inline]
    pub fn sample(&self, u: f32, v: f32) -> (u8, u8, u8) {
        // Wrap UV coordinates using rem_euclid for correct negative handling
        let u = u.rem_euclid(1.0);
        let v = v.rem_euclid(1.0);

        let x = (u * self.width as f32) as u32 % self.width;
        let y = (v * self.height as f32) as u32 % self.height;

        let idx = ((y * self.width + x) * 4) as usize;
        (self.pixels[idx], self.pixels[idx + 1], self.pixels[idx + 2])
    }

    /// Sample texture with UV coordinates, returning RGBA including alpha channel
    #[inline]
    pub fn sample_rgba(&self, u: f32, v: f32) -> (u8, u8, u8, u8) {
        let u = u.rem_euclid(1.0);
        let v = v.rem_euclid(1.0);

        let x = (u * self.width as f32) as u32 % self.width;
        let y = (v * self.height as f32) as u32 % self.height;

        let idx = ((y * self.width + x) * 4) as usize;
        (
            self.pixels[idx],
            self.pixels[idx + 1],
            self.pixels[idx + 2],
            self.pixels[idx + 3],
        )
    }

    /// Sample with bilinear interpolation for smoother results
    pub fn sample_bilinear(&self, u: f32, v: f32) -> (u8, u8, u8) {
        let u = u.rem_euclid(1.0) * self.width as f32;
        let v = v.rem_euclid(1.0) * self.height as f32;

        let x0 = u.floor() as u32 % self.width;
        let y0 = v.floor() as u32 % self.height;
        let x1 = (x0 + 1) % self.width;
        let y1 = (y0 + 1) % self.height;

        let fx = u.fract();
        let fy = v.fract();

        // Sample 4 corners
        let c00 = self.get_pixel_internal(x0, y0);
        let c10 = self.get_pixel_internal(x1, y0);
        let c01 = self.get_pixel_internal(x0, y1);
        let c11 = self.get_pixel_internal(x1, y1);

        // Bilinear interpolation with clamping for numerical stability
        let lerp = |a: u8, b: u8, t: f32| -> u8 {
            let result = a as f32 + (b as f32 - a as f32) * t;
            result.clamp(0.0, 255.0) as u8
        };

        let r = lerp(lerp(c00.0, c10.0, fx), lerp(c01.0, c11.0, fx), fy);
        let g = lerp(lerp(c00.1, c10.1, fx), lerp(c01.1, c11.1, fx), fy);
        let b = lerp(lerp(c00.2, c10.2, fx), lerp(c01.2, c11.2, fx), fy);

        (r, g, b)
    }

    #[inline]
    fn get_pixel_internal(&self, x: u32, y: u32) -> (u8, u8, u8) {
        let idx = ((y * self.width + x) * 4) as usize;
        (self.pixels[idx], self.pixels[idx + 1], self.pixels[idx + 2])
    }
}

// ============================================================================
// IndexedTexture — one u8 palette index per pixel
// ============================================================================

/// A texture that stores one u8 palette index per pixel instead of RGBA.
/// Designed for the classic demoscene pattern: texture holds luminance/index,
/// color comes from a palette lookup with an offset for cycling.
#[derive(Clone)]
pub struct IndexedTexture {
    width: u32,
    height: u32,
    indices: Vec<u8>,
}

impl IndexedTexture {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            indices: vec![0; (width * height) as usize],
        }
    }

    /// Convert a grayscale Texture to indexed (extracts R channel as index)
    pub fn from_grayscale(tex: &Texture) -> Self {
        let len = (tex.width * tex.height) as usize;
        let mut indices = Vec::with_capacity(len);
        for i in 0..len {
            indices.push(tex.pixels[i * 4]); // R channel
        }
        Self {
            width: tex.width,
            height: tex.height,
            indices,
        }
    }

    pub fn set_index(&mut self, x: u32, y: u32, index: u8) {
        if x < self.width && y < self.height {
            self.indices[(y * self.width + x) as usize] = index;
        }
    }

    /// Fast sample with bitmask wrapping (power-of-2 only)
    #[inline]
    pub fn sample_index(&self, tx: i32, ty: i32) -> u8 {
        let x = (tx as u32) & (self.width - 1);
        let y = (ty as u32) & (self.height - 1);
        self.indices[(y * self.width + x) as usize]
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }
}

// ============================================================================
// MipTexture — mipmapped RGBA texture chain
// ============================================================================

/// A mipmapped texture: a chain of progressively halved Texture levels.
/// Level 0 is the original, level N is original_size >> N.
pub struct MipTexture {
    levels: Vec<Texture>,
}

impl MipTexture {
    /// Build mip chain by box-filtering 2x2 blocks. Source must be power-of-2.
    pub fn from_texture(tex: &Texture) -> Self {
        let mut levels = vec![tex.clone()];
        let mut w = tex.width;
        let mut h = tex.height;

        while w > 1 && h > 1 {
            let prev = &levels[levels.len() - 1];
            let new_w = w / 2;
            let new_h = h / 2;
            let mut down = Texture::new(new_w, new_h);

            for dy in 0..new_h {
                for dx in 0..new_w {
                    let sx = dx * 2;
                    let sy = dy * 2;
                    // Average 2x2 block
                    let mut r = 0u32;
                    let mut g = 0u32;
                    let mut b = 0u32;
                    let mut a = 0u32;
                    for oy in 0..2 {
                        for ox in 0..2 {
                            let idx = (((sy + oy) * w + (sx + ox)) * 4) as usize;
                            r += prev.pixels[idx] as u32;
                            g += prev.pixels[idx + 1] as u32;
                            b += prev.pixels[idx + 2] as u32;
                            a += prev.pixels[idx + 3] as u32;
                        }
                    }
                    down.set_pixel(
                        dx,
                        dy,
                        (r / 4) as u8,
                        (g / 4) as u8,
                        (b / 4) as u8,
                        (a / 4) as u8,
                    );
                }
            }

            levels.push(down);
            w = new_w;
            h = new_h;
        }

        Self { levels }
    }

    /// Number of mip levels
    pub fn level_count(&self) -> usize {
        self.levels.len()
    }

    /// Access a specific level (for direct Texture API use)
    pub fn level(&self, level: u32) -> &Texture {
        let l = (level as usize).min(self.levels.len() - 1);
        &self.levels[l]
    }

    /// Nearest-neighbor sample at chosen mip level.
    /// tx/ty are in level-0 texel space; internally shifted by `level`.
    /// Bitmask wrapping at each level's resolution.
    #[inline]
    pub fn sample_mipped(&self, tx: i32, ty: i32, level: u32) -> (u8, u8, u8) {
        let l = (level as usize).min(self.levels.len() - 1);
        let tex = &self.levels[l];
        // Shift coords to match this mip level's resolution
        tex.sample_texel(tx >> l as i32, ty >> l as i32)
    }
}

// ============================================================================
// IndexedMipTexture — mipmapped palette-indexed texture chain
// ============================================================================

/// Mipmapped version of IndexedTexture. Chain of halved IndexedTexture levels.
pub struct IndexedMipTexture {
    levels: Vec<IndexedTexture>,
}

impl IndexedMipTexture {
    /// Build mip chain from an IndexedTexture.
    /// Downsampling averages 2x2 blocks (clamped to u8).
    pub fn from_indexed(tex: &IndexedTexture) -> Self {
        let mut levels = vec![tex.clone()];
        let mut w = tex.width;
        let mut h = tex.height;

        while w > 1 && h > 1 {
            let prev = &levels[levels.len() - 1];
            let new_w = w / 2;
            let new_h = h / 2;
            let mut down = IndexedTexture::new(new_w, new_h);

            for dy in 0..new_h {
                for dx in 0..new_w {
                    let sx = dx * 2;
                    let sy = dy * 2;
                    let mut sum = 0u32;
                    for oy in 0..2 {
                        for ox in 0..2 {
                            sum += prev.indices[((sy + oy) * w + (sx + ox)) as usize] as u32;
                        }
                    }
                    down.indices[(dy * new_w + dx) as usize] = (sum / 4) as u8;
                }
            }

            levels.push(down);
            w = new_w;
            h = new_h;
        }

        Self { levels }
    }

    /// Build from a grayscale Texture directly
    pub fn from_grayscale(tex: &Texture) -> Self {
        Self::from_indexed(&IndexedTexture::from_grayscale(tex))
    }

    pub fn level_count(&self) -> usize {
        self.levels.len()
    }

    /// Sample palette index at chosen mip level.
    /// tx/ty in level-0 texel space, shifted internally.
    #[inline]
    pub fn sample_index_mipped(&self, tx: i32, ty: i32, level: u32) -> u8 {
        let l = (level as usize).min(self.levels.len() - 1);
        self.levels[l].sample_index(tx >> l as i32, ty >> l as i32)
    }
}

// ============================================================================
// Procedural Texture Generators
// ============================================================================

impl Texture {
    /// Generate a checkerboard pattern
    pub fn checkerboard(size: u32, tile_size: u32, c1: (u8, u8, u8), c2: (u8, u8, u8)) -> Self {
        let mut tex = Self::new(size, size);
        for y in 0..size {
            for x in 0..size {
                let checker = ((x / tile_size) + (y / tile_size)) % 2 == 0;
                let (r, g, b) = if checker { c1 } else { c2 };
                tex.set_pixel(x, y, r, g, b, 255);
            }
        }
        tex
    }

    /// Generate an XOR pattern (classic demoscene texture)
    pub fn xor_pattern(size: u32) -> Self {
        let mut tex = Self::new(size, size);
        for y in 0..size {
            for x in 0..size {
                let v = (x ^ y) as u8;
                tex.set_pixel(x, y, v, v, v, 255);
            }
        }
        tex
    }

    /// Generate a plasma texture using sine waves
    pub fn plasma(size: u32, palette: &[(u8, u8, u8)]) -> Self {
        let mut tex = Self::new(size, size);
        let scale = std::f32::consts::TAU / size as f32;

        for y in 0..size {
            for x in 0..size {
                let fx = x as f32 * scale;
                let fy = y as f32 * scale;

                // Sum of sines
                let v1 = (fx * 2.0).sin();
                let v2 = (fy * 3.0).sin();
                let v3 = ((fx + fy) * 1.5).sin();
                let v4 = ((fx * fx + fy * fy).sqrt() * 2.0).sin();

                let sum = (v1 + v2 + v3 + v4 + 4.0) / 8.0; // Normalize to 0-1
                let idx = (sum * (palette.len() - 1) as f32) as usize;
                let (r, g, b) = palette[idx.min(palette.len() - 1)];

                tex.set_pixel(x, y, r, g, b, 255);
            }
        }
        tex
    }
}
