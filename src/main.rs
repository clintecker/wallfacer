// Allow unused code for designed-but-not-yet-used APIs
// Remove these as the codebase matures
#![allow(dead_code)]

mod display;
mod effects;
mod geometry;
mod input;
mod math3d;
mod particles;
mod regions;
mod texture;
mod util;

use display::{
    draw_text, Display, InputEvent, PixelBuffer, RenderTarget, DEFAULT_HEIGHT, DEFAULT_WIDTH,
};
use effects::{
    Bobs, CopperBars, Dvd, Earth, Earth2, Effect, Fire, Glenz, Plasma, Rotozoomer, ScrollerDemo,
    Starfield, TestPattern, TextFxDemo, Tunnel, Worms,
};
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
    for region in &scene.regions {
        buffer.fill_polygon(&region.polygon.as_tuples(), color.0, color.1, color.2);
    }
}

/// Parse command line arguments and return (width, height, vsync)
fn parse_args() -> (u32, u32, bool) {
    let args: Vec<String> = std::env::args().collect();
    let mut width = DEFAULT_WIDTH;
    let mut height = DEFAULT_HEIGHT;
    let mut vsync = true;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--no-vsync" => vsync = false,
            "--width" | "-w" => {
                if i + 1 < args.len() {
                    if let Ok(w) = args[i + 1].parse::<u32>() {
                        width = w;
                    }
                    i += 1;
                }
            },
            "--height" | "-h" => {
                if i + 1 < args.len() {
                    if let Ok(h) = args[i + 1].parse::<u32>() {
                        height = h;
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
                            width = w;
                            height = h;
                        }
                    }
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
                println!("  --no-vsync            Disable VSync for uncapped framerate");
                println!("  --help                Show this help message");
                std::process::exit(0);
            },
            _ => {},
        }
        i += 1;
    }

    (width, height, vsync)
}

fn main() -> Result<(), String> {
    let (width, height, vsync) = parse_args();

    let (mut display, texture_creator) = Display::with_options("wallfacer", width, height, vsync)?;
    let mut target = RenderTarget::with_size(&texture_creator, width, height)?;
    let mut buffer = PixelBuffer::with_size(width, height);

    // FPS counter with 60 sample rolling average
    let mut fps_counter = FpsCounter::new(60);
    let mut show_fps = false;

    // Load scene or create new
    let scene = Scene::load("scene.json").unwrap_or_else(|_| Scene::new("default"));

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
    ];
    // Test pattern shown for any unassigned slot
    let mut test_pattern = TestPattern::new();
    // None = test pattern, Some(idx) = effects[idx]
    let mut current_effect: Option<usize> = Some(0);

    // Calibration mode
    let mut calibration = CalibrationMode::new(scene);
    let mut mode = AppMode::Effect;

    println!("=== wallfacer ===");
    println!("Resolution: {}x{}", width, height);
    if vsync {
        println!("VSync: ON (60fps locked). Use --no-vsync for uncapped.");
    } else {
        println!("VSync: OFF (uncapped framerate)");
    }
    println!("Use --help for command line options.");
    println!("Controls:");
    println!("  Tab        - Toggle calibration mode");
    println!("  Left/Right - Cycle through effects");
    println!("  1          - Plasma");
    println!("  2          - Starfield");
    println!("  3          - Fire");
    println!("  4          - Scroller Demo");
    println!("  5          - Text FX Demo");
    println!("  6          - Worms");
    println!("  7          - DVD Bounce");
    println!("  8          - Copper Bars");
    println!("  9          - Glenz Vectors");
    println!("  0          - Rotozoomer");
    println!("  -          - Tunnel");
    println!("  =          - Bobs");
    println!("  [          - Earth");
    println!("  ]          - Earth 2.0");
    println!("  Backspace  - Test Pattern");
    println!("  F          - Toggle FPS display");
    println!("  S          - Save scene");
    println!("  L          - Load scene");
    println!("  Escape     - Quit");
    println!();
    println!("Calibration mode:");
    println!("  Left click        - Select region / start drawing");
    println!("  Click + drag      - Move vertices");
    println!("  Click empty space - Start new polygon");
    println!("  Close polygon     - Click near first vertex");
    println!("  Right click       - Cancel / deselect");
    println!("  Delete            - Delete selected region");

    'main: loop {
        // Delta time and FPS measurement
        let (dt, _current_fps, avg_fps) = fps_counter.tick();

        // Handle input
        for event in display.poll_events() {
            // Handle global keys
            if let InputEvent::KeyDown(key) = &event {
                match *key {
                    Keycode::Escape => break 'main,
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
                    Keycode::Delete => {
                        if mode == AppMode::Calibration {
                            calibration.delete_selected();
                        }
                        continue;
                    },
                    // Number keys: 1-5 select effects, 6-0 show test pattern
                    Keycode::Num1 => {
                        current_effect = Some(0);
                        continue;
                    },
                    Keycode::Num2 => {
                        current_effect = Some(1);
                        continue;
                    },
                    Keycode::Num3 => {
                        current_effect = Some(2);
                        continue;
                    },
                    Keycode::Num4 => {
                        current_effect = Some(3);
                        continue;
                    },
                    Keycode::Num5 => {
                        current_effect = Some(4);
                        continue;
                    },
                    Keycode::Num6 => {
                        current_effect = Some(5);
                        continue;
                    },
                    Keycode::Num7 => {
                        current_effect = Some(6);
                        continue;
                    },
                    Keycode::Num8 => {
                        current_effect = Some(7); // Copper Bars
                        continue;
                    },
                    Keycode::Num9 => {
                        current_effect = Some(8); // Glenz
                        continue;
                    },
                    Keycode::Num0 => {
                        current_effect = Some(9); // Rotozoomer
                        continue;
                    },
                    Keycode::Minus => {
                        current_effect = Some(10); // Tunnel
                        continue;
                    },
                    Keycode::Equals => {
                        current_effect = Some(11); // Bobs
                        continue;
                    },
                    Keycode::LeftBracket => {
                        current_effect = Some(12); // Earth
                        continue;
                    },
                    Keycode::RightBracket => {
                        current_effect = Some(13); // Earth 2.0
                        continue;
                    },
                    Keycode::Backspace => {
                        if mode == AppMode::Calibration {
                            calibration.delete_selected();
                        } else {
                            current_effect = None; // Test pattern
                        }
                        continue;
                    },
                    Keycode::Left => {
                        current_effect = match current_effect {
                            Some(0) => None,
                            Some(idx) => Some(idx - 1),
                            None => Some(effects.len() - 1),
                        };
                        continue;
                    },
                    Keycode::Right => {
                        current_effect = match current_effect {
                            Some(idx) if idx + 1 >= effects.len() => None,
                            Some(idx) => Some(idx + 1),
                            None => Some(0),
                        };
                        continue;
                    },
                    _ => {},
                }
            }

            if matches!(&event, InputEvent::Quit) {
                break 'main;
            }

            // Pass all events to calibration mode (mouse move, down, up)
            if mode == AppMode::Calibration {
                calibration.handle_event(&event);
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

        // Present
        display.present(&mut target, &buffer)?;
    }

    Ok(())
}
