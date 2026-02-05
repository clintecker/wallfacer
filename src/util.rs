//! Shared utilities

/// Simple deterministic RNG using xorshift64
/// Good for effects that need reproducible randomness without external dependencies
pub struct Rng {
    state: u64,
}

impl Rng {
    /// Create a new RNG with the given seed
    pub fn new(seed: u64) -> Self {
        Self { state: seed.max(1) } // Ensure non-zero
    }

    /// Get the next random u64
    #[inline]
    pub fn next_u64(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    /// Get a random u32
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    /// Get a random u8
    #[inline]
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }

    /// Get a random f32 in [0, 1)
    #[inline]
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u64() & 0xFFFFFF) as f32 / 0x1000000 as f32
    }

    /// Get a random f32 in [min, max)
    #[inline]
    pub fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_f32() * (max - min)
    }

    /// Get a random i32 in [min, max]
    ///
    /// # Panics
    /// Panics in debug builds if `min > max`
    #[inline]
    pub fn range_i32(&mut self, min: i32, max: i32) -> i32 {
        debug_assert!(min <= max, "range_i32: min ({}) must be <= max ({})", min, max);
        if min >= max {
            return min;
        }
        let range = (max - min + 1) as u64;
        min + (self.next_u64() % range) as i32
    }
}

/// HSV to RGB color conversion
/// h: 0-360, s: 0-1, v: 0-1
pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let h_prime = h / 60.0;
    let x = c * (1.0 - ((h_prime % 2.0) - 1.0).abs());
    let m = v - c;

    let (r1, g1, b1) = match h_prime as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    (
        ((r1 + m) * 255.0) as u8,
        ((g1 + m) * 255.0) as u8,
        ((b1 + m) * 255.0) as u8,
    )
}

/// Linear interpolation between two colors
#[inline]
pub fn lerp_color(c1: (u8, u8, u8), c2: (u8, u8, u8), t: f32) -> (u8, u8, u8) {
    let t = t.clamp(0.0, 1.0);
    (
        (c1.0 as f32 + (c2.0 as f32 - c1.0 as f32) * t) as u8,
        (c1.1 as f32 + (c2.1 as f32 - c1.1 as f32) * t) as u8,
        (c1.2 as f32 + (c2.2 as f32 - c1.2 as f32) * t) as u8,
    )
}

// ============================================================================
// FPS Counter
// ============================================================================

use std::collections::VecDeque;
use std::time::Instant;

/// FPS counter with rolling average
pub struct FpsCounter {
    frame_times: VecDeque<f32>,
    last_frame: Instant,
    sample_count: usize,
}

impl FpsCounter {
    /// Create a new FPS counter with specified sample window
    pub fn new(sample_count: usize) -> Self {
        Self {
            frame_times: VecDeque::with_capacity(sample_count),
            last_frame: Instant::now(),
            sample_count,
        }
    }

    /// Call at the start of each frame to record timing
    /// Returns (delta_time, current_fps, average_fps)
    pub fn tick(&mut self) -> (f32, f32, f32) {
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32();
        self.last_frame = now;

        // Add to rolling window
        self.frame_times.push_back(dt);
        if self.frame_times.len() > self.sample_count {
            self.frame_times.pop_front();
        }

        let current_fps = if dt > 0.0 { 1.0 / dt } else { 0.0 };
        let avg_dt: f32 =
            self.frame_times.iter().sum::<f32>() / self.frame_times.len().max(1) as f32;
        let avg_fps = if avg_dt > 0.0 { 1.0 / avg_dt } else { 0.0 };

        (dt, current_fps, avg_fps)
    }

    /// Get the average frame time in milliseconds
    pub fn avg_frame_time_ms(&self) -> f32 {
        let avg_dt: f32 =
            self.frame_times.iter().sum::<f32>() / self.frame_times.len().max(1) as f32;
        avg_dt * 1000.0
    }

    /// Get min/max FPS from sample window
    pub fn min_max_fps(&self) -> (f32, f32) {
        if self.frame_times.is_empty() {
            return (0.0, 0.0);
        }
        let min_dt = self
            .frame_times
            .iter()
            .cloned()
            .fold(f32::INFINITY, f32::min);
        let max_dt = self.frame_times.iter().cloned().fold(0.0, f32::max);
        let max_fps = if min_dt > 0.0 { 1.0 / min_dt } else { 0.0 };
        let min_fps = if max_dt > 0.0 { 1.0 / max_dt } else { 0.0 };
        (min_fps, max_fps)
    }

    /// Get total number of frames recorded
    pub fn frame_count(&self) -> usize {
        self.frame_times.len()
    }

    /// Get standard deviation of frame times in milliseconds
    pub fn std_dev_ms(&self) -> f32 {
        if self.frame_times.len() < 2 {
            return 0.0;
        }
        let mean = self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32;
        let variance = self.frame_times.iter()
            .map(|&dt| (dt - mean).powi(2))
            .sum::<f32>() / self.frame_times.len() as f32;
        variance.sqrt() * 1000.0
    }

    /// Get percentile frame times in milliseconds (sorted copy)
    /// Returns (1st percentile, 50th/median, 99th percentile)
    pub fn percentiles_ms(&self) -> (f32, f32, f32) {
        if self.frame_times.is_empty() {
            return (0.0, 0.0, 0.0);
        }
        let mut sorted: Vec<f32> = self.frame_times.iter().cloned().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let len = sorted.len();
        let p1_idx = (len as f32 * 0.01).floor() as usize;
        let p50_idx = len / 2;
        let p99_idx = ((len as f32 * 0.99).floor() as usize).min(len - 1);

        (
            sorted[p1_idx] * 1000.0,
            sorted[p50_idx] * 1000.0,
            sorted[p99_idx] * 1000.0,
        )
    }
}
