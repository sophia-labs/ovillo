# Unified U3 — the persistent protected backbone in `mus-dsp`

Binding sources: `RUST-API-CONTRACT.md` §§10–13, `UNIFIED-INSTRUMENT-THEORY.md`
§§13–15, handoff stage U3, and their N2 (no sidecar), N3 (no one-way core),
N14 (no fresh body per note). U1+U2 are landed (`3e175cc`): the complex
executor replays the committed words; the family/path layer refuses
lawfully. U3 is where sound changes: the braid stops being a Python gain
law and becomes state inside the instrument.

## 0. Architecture honesty — what our engine is

The render face is **eager**: `weave_note` returns a complete note buffer;
there is no per-sample host callback. The backbone therefore lives at the
**track level** and advances in **absolute sample time** across the track's
events, with all coupling happening inside `StringNetworkVoice`'s sample
loop while a note renders and free evolution bridging the gaps between
events. This is a faithful reduction of their U3, not the streaming plugin
lifecycle — that arrives with the CLAP work and is deliberately out of
scope here.

## 1. Module and ownership

`mus-rs/crates/mus-dsp/src/backbone.rs`:

```text
ProtectedBackbone {
    state: [Complex64; 16],          // region-major: string 0-3, body 4-7,
                                     // near 8-11, room 12-15
    absolute_sample: u64,
    schedule: armed complex words at absolute sample indices,
    coupling: f64,                   // bridge strength, 0 = exact absence
    ledger: accumulated UnifiedWorkFrame fields,
}
```

The backbone follows the gesture-seam doctrine verbatim: it takes
`mus_watsn::ComplexOperatorProgram` words **as data** and does not know
what compiled them. `mus-dsp`'s library dependency set is unchanged.

One backbone per weave track, constructed at track start, surviving every
event boundary. Only explicit reset (score-authored) clears it; the
reset is an operation with a receipt, never a side effect of note-off.

## 2. Frozen stage order (per sample, while coupled)

```text
1. transport   — due gates from the armed schedule execute (CompiledGeometric)
2. bridge      — energy-preserving rotation between string-region channels
                 (0..3) and the note's selected course wave variables
3. radiate     — passive colligation on room channels (12..15); emitted
                 sample adds into the note buffer
4. ledger      — per-stage energy deltas accumulate
```

Between events (gap samples) stages 1, 3, 4 run at event boundaries in
closed form (unitary transport preserves state; radiation applies its
per-sample contraction as an exact power); no per-sample loop runs for
silent gaps. Any change to this order is a version change.

## 3. Score surface (opt-in, corpus-invariant)

New weave patch keys, all absent from every existing score:

```text
braid=1                arm the backbone for this track
braid_word=A|B|AB|BA|C|inverse   compiled word(s) for this event,
                       executed across the event's samples
braid_couple=0.02      per-sample bridge coupling (0 = exact zero law)
braid_reset=1          explicit state retirement before this event
```

`braid` absent → the backbone path is not entered at all → today's render,
byte-identical, structurally (the retune-parity law extends unchanged).
Two braid-armed events overlapping in time on one track is a render
refusal naming `BlockScheduleAmbiguous` — eager rendering cannot
interleave two concurrent sample loops over shared state, and a wrong
answer must not be representable.

## 4. Laws (each one a test)

1. **Absence.** No `braid` key → byte-identical output on the full corpus
   digest suite (structural non-participation, same argument as the
   gesture seam).
2. **Zero-coupling.** `braid=1 braid_couple=0` enters the path, evolves
   the backbone, and still produces byte-identical audio — injection is
   the only door; zero coupling means zero energy in, zero radiated out.
3. **Bidirectionality.** With coupling on, energy measurably flows
   note→backbone AND backbone→note (their N3): a ringing backbone excites
   a fresh silent string; receipts carry both directions.
4. **Persistence.** Word A on event 1, word B on event 2: final backbone
   state equals the Python checkpoint composition (`W_B W_A prepared`,
   exported to the test vectors); the reset-between-events control
   differs by the checkpointed distance (`0.8897`); AB vs BA final-state
   distance replays `0.7757`.
5. **Ledger.** Per-event frame closes: `E1 = E0 + injected - radiated -
   retired + residual`, residual at float scale; radiated energy equals
   the energy added to the audio buffer before any downstream bus/master
   (which stay outside the ledger, labeled).
6. **Segmentation.** One event of duration 2d and two tied events of
   duration d with the same word schedule produce identical backbone
   state and identical audio.
7. **Refusals.** Overlap → `BlockScheduleAmbiguous`; unknown word name →
   typed render refusal; malformed word data → the U1 validation error
   surfaces with its gate index.

## 5. The audible deliverable

`tools/` demo: one weave track, four events — pluck·A, pluck·B versus
pluck·B, pluck·A — same gates, same energies, opposite order, rendered
through the real engine with the backbone radiating between notes. The
first sound of the unified instrument *inside* mus. Delivered to the
listening folder next to the mathematicians' reference stimuli.

## 6. Exit gates

- All seven laws green as tests in `mus-dsp` (unit + integration).
- Test-vector export extended with persistent-state checkpoints
  (prepared / AB / BA final states); CI vector-drift check still passes.
- Full corpus render digests unchanged (absence law at scale).
- Workspace clippy `-D warnings`; `mus-dsp` suite green including
  retune-parity in whatever profile the pins are certified for (see the
  parity investigation running alongside this keystone — its resolution
  lands with U3, not after).

## 7. Deferred, named

- `PhysicalAdiabatic` in-voice execution (U2's refusal machinery guards
  it; frozen-family evolution in the sample loop is its own build).
- Polyphonic overlapping braid semantics (needs a mixing law with a
  proof, not an arbitrary choice).
- Host-block streaming lifecycle (CLAP plugin work).
- U4 full loss/radiation integration for the *strings* (the backbone's
  own radiation port lands here; the string network's existing damping
  stays as-is, outside the backbone ledger).
- U5 production region mapping (body/near channels participate via the
  regional mixer inherited from the family; mapping them onto the
  instrument's actual body filters is its own stage).
