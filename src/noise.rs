//! Noise Generation Utilities
//!
//! Provides value noise, fractional Brownian motion (fBm), and related functions
//! for procedural content generation in demoscene effects.

/// Hash-based pseudo-random value for integer grid coordinates.
/// Returns a value in [0.0, 1.0).
#[inline]
pub fn noise_hash(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let mut h = seed.wrapping_add(x as u32).wrapping_mul(374761393);
    h = h.wrapping_add(y as u32).wrapping_mul(668265263);
    h = h.wrapping_add(z as u32).wrapping_mul(2147483647);
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h = h ^ (h >> 16);
    (h & 0x7fff) as f32 / 0x7fff as f32
}

/// 2D version of noise_hash for simpler use cases
#[inline]
pub fn noise_hash_2d(x: i32, y: i32, seed: u32) -> f32 {
    noise_hash(x, y, 0, seed)
}

/// Smoothstep interpolation: 3t² - 2t³
/// Maps [0,1] to [0,1] with smooth acceleration and deceleration.
#[inline]
pub fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Quintic smoothstep (Ken Perlin's improved version): 6t⁵ - 15t⁴ + 10t³
/// Smoother than basic smoothstep with zero first and second derivatives at endpoints.
#[inline]
pub fn smoothstep_quintic(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

/// 3D value noise with smoothstep interpolation.
/// Returns a value in approximately [0.0, 1.0].
pub fn value_noise(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let iz = z.floor() as i32;
    let fx = smoothstep(x - ix as f32);
    let fy = smoothstep(y - iy as f32);
    let fz = smoothstep(z - iz as f32);

    // Sample 8 corners of the unit cube
    let c000 = noise_hash(ix, iy, iz, seed);
    let c100 = noise_hash(ix + 1, iy, iz, seed);
    let c010 = noise_hash(ix, iy + 1, iz, seed);
    let c110 = noise_hash(ix + 1, iy + 1, iz, seed);
    let c001 = noise_hash(ix, iy, iz + 1, seed);
    let c101 = noise_hash(ix + 1, iy, iz + 1, seed);
    let c011 = noise_hash(ix, iy + 1, iz + 1, seed);
    let c111 = noise_hash(ix + 1, iy + 1, iz + 1, seed);

    // Trilinear interpolation
    let x0 = c000 + (c100 - c000) * fx;
    let x1 = c010 + (c110 - c010) * fx;
    let x2 = c001 + (c101 - c001) * fx;
    let x3 = c011 + (c111 - c011) * fx;

    let y0 = x0 + (x1 - x0) * fy;
    let y1 = x2 + (x3 - x2) * fy;

    y0 + (y1 - y0) * fz
}

/// 2D value noise with smoothstep interpolation.
/// Returns a value in approximately [0.0, 1.0].
pub fn value_noise_2d(x: f32, y: f32, seed: u32) -> f32 {
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let fx = smoothstep(x - ix as f32);
    let fy = smoothstep(y - iy as f32);

    let c00 = noise_hash_2d(ix, iy, seed);
    let c10 = noise_hash_2d(ix + 1, iy, seed);
    let c01 = noise_hash_2d(ix, iy + 1, seed);
    let c11 = noise_hash_2d(ix + 1, iy + 1, seed);

    let x0 = c00 + (c10 - c00) * fx;
    let x1 = c01 + (c11 - c01) * fx;

    x0 + (x1 - x0) * fy
}

/// Fractional Brownian motion (fBm) — layer multiple octaves of 3D noise.
/// Each octave doubles in frequency and halves in amplitude.
///
/// # Arguments
/// * `x`, `y`, `z` - Sample coordinates
/// * `octaves` - Number of noise layers to combine (1-8 typical)
/// * `seed` - Random seed for reproducibility
///
/// # Returns
/// Value in approximately [0.0, 1.0] (may slightly exceed due to octave summing)
pub fn fbm(x: f32, y: f32, z: f32, octaves: u32, seed: u32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 0.5;
    let mut frequency = 1.0;
    for _ in 0..octaves {
        value += amplitude * value_noise(x * frequency, y * frequency, z * frequency, seed);
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    value
}

/// 2D Fractional Brownian motion
pub fn fbm_2d(x: f32, y: f32, octaves: u32, seed: u32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 0.5;
    let mut frequency = 1.0;
    for _ in 0..octaves {
        value += amplitude * value_noise_2d(x * frequency, y * frequency, seed);
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    value
}

/// Turbulence — like fBm but uses absolute value for sharper, more chaotic patterns.
/// Useful for fire, clouds, and marble-like effects.
pub fn turbulence(x: f32, y: f32, z: f32, octaves: u32, seed: u32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 0.5;
    let mut frequency = 1.0;
    for _ in 0..octaves {
        value +=
            amplitude * (value_noise(x * frequency, y * frequency, z * frequency, seed) - 0.5).abs()
                * 2.0;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    value
}

/// 2D turbulence
pub fn turbulence_2d(x: f32, y: f32, octaves: u32, seed: u32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 0.5;
    let mut frequency = 1.0;
    for _ in 0..octaves {
        value += amplitude * (value_noise_2d(x * frequency, y * frequency, seed) - 0.5).abs() * 2.0;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    value
}

/// Ridged multifractal noise — creates sharp ridges useful for mountains, lightning.
pub fn ridged(x: f32, y: f32, z: f32, octaves: u32, seed: u32) -> f32 {
    let mut value = 0.0;
    let mut amplitude = 0.5;
    let mut frequency = 1.0;
    let mut weight = 1.0;

    for _ in 0..octaves {
        let signal =
            1.0 - (value_noise(x * frequency, y * frequency, z * frequency, seed) - 0.5).abs()
                * 2.0;
        let signal = signal * signal * weight;
        weight = signal.clamp(0.0, 1.0);
        value += amplitude * signal;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noise_hash_deterministic() {
        let v1 = noise_hash(10, 20, 30, 42);
        let v2 = noise_hash(10, 20, 30, 42);
        assert_eq!(v1, v2);
    }

    #[test]
    fn test_noise_hash_range() {
        for x in -10..10 {
            for y in -10..10 {
                let v = noise_hash(x, y, 0, 12345);
                assert!(v >= 0.0 && v < 1.0);
            }
        }
    }

    #[test]
    fn test_smoothstep_bounds() {
        assert_eq!(smoothstep(0.0), 0.0);
        assert_eq!(smoothstep(1.0), 1.0);
        assert!((smoothstep(0.5) - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_value_noise_continuity() {
        // Test that adjacent samples don't have huge jumps
        let seed = 999;
        for i in 0..100 {
            let x = i as f32 * 0.1;
            let v1 = value_noise(x, 0.0, 0.0, seed);
            let v2 = value_noise(x + 0.01, 0.0, 0.0, seed);
            assert!((v1 - v2).abs() < 0.5, "Noise discontinuity at x={}", x);
        }
    }
}
