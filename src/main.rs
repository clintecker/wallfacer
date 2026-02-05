// Allow unused code for designed-but-not-yet-used APIs
// Remove these as the codebase matures
#![allow(dead_code)]

mod control;
mod display;
mod effects;
mod geometry;
mod input;
mod math3d;
mod noise;
mod particles;
mod regions;
mod texture;
mod util;

use display::{
    draw_text, Display, InputEvent, PixelBuffer, RenderTarget, DEFAULT_HEIGHT, DEFAULT_WIDTH,
};
use effects::{
    Bobs, CopperBars, DotTunnel, Dvd, Earth, Earth2, Effect, EtherealInk, Fire, Glenz,
    GravityBalls, Julia, LavaRegions, Metaballs, Plasma, Raycaster, RegionFire, Ripples,
    Rotozoomer, Rubber, ScrollerDemo, Snowfall, Starfield, TestPattern, TextFxDemo, Tunnel,
    VectorBalls, Vortex, Worms,
};
use control::{Command, Controller};
use input::CalibrationMode;
use regions::Scene;
use sdl2::keyboard::Keycode;
use util::FpsCounter;

#[derive(PartialEq)]
enum AppMode {
    Effect,
    Calibration,
}

/// Mask all regions in the scene by filling them with the specified color
fn mask_regions(buffer: &mut PixelBuffer, scene: &Scene, color: (u8, u8, u8)) {
    use regions::Shape;
    for region in &scene.regions {
        match region.get_shape() {
            Shape::Polygon(poly) => {
                buffer.fill_polygon(&poly.as_tuples(), color.0, color.1, color.2);
            }
            Shape::Circle(circle) => {
                buffer.fill_circle(
                    circle.center.x as i32,
                    circle.center.y as i32,
                    circle.radius as i32,
                    color.0,
                    color.1,
                    color.2,
                );
            }
        }
    }
}

/// Display rotation for portrait/landscape modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Rotation {
    None,      // 0°
    Cw90,      // 90° clockwise (portrait)
    Cw180,     // 180° (upside down)
    Cw270,     // 270° clockwise / 90° counter-clockwise (portrait)
}

/// Transform window coordinates to content coordinates based on rotation
/// This is the inverse of the rotation applied during rendering
fn transform_mouse(wx: i32, wy: i32, rotation: Rotation, content_w: u32, content_h: u32) -> (i32, i32) {
    match rotation {
        Rotation::None => (wx, wy),
        Rotation::Cw90 => {
            // Rotation: content(x,y) -> window(h-1-y, x) where h=content_h
            // Inverse: window(wx,wy) -> content(wy, h-1-wx)
            (wy, content_h as i32 - 1 - wx)
        }
        Rotation::Cw180 => {
            (content_w as i32 - 1 - wx, content_h as i32 - 1 - wy)
        }
        Rotation::Cw270 => {
            // Rotation: content(x,y) -> window(y, w-1-x) where w=content_w
            // Inverse: window(wx,wy) -> content(w-1-wy, wx)
            (content_w as i32 - 1 - wy, wx)
        }
    }
}

/// Transform mouse movement delta from window space to content space
/// This makes mouse movement feel natural when display is rotated
fn transform_mouse_delta(dx: i32, dy: i32, rotation: Rotation) -> (i32, i32) {
    match rotation {
        Rotation::None => (dx, dy),
        Rotation::Cw90 => {
            // Content is rotated 90° CW on display
            // Physical right (window +X) should move cursor right in content (+X)
            // But window +X is content -Y after rotation, so we need to transform:
            // content delta = (dy, -dx)
            (dy, -dx)
        }
        Rotation::Cw180 => {
            // Content is upside down
            // Physical right (+X) should move cursor right (+X in content = -X in window)
            (-dx, -dy)
        }
        Rotation::Cw270 => {
            // Content is rotated 270° CW (90° CCW)
            // Physical right (window +X) is content +Y, physical down is content -X
            (-dy, dx)
        }
    }
}

/// Apply mouse acceleration curve
/// Small movements stay linear, fast movements get amplified
fn apply_mouse_acceleration(dx: i32, dy: i32) -> (i32, i32) {
    let speed = ((dx * dx + dy * dy) as f32).sqrt();

    // Acceleration curve: linear up to threshold, then quadratic boost
    // threshold=3: below this, no acceleration
    // multiplier ramps from 1.0 to ~2.5 for fast movements
    let threshold = 3.0;
    let multiplier = if speed <= threshold {
        1.0
    } else {
        // Quadratic ramp: faster movements get more boost
        // At speed=10, multiplier ≈ 1.8
        // At speed=20, multiplier ≈ 2.5
        1.0 + ((speed - threshold) / 10.0).min(1.5)
    };

    let ax = (dx as f32 * multiplier).round() as i32;
    let ay = (dy as f32 * multiplier).round() as i32;

    (ax, ay)
}

/// Draw a simple arrow cursor at the given position
fn draw_software_cursor(buffer: &mut PixelBuffer, x: i32, y: i32) {
    // Simple arrow cursor (pointing up-left like standard cursor)
    let cursor = [
        (0, 0), (0, 1), (0, 2), (0, 3), (0, 4), (0, 5), (0, 6), (0, 7), (0, 8), (0, 9),
        (1, 1), (1, 2), (1, 3), (1, 4), (1, 5), (1, 6), (1, 7), (1, 8),
        (2, 2), (2, 3), (2, 4), (2, 5), (2, 6), (2, 7),
        (3, 3), (3, 4), (3, 5), (3, 6),
        (4, 4), (4, 5), (4, 6), (4, 7),
        (5, 5), (5, 6), (5, 7), (5, 8),
        (6, 6), (6, 7), (6, 8), (6, 9),
        (7, 7), (7, 8),
        (8, 8),
    ];

    // Draw black outline
    for &(dx, dy) in &cursor {
        buffer.set_pixel(x + dx - 1, y + dy, 0, 0, 0);
        buffer.set_pixel(x + dx + 1, y + dy, 0, 0, 0);
        buffer.set_pixel(x + dx, y + dy - 1, 0, 0, 0);
        buffer.set_pixel(x + dx, y + dy + 1, 0, 0, 0);
    }

    // Draw white cursor
    for &(dx, dy) in &cursor {
        buffer.set_pixel(x + dx, y + dy, 255, 255, 255);
    }
}

/// Parsed command line options
struct AppOptions {
    width: u32,
    height: u32,
    vsync: bool,
    start_effect: Option<usize>,
    rotation: Rotation,
    benchmark_seconds: Option<f32>,
    scene_file: Option<String>,
}

/// Parse command line arguments
fn parse_args() -> AppOptions {
    let args: Vec<String> = std::env::args().collect();
    let mut opts = AppOptions {
        width: DEFAULT_WIDTH,
        height: DEFAULT_HEIGHT,
        vsync: true,
        start_effect: Some(0),
        rotation: Rotation::None,
        benchmark_seconds: None,
        scene_file: None,
    };

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--no-vsync" => opts.vsync = false,
            "--width" | "-w" => {
                if i + 1 < args.len() {
                    if let Ok(w) = args[i + 1].parse::<u32>() {
                        opts.width = w;
                    }
                    i += 1;
                }
            },
            "--height" | "-h" => {
                if i + 1 < args.len() {
                    if let Ok(h) = args[i + 1].parse::<u32>() {
                        opts.height = h;
                    }
                    i += 1;
                }
            },
            "--resolution" | "-r" => {
                if i + 1 < args.len() {
                    // Parse WxH format (e.g., 1920x1080)
                    let parts: Vec<&str> = args[i + 1].split('x').collect();
                    if parts.len() == 2 {
                        if let (Ok(w), Ok(h)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                            opts.width = w;
                            opts.height = h;
                        }
                    }
                    i += 1;
                }
            },
            "--effect" | "-e" => {
                if i + 1 < args.len() {
                    if let Ok(e) = args[i + 1].parse::<usize>() {
                        opts.start_effect = Some(e);
                    }
                    i += 1;
                }
            },
            "--rotate" => {
                if i + 1 < args.len() {
                    opts.rotation = match args[i + 1].as_str() {
                        "90" | "cw90" => Rotation::Cw90,
                        "180" => Rotation::Cw180,
                        "270" | "ccw90" | "cw270" => Rotation::Cw270,
                        _ => Rotation::None,
                    };
                    i += 1;
                }
            },
            "--benchmark" | "-b" => {
                // Default to 10 seconds, or parse optional duration
                let mut duration = 10.0;
                if i + 1 < args.len() {
                    if let Ok(secs) = args[i + 1].parse::<f32>() {
                        duration = secs;
                        i += 1;
                    }
                }
                opts.benchmark_seconds = Some(duration);
                opts.vsync = false; // Benchmark always runs without vsync
            },
            "--scene" | "-s" => {
                if i + 1 < args.len() {
                    opts.scene_file = Some(args[i + 1].clone());
                    i += 1;
                }
            },
            "--help" => {
                println!("Usage: wallfacer [OPTIONS]");
                println!();
                println!("Options:");
                println!(
                    "  --width W, -w W       Set window width (default: {})",
                    DEFAULT_WIDTH
                );
                println!(
                    "  --height H, -h H      Set window height (default: {})",
                    DEFAULT_HEIGHT
                );
                println!("  --resolution WxH, -r WxH  Set resolution (e.g., 1920x1080)");
                println!("  --effect N, -e N      Start with effect number N (0-indexed)");
                println!("  --rotate N            Rotate display (0, 90, 180, 270)");
                println!("  --benchmark [S], -b   Run benchmark for S seconds (default: 10)");
                println!("  --scene FILE, -s      Load scene/regions from FILE");
                println!("  --no-vsync            Disable VSync for uncapped framerate");
                println!("  --help                Show this help message");
                std::process::exit(0);
            },
            _ => {},
        }
        i += 1;
    }

    opts
}

fn main() -> Result<(), String> {
    let opts = parse_args();
    let width = opts.width;
    let height = opts.height;
    let vsync = opts.vsync;
    let start_effect = opts.start_effect;
    let rotation = opts.rotation;
    let benchmark_seconds = opts.benchmark_seconds;
    let scene_file = opts.scene_file;

    // For 90/270 rotation, the window dimensions are swapped
    let (window_w, window_h) = match rotation {
        Rotation::Cw90 | Rotation::Cw270 => (height, width),
        _ => (width, height),
    };

    let (mut display, texture_creator) =
        Display::with_options("wallfacer", window_w, window_h, vsync)?;
    let mut target = RenderTarget::with_size(&texture_creator, window_w, window_h)?;
    // Effects render at original dimensions, then we rotate for display
    let mut buffer = PixelBuffer::with_size(width, height);

    // FPS counter - use larger window for benchmarks to capture all frames
    let sample_count = if benchmark_seconds.is_some() { 100_000 } else { 60 };
    let mut fps_counter = FpsCounter::new(sample_count);
    let mut show_fps = false;
    let mut total_elapsed = 0.0f32;

    // Load scene or create new
    let scene_path = scene_file.as_deref().unwrap_or("scene.json");
    let scene = Scene::load(scene_path).unwrap_or_else(|e| {
        if benchmark_seconds.is_some() {
            eprintln!("Warning: Failed to load scene '{}': {}", scene_path, e);
        }
        Scene::new("default")
    });
    if benchmark_seconds.is_some() {
        println!("Scene: {} ({} regions)", scene_path, scene.regions.len());
    }

    // Available effects
    let mut effects: Vec<Box<dyn Effect>> = vec![
        Box::new(Plasma::new()),       // 1
        Box::new(Starfield::new()),    // 2
        Box::new(Fire::new()),         // 3
        Box::new(ScrollerDemo::new()), // 4
        Box::new(TextFxDemo::new()),   // 5
        Box::new(Worms::new()),        // 6
        Box::new(Dvd::new()),          // 7
        Box::new(CopperBars::new()),   // 8
        Box::new(Glenz::new()),        // 9
        Box::new(Rotozoomer::new()),   // 0
        Box::new(Tunnel::new()),       // -
        Box::new(Bobs::new()),         // =
        Box::new(Earth::new()),        // [
        Box::new(Earth2::new()),       // ]
        Box::new(Snowfall::new()),     // \
        Box::new(EtherealInk::new()),  // ;
        Box::new(VectorBalls::new()),  // '
        Box::new(DotTunnel::new()),    // ,
        Box::new(Rubber::new()),       // .
        Box::new(Julia::new()),        // A
        Box::new(Raycaster::new()),    // R
        Box::new(Ripples::new()),      // X
        Box::new(RegionFire::new()),   // Z
        Box::new(Metaballs::new()),    // Metaballs
        Box::new(GravityBalls::new()), // Gravity Balls
        Box::new(LavaRegions::new()),  // Lava Regions
        Box::new(Vortex::new()),       // Vortex
    ];
    // Test pattern shown for any unassigned slot
    let mut test_pattern = TestPattern::new();
    // None = test pattern, Some(idx) = effects[idx]
    // Clamp start_effect to valid range
    let mut current_effect: Option<usize> = start_effect.map(|e: usize| e.min(effects.len() - 1));

    // Calibration mode
    let mut calibration = CalibrationMode::new(scene);
    let mut mode = AppMode::Effect;

    // Cursor auto-hide after 60 seconds of no mouse movement
    let mut last_mouse_move: f32 = 0.0;
    const CURSOR_HIDE_DELAY: f32 = 60.0;

    // Keyboard cursor for calibration mode on rotated displays
    // Arrow keys move cursor, Enter clicks - much simpler than mouse transforms
    let mut cursor_pos: (i32, i32) = (width as i32 / 2, height as i32 / 2);
    let mut cursor_visible = true;
    const CURSOR_STEP: i32 = 5; // Pixels per arrow key press
    const CURSOR_STEP_FAST: i32 = 20; // Pixels when holding shift
    let mut shift_held = false;

    // On rotated displays, hide OS cursor and use keyboard cursor exclusively
    let use_keyboard_cursor = rotation != Rotation::None;
    if use_keyboard_cursor {
        display.hide_cursor();
    }

    // Remote control socket
    let controller = Controller::new().ok();
    if controller.is_some() {
        eprintln!("Control socket: {}", Controller::socket_path());
    }

    // Get effect name for benchmark output
    let effect_name = match current_effect {
        Some(idx) => effects[idx].name().to_string(),
        None => "Test Pattern".to_string(),
    };

    if let Some(duration) = benchmark_seconds {
        println!("=== wallfacer benchmark ===");
        println!("Resolution: {}x{}", width, height);
        println!("Effect: {} (index {})", effect_name, current_effect.unwrap_or(0));
        println!("Duration: {} seconds", duration);
        println!("Running...");
    } else {
        println!("=== wallfacer ===");
        println!("Resolution: {}x{}", width, height);
        if rotation != Rotation::None {
            println!(
                "Rotation: {:?} (window: {}x{})",
                rotation, window_w, window_h
            );
        }
        if vsync {
            println!("VSync: ON (60fps locked). Use --no-vsync for uncapped.");
        } else {
            println!("VSync: OFF (uncapped framerate)");
        }
        println!();
        println!("Available effects (use --effect N or arrow keys):");
        for (i, effect) in effects.iter().enumerate() {
            println!("  {:2} - {}", i, effect.name());
        }
        println!();
        println!("Controls:");
        println!("  Left/Right - Cycle through effects");
        println!("  Tab        - Toggle calibration mode");
        println!("  F          - Toggle FPS display");
        println!("  S          - Save scene");
        println!("  L          - Load scene");
        println!("  Escape     - Quit");
        println!();
        println!("Calibration mode:");
        println!("  Left click        - Select region / start drawing polygon");
        println!("  Shift + drag      - Draw circle (drag to set radius)");
        println!("  Click + drag      - Move vertices / resize circle");
        println!("  Close polygon     - Click near first vertex");
        println!("  Right click       - Cancel / deselect");
        println!("  Delete            - Delete selected region");
    }

    'main: loop {
        // Delta time and FPS measurement
        let (dt, _current_fps, avg_fps) = fps_counter.tick();
        total_elapsed += dt;

        // Check benchmark completion
        if let Some(duration) = benchmark_seconds {
            if total_elapsed >= duration {
                // Print benchmark results
                let frame_count = fps_counter.frame_count();
                let (min_fps, max_fps) = fps_counter.min_max_fps();
                let avg_ms = fps_counter.avg_frame_time_ms();
                let std_dev_ms = fps_counter.std_dev_ms();
                let (p1_ms, p50_ms, p99_ms) = fps_counter.percentiles_ms();
                let avg_fps_final = if avg_ms > 0.0 { 1000.0 / avg_ms } else { 0.0 };

                println!();
                println!("=== Benchmark Results ===");
                println!();
                println!("Configuration:");
                println!("  Resolution:     {}x{}", width, height);
                println!("  Effect:         {} (index {})", effect_name, current_effect.unwrap_or(0));
                println!("  Duration:       {:.2}s (requested {}s)", total_elapsed, duration);
                println!();
                println!("Frame Statistics:");
                println!("  Total frames:   {}", frame_count);
                println!("  Average FPS:    {:.1}", avg_fps_final);
                println!("  Min FPS:        {:.1}", min_fps);
                println!("  Max FPS:        {:.1}", max_fps);
                println!();
                println!("Frame Time (ms):");
                println!("  Average:        {:.2}", avg_ms);
                println!("  Std deviation:  {:.2}", std_dev_ms);
                println!("  1st percentile: {:.2} (fastest 1%)", p1_ms);
                println!("  Median (p50):   {:.2}", p50_ms);
                println!("  99th percentile:{:.2} (slowest 1%)", p99_ms);

                break 'main;
            }
        }

        // Handle input - minimal handling in benchmark mode
        for event in display.poll_events() {
            // Always allow quit
            if matches!(&event, InputEvent::Quit) {
                break 'main;
            }

            if let InputEvent::KeyDown(key) = &event {
                // Escape always quits
                if *key == Keycode::Escape {
                    break 'main;
                }

                // Skip interactive controls during benchmark
                if benchmark_seconds.is_some() {
                    continue;
                }

                match *key {
                    Keycode::Tab => {
                        mode = if mode == AppMode::Effect {
                            AppMode::Calibration
                        } else {
                            AppMode::Effect
                        };
                        continue;
                    },
                    Keycode::S => {
                        if let Err(e) = calibration.scene().save("scene.json") {
                            eprintln!("Failed to save: {}", e);
                        } else {
                            println!("Scene saved to scene.json");
                        }
                        continue;
                    },
                    Keycode::L => {
                        match Scene::load("scene.json") {
                            Ok(scene) => {
                                calibration = CalibrationMode::new(scene);
                                println!("Scene loaded from scene.json");
                            },
                            Err(e) => eprintln!("Failed to load: {}", e),
                        }
                        continue;
                    },
                    Keycode::F => {
                        show_fps = !show_fps;
                        continue;
                    },
                    Keycode::Delete | Keycode::Backspace => {
                        if mode == AppMode::Calibration {
                            calibration.delete_selected();
                        }
                        continue;
                    },
                    Keycode::LShift | Keycode::RShift => {
                        shift_held = true;
                        continue;
                    },
                    Keycode::Left => {
                        if mode == AppMode::Calibration && use_keyboard_cursor {
                            let step = if shift_held { CURSOR_STEP_FAST } else { CURSOR_STEP };
                            cursor_pos.0 = (cursor_pos.0 - step).max(0);
                            // Send mouse move event to calibration
                            let evt = InputEvent::MouseMove { x: cursor_pos.0, y: cursor_pos.1 };
                            calibration.handle_event(&evt);
                        } else {
                            current_effect = match current_effect {
                                Some(0) => None,
                                Some(idx) => Some(idx - 1),
                                None => Some(effects.len() - 1),
                            };
                        }
                        continue;
                    },
                    Keycode::Right => {
                        if mode == AppMode::Calibration && use_keyboard_cursor {
                            let step = if shift_held { CURSOR_STEP_FAST } else { CURSOR_STEP };
                            cursor_pos.0 = (cursor_pos.0 + step).min(width as i32 - 1);
                            let evt = InputEvent::MouseMove { x: cursor_pos.0, y: cursor_pos.1 };
                            calibration.handle_event(&evt);
                        } else {
                            current_effect = match current_effect {
                                Some(idx) if idx + 1 >= effects.len() => None,
                                Some(idx) => Some(idx + 1),
                                None => Some(0),
                            };
                        }
                        continue;
                    },
                    Keycode::Up => {
                        if mode == AppMode::Calibration && use_keyboard_cursor {
                            let step = if shift_held { CURSOR_STEP_FAST } else { CURSOR_STEP };
                            cursor_pos.1 = (cursor_pos.1 - step).max(0);
                            let evt = InputEvent::MouseMove { x: cursor_pos.0, y: cursor_pos.1 };
                            calibration.handle_event(&evt);
                        }
                        continue;
                    },
                    Keycode::Down => {
                        if mode == AppMode::Calibration && use_keyboard_cursor {
                            let step = if shift_held { CURSOR_STEP_FAST } else { CURSOR_STEP };
                            cursor_pos.1 = (cursor_pos.1 + step).min(height as i32 - 1);
                            let evt = InputEvent::MouseMove { x: cursor_pos.0, y: cursor_pos.1 };
                            calibration.handle_event(&evt);
                        }
                        continue;
                    },
                    Keycode::Return | Keycode::KpEnter => {
                        if mode == AppMode::Calibration && use_keyboard_cursor {
                            // Simulate left click at cursor position
                            use display::MouseButtonKind;
                            let down = InputEvent::MouseDown { x: cursor_pos.0, y: cursor_pos.1, button: MouseButtonKind::Left };
                            let up = InputEvent::MouseUp { x: cursor_pos.0, y: cursor_pos.1, button: MouseButtonKind::Left };
                            calibration.handle_event(&down);
                            calibration.handle_event(&up);
                        }
                        continue;
                    },
                    _ => {},
                }
            }

            // Track shift key release
            if let InputEvent::KeyUp(key) = &event {
                if *key == Keycode::LShift || *key == Keycode::RShift {
                    shift_held = false;
                }
            }

            // Track mouse movement for cursor auto-hide (non-rotated mode only)
            if !use_keyboard_cursor {
                if matches!(&event, InputEvent::MouseMove { .. } | InputEvent::MouseDown { .. }) {
                    last_mouse_move = total_elapsed;
                    cursor_visible = true;
                    if !display.is_cursor_visible() {
                        display.show_cursor();
                    }
                }

                // Pass mouse events to calibration mode (non-rotated only, rotated uses keyboard)
                if mode == AppMode::Calibration && benchmark_seconds.is_none() {
                    calibration.handle_event(&event);
                }
            }
        }

        // Auto-hide cursor after 60 seconds of no mouse activity
        if total_elapsed - last_mouse_move > CURSOR_HIDE_DELAY {
            cursor_visible = false;
            if !use_keyboard_cursor && display.is_cursor_visible() {
                display.hide_cursor();
            }
        }

        // Process remote control commands
        if let Some(ref ctrl) = controller {
            for cmd in ctrl.poll() {
                match cmd {
                    Command::Left => {
                        current_effect = match current_effect {
                            Some(0) => None,
                            Some(idx) => Some(idx - 1),
                            None => Some(effects.len() - 1),
                        };
                    }
                    Command::Right => {
                        current_effect = match current_effect {
                            Some(idx) if idx + 1 >= effects.len() => None,
                            Some(idx) => Some(idx + 1),
                            None => Some(0),
                        };
                    }
                    Command::Tab => {
                        mode = if mode == AppMode::Effect {
                            AppMode::Calibration
                        } else {
                            AppMode::Effect
                        };
                    }
                    Command::ToggleFps => {
                        show_fps = !show_fps;
                    }
                    Command::Save => {
                        if let Err(e) = calibration.scene().save("scene.json") {
                            eprintln!("Failed to save: {}", e);
                        } else {
                            eprintln!("Scene saved to scene.json");
                        }
                    }
                    Command::Load => {
                        match Scene::load("scene.json") {
                            Ok(scene) => {
                                calibration = CalibrationMode::new(scene);
                                eprintln!("Scene loaded from scene.json");
                            }
                            Err(e) => eprintln!("Failed to load: {}", e),
                        }
                    }
                    Command::Quit => {
                        break 'main;
                    }
                    Command::Effect(n) => {
                        if n < effects.len() {
                            current_effect = Some(n);
                        }
                    }
                }
            }
        }

        // Update and render current effect (or test pattern for unassigned slots)
        // Pause animation updates when in calibration mode
        let region_color = match current_effect {
            Some(idx) => {
                if mode == AppMode::Effect {
                    effects[idx].update(dt, width, height, calibration.scene());
                }
                effects[idx].render(&mut buffer);
                effects[idx].region_color()
            },
            None => {
                if mode == AppMode::Effect {
                    test_pattern.update(dt, width, height, calibration.scene());
                }
                test_pattern.render(&mut buffer);
                test_pattern.region_color()
            },
        };

        // Mask regions with the effect's custom color
        mask_regions(&mut buffer, calibration.scene(), region_color);

        if mode == AppMode::Calibration {
            // Dim the effect a bit more for visibility
            let pixels = buffer.as_bytes_mut();
            for chunk in pixels.chunks_exact_mut(4) {
                chunk[0] /= 2;
                chunk[1] /= 2;
                chunk[2] /= 2;
            }
            // Overlay calibration UI
            calibration.render(&mut buffer);
        }

        // Draw keyboard cursor when in calibration mode with rotation
        if use_keyboard_cursor && mode == AppMode::Calibration && cursor_visible {
            draw_software_cursor(&mut buffer, cursor_pos.0, cursor_pos.1);
        }

        // FPS overlay (press F to toggle)
        if show_fps {
            let (min_fps, max_fps) = fps_counter.min_max_fps();
            let ms = fps_counter.avg_frame_time_ms();
            let fps_text = format!(
                "FPS {} avg  {} min  {} max  {}ms",
                avg_fps as u32, min_fps as u32, max_fps as u32, ms as u32
            );
            // Draw at bottom of screen with shadow for visibility
            let y = buffer.height() as i32 - 12;
            draw_text(&mut buffer, 5, y + 1, &fps_text, 0, 0, 0);
            draw_text(&mut buffer, 4, y, &fps_text, 255, 255, 0);
        }

        // Apply rotation and present
        match rotation {
            Rotation::None => {
                display.present(&mut target, &buffer)?;
            }
            Rotation::Cw90 => {
                let rotated = buffer.rotated_90();
                display.present(&mut target, &rotated)?;
            }
            Rotation::Cw180 => {
                let rotated = buffer.rotated_180();
                display.present(&mut target, &rotated)?;
            }
            Rotation::Cw270 => {
                let rotated = buffer.rotated_270();
                display.present(&mut target, &rotated)?;
            }
        }
    }

    Ok(())
}
