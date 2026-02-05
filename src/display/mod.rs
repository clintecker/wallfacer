mod font;
mod pixel_buffer;
mod scroller;
pub mod text_fx;

#[allow(unused_imports)]
pub use font::{
    draw_char_scaled, draw_text, draw_text_boxed, draw_text_centered, draw_text_centered_scaled,
    draw_text_scaled, text_width, text_width_scaled, GLYPH_HEIGHT, GLYPH_WIDTH,
};
#[allow(unused_imports)]
pub use pixel_buffer::{BlendMode, PixelBuffer};
#[allow(unused_imports)]
pub use scroller::{
    ColorEffect, LayerEffect, OffsetEffect, ScrollDirection, ScrollMode, Scroller, SineScroller,
    StyledScroller, Typewriter, VisibilityEffect,
};

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::mouse::MouseButton;
use sdl2::pixels::PixelFormatEnum;
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::video::{Window, WindowContext};
use sdl2::EventPump;

pub const DEFAULT_WIDTH: u32 = 640;
pub const DEFAULT_HEIGHT: u32 = 480;

pub struct Display {
    canvas: Canvas<Window>,
    event_pump: EventPump,
    width: u32,
    height: u32,
}

pub struct RenderTarget<'a> {
    texture: Texture<'a>,
    width: u32,
    height: u32,
}

#[derive(Debug, Clone)]
pub enum InputEvent {
    Quit,
    KeyDown(Keycode),
    KeyUp(Keycode),
    MouseMove {
        x: i32,
        y: i32,
    },
    MouseDown {
        x: i32,
        y: i32,
        button: MouseButtonKind,
    },
    MouseUp {
        x: i32,
        y: i32,
        button: MouseButtonKind,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButtonKind {
    Left,
    Right,
    Middle,
}

impl Display {
    /// Create display with VSync enabled (default, 60fps locked)
    pub fn new(title: &str) -> Result<(Self, TextureCreator<WindowContext>), String> {
        Self::with_options(title, DEFAULT_WIDTH, DEFAULT_HEIGHT, true)
    }

    /// Create display with configurable VSync
    /// vsync=true: locked to monitor refresh (typically 60fps)
    /// vsync=false: uncapped framerate for performance testing
    pub fn with_vsync(
        title: &str,
        vsync: bool,
    ) -> Result<(Self, TextureCreator<WindowContext>), String> {
        Self::with_options(title, DEFAULT_WIDTH, DEFAULT_HEIGHT, vsync)
    }

    /// Create display with custom resolution and VSync settings
    pub fn with_options(
        title: &str,
        width: u32,
        height: u32,
        vsync: bool,
    ) -> Result<(Self, TextureCreator<WindowContext>), String> {
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;

        let window = video_subsystem
            .window(title, width, height)
            .position_centered()
            .build()
            .map_err(|e| e.to_string())?;

        let mut canvas_builder = window.into_canvas().accelerated();
        if vsync {
            canvas_builder = canvas_builder.present_vsync();
        }
        let canvas = canvas_builder.build().map_err(|e| e.to_string())?;

        let texture_creator = canvas.texture_creator();
        let event_pump = sdl_context.event_pump()?;

        Ok((
            Self {
                canvas,
                event_pump,
                width,
                height,
            },
            texture_creator,
        ))
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn present(
        &mut self,
        target: &mut RenderTarget,
        buffer: &PixelBuffer,
    ) -> Result<(), String> {
        target
            .texture
            .update(None, buffer.as_bytes(), (buffer.width() * 4) as usize)
            .map_err(|e| e.to_string())?;

        self.canvas.copy(&target.texture, None, None)?;
        self.canvas.present();
        Ok(())
    }

    pub fn poll_events(&mut self) -> Vec<InputEvent> {
        let mut events = Vec::new();

        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => events.push(InputEvent::Quit),
                Event::KeyDown {
                    keycode: Some(k), ..
                } => events.push(InputEvent::KeyDown(k)),
                Event::KeyUp {
                    keycode: Some(k), ..
                } => events.push(InputEvent::KeyUp(k)),
                Event::MouseMotion { x, y, .. } => events.push(InputEvent::MouseMove { x, y }),
                Event::MouseButtonDown {
                    x, y, mouse_btn, ..
                } => {
                    if let Some(button) = map_mouse_button(mouse_btn) {
                        events.push(InputEvent::MouseDown { x, y, button });
                    }
                },
                Event::MouseButtonUp {
                    x, y, mouse_btn, ..
                } => {
                    if let Some(button) = map_mouse_button(mouse_btn) {
                        events.push(InputEvent::MouseUp { x, y, button });
                    }
                },
                _ => {},
            }
        }

        events
    }
}

impl<'a> RenderTarget<'a> {
    /// Create render target with default resolution
    pub fn new(texture_creator: &'a TextureCreator<WindowContext>) -> Result<Self, String> {
        Self::with_size(texture_creator, DEFAULT_WIDTH, DEFAULT_HEIGHT)
    }

    /// Create render target with custom resolution
    pub fn with_size(
        texture_creator: &'a TextureCreator<WindowContext>,
        width: u32,
        height: u32,
    ) -> Result<Self, String> {
        let texture = texture_creator
            .create_texture_streaming(PixelFormatEnum::RGBA8888, width, height)
            .map_err(|e| e.to_string())?;
        Ok(Self {
            texture,
            width,
            height,
        })
    }
}

fn map_mouse_button(btn: MouseButton) -> Option<MouseButtonKind> {
    match btn {
        MouseButton::Left => Some(MouseButtonKind::Left),
        MouseButton::Right => Some(MouseButtonKind::Right),
        MouseButton::Middle => Some(MouseButtonKind::Middle),
        _ => None,
    }
}
