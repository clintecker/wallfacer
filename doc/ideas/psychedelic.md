# Fractals & Psychedelic Effects

Mind-bending visuals inspired by 90s rave culture and mathematical beauty.

## Fractal Zoom

Infinite zoom into Mandelbrot or Julia sets.

**How it works:** For each pixel, iterate `z = z² + c` until escape or max iterations. Color by iteration count.

**Variations:**
- Mandelbrot set (c = pixel coordinate)
- Julia sets (c = constant, z₀ = pixel)
- Burning ship fractal
- Animated Julia (vary c over time)

**Region interaction:** Each region zooms into different fractal coordinates.

## Kaleidoscope

Symmetric patterns through reflection and rotation.

**How it works:** Map screen coordinates through angular symmetry. Source image is reflected/rotated to fill all segments.

**Variations:**
- 4-fold, 6-fold, 8-fold symmetry
- Animated source (plasma, video)
- Multiple kaleidoscope centers

**Region interaction:** Each region becomes its own kaleidoscope with independent source.

## Color Cycling

Palette animation creating motion without moving pixels.

**How it works:** Rotate color palette indices each frame. Static pixel values appear to animate as their colors shift.

**Classic 90s VJ technique:** Fractal images with cycling palettes were iconic rave visuals.

## Interference Patterns

Multiple wave sources creating complex patterns.

**How it works:** Sum sine waves emanating from multiple points. Interference creates organic, flowing patterns.

**Region interaction:** Each region centroid is a wave source, patterns interact across regions.

## Hypnotic Spirals

Rotating spiral patterns that appear to move inward/outward.

**How it works:** For each pixel, calculate angle and distance from center. Spiral = `brightness = sin(angle + distance × twist + time)`.

**Variations:**
- Single spiral
- Multi-arm spirals
- Contracting vs expanding

## Liquid Light Show

Simulating the analog overhead projector effects from 60s/70s.

**Techniques:**
- Layered colored blobs
- Oil-on-water simulation
- Slow morphing organic shapes
- High color saturation

**Historical context:** Pre-digital VJs used overhead projectors with colored oils, creating organic flowing visuals. Early rave VJs like The Light Surgeons combined these with video/slides.

## Reaction-Diffusion

Turing patterns and organic texture generation.

**How it works:** Simulate two chemicals diffusing and reacting. Creates spots, stripes, and organic patterns.

**Examples:**
- Gray-Scott model
- Belousov-Zhabotinsky patterns
- Coral/leopard spot textures

---

## 90s Rave VJ Context

The birth of electronic dance music coincided with early computer graphics. Key elements:

- **Fractal art** became iconic - vivid colors, kaleidoscopic patterns, psychedelic feel
- **Real-time generation** was the demoscene ethos - no pre-rendered video
- **The Light Surgeons**, **Eikon** (UK) combined liquid lightshows with video
- **Dimension 7** brought projectors to Burning Man (1996-98)

Tools of the era:
- NewTek Video Toaster (Amiga)
- Panasonic WJ-MX50 video mixer
- Custom demoscene software
