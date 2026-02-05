# Audio-Reactive Effects

Sound-driven visuals (requires audio input integration).

## Spectrum Analyzer

Classic frequency visualization.

**How it works:**
- FFT on audio input
- Map frequency bins to visual bars
- Animate bar heights based on amplitude

**Region interaction:** Each region displays different frequency band:
- Region 1: Sub bass (20-60 Hz)
- Region 2: Bass (60-250 Hz)
- Region 3: Mids (250-2000 Hz)
- Region 4: Highs (2000-20000 Hz)

## Beat Detection

Flash/pulse effects on drum hits.

**How it works:**
- Detect transients in specific frequency bands
- Trigger visual events on detection
- Decay over time

**Region triggers:**
- Kick drum → flash region A
- Snare → flash region B
- Hi-hat → flash region C

## Waveform Display

Oscilloscope-style audio visualization.

**Variations:**
- Classic oscilloscope
- Circular/radial waveform
- Lissajous patterns (stereo L/R as X/Y)

## Volume-Driven Effects

Simple amplitude modulation.

**Uses:**
- Brightness modulation
- Scale/size pulsing
- Color intensity
- Effect speed

## Frequency-Mapped Colors

Map frequency content to color palette.

**How it works:**
- Low frequencies → warm colors (red, orange)
- High frequencies → cool colors (blue, purple)
- Creates synesthetic color response

## Audio-Reactive Particles

Particles spawned/affected by sound.

**Variations:**
- Spawn rate tied to amplitude
- Particle velocity tied to frequency
- Explosion on beat detection

## VU Meter Style

Classic analog meter aesthetics.

**Elements:**
- Needle movement
- Peak hold indicators
- LED-style segmented display

---

## Implementation Notes

**Audio input options:**
- System audio loopback
- Microphone input
- MIDI control signals
- OSC protocol from external software

**Latency considerations:**
- Visual response should feel instant
- Buffer sizes affect perceived sync
- Pre-delay audio if needed
