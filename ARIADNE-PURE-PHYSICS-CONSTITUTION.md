# Ariadne pure-physics constitution

**Status:** binding architectural decision  
**Authority:** codifies the corrected `ARIADNE-GUITAR-METAMATERIAL-AUDIT.md` and supersedes every electrical/multi-domain recommendation or later handoff that treats pickup, pedal, amplifier, transformer, loudspeaker, cable, or circuit anatomy as part of Ariadne  
**Preserves:** `ARIADNE-IMPLEMENTATION-CONSENSUS.md`, `SPEC-ARIADNE-JAZZMASTER.md`, the work–holonomy theory, WATSN, Weave, GARGANTA, trajectory, gesture, room, and persistent-body work

## 1. Decision

Ariadne does not model electric circuits.

This is not a temporary simplification and not a product-tier distinction. Electrical and electromagnetic device anatomy is outside the Ariadne ontology. Ariadne may be evaluated against recordings, measurements, and musical practices associated with electric guitars, amplifiers, pedals, loudspeakers, and studios, but those are **behavioral references**, not renderer components.

The governing statement is:

> **Electric-guitar behavior is a phenotype in the latent behavioral space of one persistent mechanical/acoustic/material world. It is not a substrate that Ariadne reproduces.**

The same physical constitution should also contain plucked-string, percussion, bowed, synthetic, vocal-adjacent, spatial, and ambient regions. “Electric guitar” names one reachable family of behavior within that constitution.

## 2. Why this is the original Ariadne direction

The binding Jazzmaster contract already states that its target is a powered, self-radiating Ariadne string body that reaches Jazzmaster-like territory **without simulating pickup, pedal, amplifier, transformer, or loudspeaker circuits**. It derives clean response, breakup, compression, sag, bloom, sustain, radiator identity, and feedback from:

```text
string fields
<-> bridge, afterlength, tremolo, and support geometry
-> finite-aperture field readouts
-> powered nonlinear material body with finite stored supply
<-> extended mechanical/acoustic radiator
<-> causal room field
-> acoustic feedback into support and strings
```

That is the constitutional target. The later proposal to add electrical, magnetic, and electrodynamic regions was a design drift caused by treating known implementation anatomy as the shortest path to behavioral fidelity. This constitution rejects that shortcut.

## 3. Ontology

Ariadne may contain the following kinds of physical object.

### 3.1 Stored bodies

Persistent coordinates with a declared energy metric or Hamiltonian:

- string and membrane fields;
- modal and waveguide bodies;
- plates, shells, cavities, lattices, and graph media;
- displacement, momentum, strain, pressure, volume velocity, and normalized wave coordinates;
- internal material variables such as contact state, phase fraction, friction state, damage-free hysteretic state, or slow constitutive memory;
- finite active reservoirs represented as stored work capacity.

### 3.2 Lawful interconnection

- metric-orthogonal or symplectic scattering;
- reciprocal mechanical/acoustic coupling where reciprocity is claimed;
- explicit one-way observation only when it is declared as a non-loading readout;
- time-varying transport with work and monodromy receipts;
- rank changes with radiate, dissipate, retire, or reject semantics.

### 3.3 Constitutive nonlinearity

Nonlinearity belongs to evolving state and stored potential, contact, damping, geometry, or supply conversion. Admissible examples include:

- cubic and quartic elastic potentials;
- geometric tension coupling;
- buckling and bistability;
- unilateral contact and vibro-impact;
- stick/slip and rate-dependent friction;
- amplitude- and state-dependent damping;
- hysteretic material coordinates with explicit dissipation;
- state-dependent metric-compatible routing;
- reservoir-limited active damping or deformation.

### 3.4 Radiation and space

- extended radiating bodies;
- finite-aperture observation;
- directivity and spatial mode shapes;
- causal propagation;
- cavities, rooms, and long-delay fields;
- bidirectional acoustic feedback.

## 4. Excluded ontology

The following may not appear in Ariadne’s renderer IR, physical-reference modules, anatomy schemas, or promoted implementation claims:

- voltage, current, charge, electrical flux, resistance, inductance, or capacitance;
- circuit nodes, netlists, RLC networks, or circuit solvers;
- pickup coils, cable loading, passive tone networks, or switching circuits;
- diodes, tubes, transistors, transformers, rectifiers, or named amplifier stages;
- moving-coil loudspeaker anatomy, voice-coil back-emf, or electrodynamic transduction;
- circuit-faithful pedal, amplifier, or cabinet models;
- cabinet impulse responses or circuit-derived transfer functions as native physical identity.

These terms may occur in external corpus metadata, historical notes, comparison labels, or evaluation reports. They may not determine the active Ariadne state graph.

## 5. The anti-smuggling rule

Changing the nouns is not enough.

A chain written as “spring gain -> mechanical saturator -> mechanical tone section -> acoustic cabinet” is still an effects or circuit architecture in disguise. A subsystem is not Ariadne-faithful merely because every scalar transfer curve has a work receipt.

A promoted Ariadne subsystem must satisfy all of the following:

1. **Persistent distributed state.** Its identity lives in a body or field, not a memoryless sample map.
2. **Spatial consequence.** Moving a contact, aperture, coupling site, boundary, or radiator changes behavior through geometry.
3. **Back-action.** Coupled bodies react on one another unless the boundary is explicitly a readout.
4. **Constitutive nonlinearity.** Harmonic generation and compression arise in stored state, contact, or material law.
5. **Explicit active work.** Sustained or amplified behavior consumes a finite, receipted supply.
6. **One world.** There is no privileged serial chain corresponding to legacy hardware boxes.
7. **Behavioral breadth.** The same anatomy must produce non-guitar regions under other controls; it is not a circuit emulator translated into another substrate.
8. **Exact limits.** Removing each mechanism recovers a simpler physical limit exactly or within a declared numerical tolerance.

## 6. Formal behavioral framing

Let one Ariadne anatomy be parameterized by `theta`, its admissible control trajectory by `gamma`, and its persistent state by `x`:

```text
x_(n+1) = F_theta(x_n, gamma_n, u_n)
y_n     = C_theta(x_n, gamma_n)
```

The renderer is constrained by a pure-physics admissibility predicate:

```text
Pure(theta, F, C) = true
```

Let `Phi` map a render and its receipts into a behavior descriptor space containing transient, spectral, dynamic, intermodulation, memory, spatial, and feedback observables.

An electric-guitar behavioral region is a set `B_electric` in that descriptor space. The research problem is:

```text
find theta, gamma
such that Phi(render(theta, gamma)) is in or near B_electric
subject to Pure(theta, F, C)
```

This is inverse morphology and control-path search. It is not circuit identification.

Mechanistic fidelity and behavioral fidelity are deliberately separated:

- **mechanistic claim:** the render is generated by the declared Ariadne material world;
- **behavioral claim:** its measured and heard behavior lies near a named electric-guitar reference region;
- **forbidden claim:** the implementation reproduces the hidden electrical anatomy of the reference device.

## 7. Electric-guitar phenotype from pure physics

### 7.1 Clean articulation and readout identity

Target behavior:

- bright or rounded attack;
- note- and position-dependent harmonic nulls;
- bridge/neck-like contrast;
- polarity-sensitive combinations;
- touch sensitivity without post-EQ.

Ariadne mechanism:

- two-polarization string fields;
- finite-width velocity or displacement apertures;
- multiple spatial apertures with signed mode shapes;
- speaking/afterlength weighting;
- register changes that drive and observe different body subspaces.

The old `circuit=lead|rhythm` name is deprecated. The physical concept is a **register** or **routing constitution**, not a circuit.

### 7.2 Compression and edge of breakup

Target behavior:

- level-dependent crest reduction;
- gradual harmonic growth;
- attack-sensitive cleanup;
- asymmetry and even/odd harmonic balance.

Ariadne mechanism:

- positive asymmetric nonlinear potentials;
- strain-dependent passive loss;
- contact thresholds and buckling transitions;
- distributed mode coupling;
- finite reservoir limiting active work.

### 7.3 Crunch, grind, and fuzz

Target behavior:

- dense harmonic generation;
- cross-string intermodulation;
- chord clarity changing with merge topology;
- hard but bounded transitions.

Ariadne mechanism:

- cubic/quartic cross-mode potentials;
- mono, grouped, hex, and woven course-to-body coupling;
- unilateral contact and vibro-impact;
- bistable or snap-through material cells;
- path-ordered transport between nonlinear regions.

No emitted waveform clipper owns the phenotype.

### 7.4 Sag, bloom, and memory

Target behavior:

- loud events reduce subsequent headroom;
- slow recovery;
- soft-after-loud memory;
- energy arriving late in radiating modes.

Ariadne mechanism:

- finite supply reservoir;
- state-dependent supply conversion;
- slow hidden storage coupled to faster radiating modes;
- constitutive memory variables and explicit dissipation;
- work–holonomy path dependence.

### 7.5 Sustain and harmonic takeover

Target behavior:

- longer-than-passive decay;
- stable or near-stable sustain;
- selected harmonic capture;
- geometry-dependent feedback threshold.

Ariadne mechanism:

- bounded negative damping or positive deformation paid by the reservoir;
- an extended radiator coupled to a causal room;
- delayed pressure/velocity feedback into body and strings;
- nonlinear competition among modes.

### 7.6 Cabinet- and speaker-like identity without a speaker model

Target behavior:

- broad spectral tilt;
- breakup bands;
- direction-dependent tone;
- near/far differences;
- room interaction.

Ariadne mechanism:

- a mechanical/acoustic shell, plate, membrane, or graph radiator;
- distributed modal radiation resistance;
- nonlinear mode coupling and contact inside the radiator;
- enclosure or cavity geometry as an acoustic body;
- directivity derived from spatial mode shapes;
- room coupling from the radiator field itself.

The target is a radiating phenotype, not moving-coil anatomy.

## 8. Shared latent world

The electric-guitar region must remain connected to other Ariadne regions.

| Region | Dominant physical coordinates |
|---|---|
| plucked/acoustic | string mechanics, passive body, direct radiation |
| percussion | impact/contact, high loss, short storage, shell modes |
| bowed/breathed | continuous source work, friction or flow coupling |
| synthesizer-like | stronger active material, dense topology, moving transport |
| ambient/spatial | long-delay halo, slow transport, high path memory |
| electric-guitar-like | finite-aperture strings, active nonlinear body, extended radiator, acoustic feedback |

A good anatomy should move among several rows continuously. If a design can only imitate one named rig, it has failed the Ariadne test even when the imitation is convincing.

## 9. Implementation decisions

### 9.1 Active reference path

The new reference path is `mus_dsp::ariadne_terminal`: one normalized mechanical/acoustic material graph with:

- positive asymmetric stored potentials;
- discrete-gradient conservative propagation;
- reservoir-limited positive deformation;
- sparse closed metric-compatible transport;
- finite-aperture boundary exchange and radiation;
- a bidirectionally coupled long-delay room field;
- one global work/loss/radiation ledger.

It contains no circuit or electrodynamic state.

### 9.2 Transitional paths

`powered_radiator` remains a compatibility and negative-control implementation. Its serial source/cabinet-like islands are not constitutional precedent and are not product-facing on this branch.

The electrical-domain `power_port` and `electrodynamic_radiator` modules are removed from Ariadne’s active surface. Their history remains available through Git.

### 9.3 Naming

New APIs, schemas, scores, and UI must use:

- `register`, not `circuit`;
- `aperture` or `readout`, not `pickup`;
- `powered_material`, `supply`, or `reservoir`, not `amp`;
- `radiator`, `shell`, `plate`, or `field`, not `speaker` or `cabinet`;
- `behavior_target`, not `emulation_target`.

Reference corpus labels may retain conventional musical vocabulary when needed for human communication.

## 10. Exact-zero laws

Every promoted profile must demonstrate at least:

- `supply = 0` removes all positive active work;
- `curvature = 0` recovers the linear material potential;
- `interlace = 0` removes ordered internal transport;
- `space = 0` disconnects the room exactly;
- `feedback = 0` removes room-to-body exchange attributable to feedback;
- `radiation = 0` preserves internal energy except declared loss;
- `nonlinearity = 0` removes level-dependent harmonic generation beyond the string/body baseline;
- passive tail state cannot grow without boundary, control, or supply work;
- dry/observer paths cannot alter physical state;
- block segmentation cannot change state or audio.

## 11. Search and validation discipline

Electric-guitar behavioral fitting must use:

1. a factorial set of strings, notes, chords, attack levels, positions, articulations, and feedback geometries;
2. held-out notes, chords, levels, gestures, and trajectories;
3. one anatomy across the corpus, with separate lawful performance controls;
4. multiobjective fitting over descriptors rather than a single waveform distance;
5. independent listening gates;
6. full work and state receipts;
7. explicit novelty controls showing the same anatomy outside the guitar region.

A target recording may guide the search. It may not enter playback, define a hidden sidecar, or become a residual renderer.

## 12. Behavioral promotion ladder

1. **pure-physics-valid** — laws, bounds, receipts, and no-circuit check pass;
2. **electric-adjacent** — descriptor movement toward the reference is reproducible;
3. **electric-region-capable** — one anatomy spans clean, edge, driven, sustaining, and feedback-rich probes;
4. **held-out-generalizing** — behavior persists on untouched notes, chords, levels, and gestures;
5. **listener-recognizable** — blinded listeners identify the intended electric-guitar region above preregistered controls;
6. **musically preferable** — players prefer or use the region in a task, separately from resemblance.

No rung implies circuit or hardware reconstruction.

## 13. Machine enforcement

The active Ariadne reference path is guarded by a source check that rejects electrical-domain identifiers and legacy device anatomy. The check is intentionally narrower than a prose grep: historical notes and evaluation labels may discuss comparison hardware, while active physical modules may not instantiate it.

Future implementation reviews must reject:

- a new circuit/electromagnetic state type;
- a serial gain/waveshape/tone chain in physical clothing;
- a post-hoc cabinet or pickup response;
- a learned residual in the native render path;
- a control whose only semantics are a conventional hardware knob;
- an unreceipted active gain event;
- behavioral fitting that changes anatomy independently per note.

## 14. Migration order

1. Land this constitution and the electric-region search spec.
2. Land and audit the pure `ariadne_terminal` reference body.
3. Add the no-circuit source guard.
4. Run law, stability, and performance tests.
5. Build the factorial behavioral atlas against the current powered-radiator control and external references.
6. Search anatomy and trajectories under the pure-physics constraint.
7. Keep the draft live plugin wired to the pure material for integration testing, but do not ship a release until Rust, real-time, aliasing, listening, and session gates pass.
8. Remove the transitional powered-radiator implementation only after its negative-control value and session-migration obligations are exhausted.

## 15. Bottom line

Ariadne should not become a universal simulator of every known mechanism that has produced music.

Its stronger wager is:

> **A sufficiently expressive, lawful, persistent world of strings, bodies, active materials, radiators, spaces, and control paths already contains the musical behaviors we care about—including electric guitar—without reproducing the historical machinery that first discovered them.**
