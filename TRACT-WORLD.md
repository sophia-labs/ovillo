# The public tract on the true World (`ovillo-tract-world`)

Vera, 2026-10-20: "True World": the public tract (the site page, the
Shrubbery component, the Android TV app) runs on `ovillo-tract-world`, a
`mus_ariadne::World`, not on `ovillo-tract`. This branch makes it so. The
engine is `TractNdAnatomy::public_v1()`; the page is `site/tract/`, whose
`tract.wasm` is now built from `ovillo-tract-world` (`site/tract/build.sh`).

## Same ABI, swap by `.wasm`

`ovillo-tract-world/src/abi.rs` exports exactly `ovillo-tract-wasm`'s
functions (`tract_init`, `tract_init_decimated`, `tract_out_ptr`,
`tract_state_ptr`, `tract_state_len`, `tract_process`, `tract_set`,
`tract_get`, `tract_preset`, `tract_constrict`, `tract_vibrato`,
`tract_gesture`, `tract_impact`, `tract_snapshot`), with the same arguments,
returns and the same 264-float state layout (books, DSP, areas, pressure RMS,
fold cycle, constrictions, narrowest aperture, decimate). `tract.js` and
`worklet.js` are unchanged except comments and one label. The tube tract's own
ABI test passes unchanged against this engine (`abi::tests`). Extra exports
(`ndt_*`, timing only) do not collide.

## What was ported, and how it is lawful here

| `ovillo-tract` fe4fb03 | World tract |
|---|---|
| touch constrictions anywhere (slots 1-4), gesture slot 0 | five slots as ten more configuration coordinates (position, degree) through a new `AreaTermSpec::Constriction` (`A *= 1 - degree bump`, flat top 0.6 section, shoulder 0.022: their numbers); every move is a World move with its receipt; contact at degree 0.98 |
| exact closures, lip seal (lips < 0.02), velum seal | `ContactSpec { follow_below_floor }`: a column whose area falls below the cone's 0.025 cm² floor follows it down to a numerical seal, continuous with the cone above it (open acoustics unchanged); the lip seal is a constriction driven by `lips` (closed at 0.004); the port seals at velum 0 |
| jets, fricatives | POC-3's jet at the narrowest column (free-jet loss) and its turbulent dipole (0.4% at Mach 0.1, x M³; Re 3000-6000: their constants), DC-correct losses |
| plosives (pa, ta, ka, papapa) | closure as configuration, pressure from the lungs, release |
| vibrato, wobble | their `tension_modulation`, applied as booked valve retunes |
| gestures mama, aiua, sss, shh, fff, zzz, vvv, pa, ta, ka, papapa | their key tables, copied; vowels from this World's own fit |
| orbs (`tract_impact`) | a raised-cosine volume-velocity pulse into the struck volume, exact work booked as source work |
| presets /a/ /i/ /u/ | a fit in this World (`examples/fit_public_presets.rs`, `receipts/public-preset-fit.json`) from their sealed-velum fit; /u/ needs velum 0.01 (theirs 0.015) |
| light mode (`tract_init_decimated(_, 2)`) | the same World at the full rate (its 24 modes need it), moved every 128 samples, skipping moves below 0.004 (full: 32, 0.001) |

## Gates

- **Ledger across consonant gestures** (`tests/public.rs`): all eleven
  gestures: max frame residual `8.9e-16 J`, cumulative `<= 1.5e-13 J`; the
  five fricatives hiss (turbulence injected 0.9e-4 to 5.1e-4 J per 1.55 s gesture);
  pa, ta, ka, papapa seal exactly (608 to 5,248 closed samples); random
  touches anywhere (30% full closures) stay bounded and release; a
  1-semitone vibrato moves F0 129 to 141 Hz over 1,500 booked retunes.
- **Formants** (public engine, same procedure as CHECKPOINT-2A; World's own
  modal response / render LPC, Hz): /a/ 710/1150/2503 / 730/1146/2522;
  /i/ 290/2248/2958 / 320/2274/2962; /u/ 312/699/2382 / 333/700/2250
  (targets 710/1150/2500, 290/2250/2950, 310/700/2350).
- **wasm**, through the shared ABI (node 22 / V8, the same harness for both
  engines, `examples/abi_bench.mjs`): constructing the World takes 9-28 ms
  (the planner's band check is now two Sylvester inertia counts per sample
  point). Worklet DSP load in headless Chrome 155 at load average about 7,
  same page, same gestures (idle / papapa / sss / touch closure):

  | | tube tract (fe4fb03) | World tract |
  |---|---|---|
  | full mode | 9.4 / 63 / 60 / 57% | 4.2 / 19 / 6.8 / 6.0% |
  | light mode (`lite=1`) | n/a / 21 / 20 / 27% | 6.8 / 9.8 / 6.4 / 5.2% |

  The page's own probe (wall RTF on its mama loop): 6.6x desktop, 4.5x in TV
  mode at 960x540 dpr 2 (it then picks light mode, under its threshold of 5).
- **Headless Chrome** (`--headless=new --mute-audio`, DevTools protocol):
  `site/tract/` and `?tv=1` at 960x540 dpr 2 load, start, run papapa, sss
  and a touch closure; no console message, no exception; books `9e-16 J`;
  the narrowest aperture reads 0 under a closure; F0 reads 0 when voiceless.

## The light schedule's spectral cost

Against moving every sample, on the public engine's own mama gesture
(`examples/zipper.rs --public`): full schedule long-term spectrum mean
|dB| 1.12, energy above 6 kHz +8.1 dB, F1/F2 tracks within 29/38 Hz; light
schedule 1.51, +10.0 dB, 37/39 Hz. Light mode costs 0.4 dB of spectrum, 1.9 dB
above 6 kHz and about 7 Hz of F1 track during fast gestures; still vowels are
identical.

## The TV, estimated, not measured

wasm wall RTF on a quiet core (load average 5; `examples/abi_bench.mjs`,
the same ABI calls for both engines, 6 s each, final builds):

| | full: held /a/ / mama / papapa / sss | light: held / mama / papapa / sss |
|---|---|---|
| World tract | 23.7 / 10.1 / 7.5 / 18.6x | 24.1 / 17.4 / 14.3 / 13.0x |
| tube tract (fe4fb03) | 1.7 / 6.8 / 1.2 / 2.0x | 5.9 / 16.1 / 4.8 / 7.5x |

The Realtek RTD288O ran the tube tract at about 82% worklet load (in light
mode: its probe threshold on TV is 5x). In light mode the World needs 0.25x
(held vowel), 0.93x (mama), 0.34x (papapa) and 0.58x (sss) of the tube's
time here, so on the TV expect about 20% to 76% for the same workload, if
the ratios carry over to 32-bit ARM. Not run on the device.

## Gaps

- No glottal aspiration noise (`tract_set(7, _)` is accepted and does
  nothing); the tube tract has it. A breathy voice and the aspiration after a
  stop's release are therefore missing.
- One jet (the narrowest column), not one per narrow junction; an orb does
  not dent the wall; the stop gestures, copied from the tube tract, hold
  `pa`'s closure only about 13 ms because this World's lips travel at the
  articulator speed.
- The turbulent source's net work includes a zero-mean cross term with the
  flow already in its column (the ledger books the net; the readout shows
  the injected part).
- The approximations of CHECKPOINT-2A and POC-3 (mode-diagonal losses,
  stepped moves, numerical seal) apply.
- `tract.wasm` is 630 KB (the tube's is 134 KB), not run through wasm-opt.
