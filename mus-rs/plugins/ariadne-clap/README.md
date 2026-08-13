# Ariadne 0.1 — Powered Body

Ariadne is a stereo guitar effect built from Sophia's Rust DSP. It is not an
amp or impulse-response model. Three powered, asymmetric contact stages feed a
broadband radiator and an eight-mode body; a finite energy reservoir produces
sag and bloom. Four persistent Telaraña fields continue that same instrument
as reverb.

This release contains universal macOS VST3 and CLAP plug-ins for Apple Silicon
and Intel. It is intended for live guitar input in Ableton Live, REAPER, and
other hosts that support either format.

## Install on macOS

Quit the DAW, then copy the bundles:

```text
Ariadne.vst3 -> ~/Library/Audio/Plug-Ins/VST3/
Ariadne.clap -> ~/Library/Audio/Plug-Ins/CLAP/
```

Rescan plug-ins in the host. This preview is ad-hoc signed but not Apple
notarized. If macOS quarantines a bundle you trust, remove quarantine from that
specific bundle and rescan:

```sh
xattr -dr com.apple.quarantine ~/Library/Audio/Plug-Ins/VST3/Ariadne.vst3
xattr -dr com.apple.quarantine ~/Library/Audio/Plug-Ins/CLAP/Ariadne.clap
```

Use VST3 in Ableton Live. REAPER can use either VST3 or CLAP; CLAP is the more
direct build.

## Start here

Place Ariadne on a mono or stereo audio track, enable input monitoring, and
choose a `Character` plus a `Telaraña` space:

- `CostaClara + Plate` keeps pick dynamics and adds a centered 24-course tail.
- `Edge + Spring` is the default touch-sensitive breakup and a dispersive,
  three-course spring.
- `Grind + Sympathique` supplies the obvious electric-guitar distortion and an
  eight-course tail that slowly follows the detected key.
- `Wall + ShimmerLite` is the saturated, sustained voice with an attack-driven
  rotating 15-course tail.

`Drive`, `Headroom`, `Bias`, `Tone`, `Bloom`, and `Sag` are the fastest route to
the powered material. `Interlace`, `Chirality`, `Motion`, and `Curvature`
change the body; Motion is input-driven and is not a free-running LFO.
`Space Mix` sends energy into the selected Telaraña field, while `Mix` is the
overall aligned dry/wet control.

Start with the input peaking around -18 to -12 dBFS. The tones deliberately
react to level. They are macros inside the DSP contract, so every continuous
parameter remains automatable after a tone is chosen.

## Runtime contract

- 96 samples of reported and dry-aligned latency at every sample rate (2.00 ms
  at 48 kHz, 0.50 ms at 192 kHz).
- Calibrated at 44.1, 48, 96, and 192 kHz; activation is defensive from 1 to
  768 kHz for host compatibility.
- Four-times local oversampling through 96 kHz with a 385-tap linear-phase FIR;
  native nonlinear processing above 96 kHz.
- No allocation, locks, I/O, or background work on the audio path after host
  initialization.
- Exact silent fixed point, finite-input guards, subnormal retirement, stable
  parameter IDs, state restoration, and sample-accurate automation.
- Conservative 20-second reported tail. The plug-in has no custom GUI yet, so
  the host supplies the parameter view.

The universal binary is approximately 2.4 MB per format and links only macOS
system libraries. It has passed pluginval strictness 10 at 44.1/48/96/192 kHz
over block sizes 1–1024, and the official CLAP validator's complete applicable
suite. Ableton Live and REAPER were not installed on the build machine, so this
preview still needs an actual DAW smoke test.

## Build from source

From `mus-rs/`:

```sh
cargo xtask bundle-universal ariadne-clap --release
cargo test -p mus-dsp
cargo test -p ariadne-clap
cargo clippy -p ariadne-clap -- -D warnings
```

The wrapper owns only host ABI and parameter plumbing. The sound, profile
mappings, latency, stability guards, and receipts live in
`crates/mus-dsp/src/powered_radiator.rs`.

## Preview status

This is a research preview, not a notarized commercial release. The supplied
GuitarJam montage and receipts demonstrate the current four tone/space pairs.
Sympathique deliberately reports low confidence when relative major/minor keys
are ambiguous. Shimmer Lite is the lawful input-driven orbit version; the full
braid-revoiced Shimmer Trenzado remains future work.
