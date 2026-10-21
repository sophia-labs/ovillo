# Checkpoint (2)(a): N-dimensional configuration in `mus_ariadne`, and the vocal tract as a true World

Branch `ariadne/nd-configuration-20261020`, base `f43f372` (site/tract-20261020).
Tag `checkpoint-2a-20261020`. Design: `mus-rs/SPECS/ariadne/ND-CONFIGURATION.md`.
Machine for every timing: Intel i9-9880H (8 cores / 16 threads, 2.3 GHz), x86_64
toolchain, shared with other agents' builds (load averages 90 to 770 during the
runs, recorded beside each number).

## What was built

1. **N-D configuration spaces in `mus-ariadne`** (new constitution `modal_space`
   in plan v3): named coordinates with bounds; three maps from a point to the
   body's modes: `multilinear` (anchor grid; a fit), `stiff_string` (partials
   computed from stopped length and tension; a law), `tube_network` (modes
   computed from a branched tube network's area function; a law); two
   connections: `modal_coordinates` (legacy, flat) and `mass_momentum` (hold
   `C p` and `L u`, re-project, dispose what 24 modes cannot hold; curved).
   Files: `src/configuration/{space,network,eigen,tree_eigen}.rs`,
   `src/space_body.rs`, `src/plan/space.rs`, `src/shared_world/space.rs`.
2. **A flow-valve law** (`src/law/valve.rs`): a reservoir-paid self-oscillating
   fold whose jet enters one network volume with an exact work contract.
3. **`ovillo/crates/ovillo-tract-world`**: the tube tract's anatomy (copied,
   cited) lowered into one `ModalSpace` body; 4 coordinates (tongue index,
   tongue diameter, lips, velum); a 44 + 31 volume oral+nasal network; the
   valve at the glottis; the World's own ledger. Measurement code copied
   byte-identical from `ovillo-tract` (`src/measure.rs`).

## G-A1 backward compatibility: PASS

Base binary built at `f43f372`, branch binary built from the checkpoint tree;
`mus render <score> --json`:

| score | renderDigest (base = branch) | planDigest (base = branch) | WAV bytes |
|---|---|---|---|
| `examples/tab/steel_string_tour.mus` | `sha256:fa3d615fe9dca59ce13c2d7b92c7248ed8f56fe634a33e7734b08f9f5a0ffbfc` | `sha256:9c7feb612c813cac447c2e02e2d0c8660ff0702cd9e492e3369c4cd230661ff7` | identical (`cmp`) |
| `examples/tab/wayfaring-stranger-doc.mus` | `sha256:9759473617875482d0ec86a278c9b85450d166fe1fda02dabefb4fbed8aaa27d` | `sha256:425b82afad27980c5fbe8aab2b9ab9e7f7f23bf8a4abfffd5fd901d5e2c28a6d` | identical (`cmp`) |

`listenerDigest` and every ledger field of both receipts are equal (tour:
cumulative residual `-1.1854564910002835e-16 J`, source work
`1.876279479222248e-07 J`; Wayfaring: `-1.5539310696489547e-15 J`,
`2.5113644799967932e-06 J`), at base and on the branch.

Tests, base vs branch (`cargo test --release ...`):

| target | base `f43f372` | branch |
|---|---|---|
| `-p mus-ariadne` (all targets) | does not compile: stale examples and integration tests (`world_bench`, `metric_radiation_proof`, `constitutive_shared_world`, ...; AGENTS.md: "Known stale") | same errors, nothing new |
| `-p mus-ariadne --lib` | 50 passed | 58 passed (8 new) |
| `--test local_medium_fixture` | 1 failed (pre-existing) | same failure |
| `--test constitutive_shared_world`, `--test metric_radiation_fixture` | do not compile | same |
| `--test nd_configuration` (new) | n/a | 6 passed |
| `-p mus-notation -p mus-engine` | 77 + 3 passed | 77 + 3 passed |
| `-p mus-cli --test ariadne_tab --test check_report` | 2 passed | 2 passed |
| `ovillo`: `-p ovillo-dsp -p ovillo-tract` | 31 + 2 passed; 7 passed | 31 + 2 passed; 7 passed |
| `ovillo`: `-p ovillo-tract-world` (new) | n/a | 6 passed (`tests/laws.rs`) |
| `ovillo-web` (not on this base; run on `site/ovillo-demo-20261020`, whose mus-ariadne equals the base's, then with this branch's mus-ariadne copied in) | 5 passed | 5 passed |

Runtime-level evidence (`tests/nd_configuration.rs`,
`one_dimensional_space_body_is_bitwise_the_legacy_body`): the steel-string
world with `string/6` replaced by its 1-D two-anchor N-D twin renders 60,000
samples through plucks and nine fret moves with bit-identical `WorldFrame`s,
emissions, final states, control work, radiation and cumulative residual.
A 1-D two-anchor multilinear map is bitwise `interpolate_configuration` at
1,001 amounts (`space.rs` unit test).

## G-A2 the ledger closes for N-D moves: PASS

`cargo test --release -p mus-ariadne --test nd_configuration -- --nocapture`:

- 4-D multilinear walk (16 anchors, a random move every 16 samples, 6,000
  moves over 96,000 samples, plucked): cumulative residual `5.6e-17 J`, max
  frame residual `1.3e-18 J`, against stored energy up to `2.0e-3 J`
  (bound used: `1e-13 x` stored per frame).
- 4-D tube-network walk (modes recomputed at all 750 moves, voiced by the
  valve, lungs 3 J): mass-momentum: cumulative `2.3e-14 J`, max frame
  `1.3e-15 J`, disposal booked `6.6e-8 J`; modal-coordinates: cumulative
  `8.9e-15 J`, max frame `1.3e-15 J`. Relative to the 3 J stored, a few ulps.
- Closed loops (5 corners, 100 instantaneous moves): modal-coordinates: net
  work exactly `0.0 J`, state returned bit for bit (flat). Mass-momentum:
  net work `3.236e-9 J`, disposal `3.048e-9 J`, field `6.719696e-6 ->
  6.719884e-6 J` = `+(work - disposal)` exactly, state not returned
  (`|dm| = 1.7e-3`): holonomy, billed. The same loop traversed while voiced
  (200 moves, 6,400 samples): net control work `3.50e-6 J` (modal) /
  `4.65e-6 J` (mass-momentum), cumulative residual stays `~1e-14 J`.

## G-A3 the tract as a true `mus_ariadne::World`: PASS

Plan digest (44 + 31 volumes, mass-momentum, 48 kHz):
`sha256:5617b467134488c2747538b8899abffb7f659356a40eacaeb44f21b62921419c`.
Source: the flow valve (fold numbers copied from `ovillo-tract`
`adult_v0`); it is not the trombone's additive buzz.

Vowels (`cargo run --release -p ovillo-tract-world --example render_vowels -- 44 31 32 mm`,
receipt `ovillo/crates/ovillo-tract-world/receipts/vowels-o44-n31-k32-massmomentum.json`).
Both worlds start at rest, glide to the vowel at articulator speed, breath
0.6, fold tuning 120 Hz, 1.4 s; LPC is `ovillo-tract`'s (copied), median of
three 50 ms frames in the last 0.4 s. Articulations are `ovillo-tract`'s
fitted presets (fit there, against the tube; not refitted here). Targets:
GARGANTA rung-1 design profiles (`mus-dsp/src/body_profiles.rs`, "design
profiles, not measurements").

| vowel | target F1/F2/F3 | N-D World, render LPC | err % | N-D World, own modal response | tube tract, render LPC | err % | tube tract, rigid-glottis peaks | F0 N-D / tube |
|---|---|---|---|---|---|---|---|---|
| /a/ | 710 / 1150 / 2500 | 730 / 1148 / 2471 | +2.8 / -0.2 / -1.1 | 716 / 1158 / 2553 | 731 / 1154 / 2501 | +3.0 / +0.3 / 0.0 | 718 / 1156 / 2547 | 137.1 / 139.1 |
| /i/ | 290 / 2250 / 2950 | 342 / 2305 / 2948 | +17.8 / +2.5 / -0.1 | 293 / 2268 / 2947 | 355 / 2308 / 2948 | +22.4 / +2.6 / -0.1 | 293 / 2268 / 2951 | 137.9 / 137.9 |
| /u/ | 310 / 700 / 2350 | 365 / 717 / 2380 | +17.6 / +2.5 / +1.3 | 304 / 699 / 2413 | 359 / 793 / 2309 | +15.7 / +13.3 / -1.7 | 305 / 702 / 2397 | 134.8 / 134.5 |

Two independent discretizations of the same area function (Kelly-Lochbaum at
96 kHz; computed modes of the lumped network) put F1-F3 within 0.7% of each
other (`tests/laws.rs`, `computed_modes_agree_with_the_tube_tracts_resonances`,
asserts 2.5%). The high LPC F1 of /i/ and /u/ appears in both worlds while
both worlds' own resonances sit at 293-305 Hz; its cause was not established
here. Every vowel's ledger: max frame residual `8.9e-16 J`, cumulative
`<= 2.2e-13 J` (3 J lungs). Coarser grid (32 + 16): /a/ 713/1151/2487,
/i/ 317/2295/2929, /u/ 335/695/2157 (receipt `vowels-o32-n16-...`).

Nasal branch through the velum coordinate (`the_velum_opens_the_nose`):
nose share of radiated energy 5.3% at velum 0 and 16.7% at velum 1 (the tube
tract: 4.7% and 11.0%). Neither closes exactly at velum 0: the port bottoms
out at GARGANTA's 0.025 cm² floor.

Real-time factor at 48 kHz (`examples/bench.rs` natively under `/usr/bin/time`;
`examples/wasm_bench.mjs` under node 22 / V8; same workload code). CPU-time RTF
= audio seconds / CPU seconds of the render; wall RTF suffers the machine load.

| workload (44 + 31 volumes) | native CPU RTF | wasm32 CPU RTF | wasm32 wall RTF | load avg |
|---|---|---|---|---|
| held /a/ | 8.8x | 5.0x | 1.6x | 765 |
| /aiua/ glide, a move every 32 samples (continuous) | 2.4x | 1.57x | 0.43x | 765 |
| /aiua/, a move every 64 samples | 3.7x | **2.41x** | 0.62x | 765 |
| /aiua/, every 32 samples, skip moves < 0.001 (the wrapper's default) | 5.4x | **3.29x** | 0.88x | 765 |
| rerun at lower load: default schedule | | 3.75x | **2.45x** | 93-120 |
| rerun: a move every 64 samples (continuous) | | 2.48x | 1.03x | 93-120 |

The gate (>= 2x in wasm) passes for the default schedule in CPU time (3.3x
and 3.75x in two runs) and in wall time once the load fell to ~100 (2.45x);
continuous motion at a 64-sample interval passes in CPU time (2.4x, 2.5x);
continuous motion at a 32-sample interval does not (1.6x). Native and wasm
residuals are the same (max frame `8.9e-16 J`).

What the move schedule costs (`examples/zipper.rs`, /aiua/ glide against a
move every sample; a 1e-9 breath perturbation differs by -148.6 dB, so this
is not chaos): energy above 6 kHz +1.5 / +4.2 / +5.6 / +8.0 dB at a move
every 8 / 32 / 64 / 128 samples, +3.9 dB for the default schedule; long-term
spectrum mean |dB| 0.27 / 0.77 / 1.12 / 1.66 (0.80 default); F1/F2 tracks
within 0.3-1.1 / 3.3-7.1 Hz.

## The coordinator's string evidence, as a test

`stiff_string_law_reproduces_the_plan_anchors_and_measures_the_interpolation_fit`:
the computed stiff-string map reproduces the committed plan's open and
twelfth-fret anchors to `1e-12` (frequency) and `1e-9` (T60) for all six
strings; between them the legacy log interpolation is sharp of the law by
11.2 cents (partial 12) and 31.9 cents (partial 24) on the low E at fret 5
(fundamental 0.09 cents), worst 33.5 cents (low E, fret 6, partial 24). The
1-D plans are untouched (G-A1); the law is opt-in via `stiff_string`.

## Law, fit, approximation

| piece | status |
|---|---|
| tube network modes from the area function (lossless Webster, staggered grid) | law; the grid's dispersion `sin(k dx/2)/(k dx/2)` is its declared error (checked exactly for a uniform tube) |
| mass-momentum connection | law of the time-varying lumped network; the truncation to 24 modes is an approximation, its energy booked as disposal |
| modal-coordinate connection | the legacy declared correspondence; not a law across eigenvalue crossings |
| stiff-string map | law (Euler-Bernoulli pinned stiff string); `sigma = a + b w^2` is the steel-string compiler's design hypothesis |
| multilinear map | fit (interpolation) |
| radiation conductance `A r^2/(4 rho c l^2)` | law of matching GARGANTA's low-ka resistance; diverges above ka ~ 1 like the oracle it matches |
| wall loss and radiation diagonal in the modes | approximation (modal; exact for uniform loss) |
| the area-function family, all anatomy numbers | copied from `ovillo-tract`, labelled there (spec / design-hypothesis) |
| vowel articulations | fit (in `ovillo-tract`, against its tube); not refitted |
| flow valve: jet law, fold mechanics | copied model (Bernoulli + Poiseuille, one mass, mucosal delay; design hypotheses); exact energy contract at the node is law-by-construction; aero work booked to the lungs is an approximation |
| configuration moves every k samples | discretization of the articulator path (measured above) |

## What I would change

- A transport that is continuous in time between two computed frames, so the
  staircase (the +4 dB above 6 kHz) goes away without more eigen solves.
- Let a tube-network body recompute only the modes that moved (the warm
  path refines all 24; most barely change per step).
- Frequency-dependent viscothermal loss per mode (`alpha ~ sqrt(f)`, which
  GARGANTA's oracle has and the waveguide cannot do) is now cheap: one
  number per mode. Not done, to keep the comparison with the tube clean.
- Bind N-D coordinates in `TrajectoryBinding` (`body:<id>/config.<name>`).
