# Ariadne Guitar Metamaterial Audit

**Branch audited:** `agent/instrument-convergence-guitar-finish`  
**Scope:** guitar/string renderer, live guitar VST, pedal/amp trajectory  
**Decision:** preserve Ariadne; replace the remaining effect-chain islands rather than elaborating them

## 1. Executive judgment

The branch contains two different levels of Ariadne fidelity.

The **declared Jazzmaster architecture** is strongly Ariadne-faithful. It describes one persistent graph containing strings, bridge, afterlengths, tremolo, supports, finite-aperture readouts, a powered nonlinear body, radiator, room, and feedback, with one work/loss ledger. That is a metamaterial and a space. It is not an oscillator followed by a collection of effects.

The **live `PoweredRadiator` implementation** is only partially faithful. Its room and modal-field machinery is recognizably Ariadne: persistent states, lossless rotations, explicit damping, a finite reservoir, feedback, and receipts. Its source and cabinet island is not. Three scalar gain stages, three memoryless contractive transfer curves, one-zero interstage damping, and three one-zero “cabinet” sections are topologically an overdrive/cabinet chain, even when the functions are called contacts and their work is measured. Accounting makes that island bounded and honest; it does not make the island spatial.

This distinction matters. The right response is not to add more stages, more carefully shaped nonlinearities, or more tone macros. It is to extend Ariadne so that electrical, magnetic, mechanical, and acoustic matter can occupy the same graph.

### Audit scorecard

| Region | Faithfulness | Judgment |
|---|---:|---|
| Ariadne consensus and Jazzmaster specification | High | Keep as constitutional target |
| Room/Telaraña fields and graph receipts | High | Continue and generalize |
| Modal body | Medium | Valid reduced matter, but currently an unmeasured design sketch |
| Plucked-string implementation | Medium | Physically informed extended Karplus–Strong, not yet a comprehensive string body |
| Powered source cascade | Low | Conventional nonlinear processing in Ariadne vocabulary |
| Cabinet sections | Low | Conventional filter surrogate; must be replaced, not augmented |
| Key-tracked sympathetic field | Orthogonal | Artistic controller; exclude from physical-fidelity claims and reference mode |

## 2. The Ariadne test

A subsystem is Ariadne-faithful when all of the following are true.

1. **It owns persistent physical state.** A sample is an observation of state evolution, not the argument to a transfer function pretending to be matter.
2. **Its coordinates have a declared energy or power metric.** Stored energy, supplied work, dissipation, and radiation can be reconciled.
3. **Couplings are spatial and reciprocal where the modeled physics is reciprocal.** The loudspeaker cone drives back electromotive force; the room can drive the guitar; a bridge moves the body and sees the body’s reaction.
4. **Nonlinearity belongs to constitutive state.** Contact, geometry, magnetic saturation, charge, temperature, compliance, or boundary motion changes the evolution. A memoryless audio waveshaper is not accepted merely because its slope is bounded.
5. **Reduction preserves structure.** A modal or waveguide reduction is welcome when it preserves passivity, reciprocity, ports, and the relevant state. A fitted response that severs back-action is not.
6. **Active behavior draws from explicit storage.** Tubes, transistors, powered metamaterial cells, sustainers, and feedback amplifiers may add energy only through declared supply ports or reservoirs.
7. **The graph remains one world.** “String,” “pickup,” “pedal,” “amp,” “speaker,” and “room” are useful regions and calibration surfaces, not privileged DSP boxes.

The live source cascade fails tests 1, 3, 4, and 7. Its scalar stage memories are filter history around transfer functions, not a sufficiently expressive local physical state; the stages do not expose reciprocal power ports; the nonlinearity is applied directly to sample amplitude; and the source/cab topology remains serial.

## 3. The extension: multi-domain Ariadne

Ariadne should adopt a small port-Hamiltonian/bond-graph vocabulary without becoming a separate circuit simulator.

Every boundary exposes an inward effort/flow pair with instantaneous power

\[
P = e f.
\]

The coordinate names and units depend on the region:

| Domain | Effort | Flow | Power |
|---|---|---|---|
| Electrical | voltage | current | `V · A` |
| Translational mechanical | force | velocity | `N · m/s` |
| Acoustic | pressure | volume velocity | `Pa · m³/s` |
| Magnetic | magnetomotive force | flux rate | `A-turn · Wb/s` |
| Ariadne-normalized | metric effort | metric flow | declared normalized work rate |

Five kinds of graph element are sufficient as the constitutional core:

- **Storage cells** carry a Hamiltonian: charge, flux, momentum, displacement, field modes, string fields, plate fields, thermal state, and supply energy.
- **Lossless junctions** impose power-conserving interconnections. Transformers, gyrators, orthogonal scatterers, bridge couplings, and coordinate transports belong here.
- **Dissipative relations** remove measured work: electrical resistance, viscoelastic loss, friction, radiation, leakage, hysteresis loss, and collision loss.
- **Powered constitutive cells** have supply ports and internal state. Their signal-port gain is an executed energy conversion, never a free coefficient.
- **Readouts** observe finite spatial apertures and may load the world when the real sensor loads it.

This is an extension of existing Ariadne semantics, not a foreign architecture. The current Givens/Householder/Cayley transports are already lossless junctions in normalized coordinates. `power_port.rs` makes the invariant explicit across physical units.

## 4. The complete guitar world

A guitar faithful enough to span a Jazzmaster, archtop, lap steel, baritone, prepared guitar, twelve-string, bass, electric harp, or an instrument not yet built should be assembled from one primitive graph rather than selected from unrelated algorithms.

### 4.1 Player and contacts

The player is a set of moving, compliant boundaries with finite impedance:

- fingertip, nail, pick, bow, slide, palm, mute, ebow/sustainer, and mechanical actuator;
- distributed collision potential rather than an onset envelope;
- friction and stick/slip where applicable;
- paid work measured at the contact port;
- continuous movement after excitation, including pick scraping and fret release.

The current one-period triangular displacement plus keyed roughness is a useful initializer, but it is not yet a player model.

### 4.2 String field

The reference string needs, at minimum:

- two transverse polarizations;
- longitudinal motion and tension modulation;
- optional torsion for bowing, winding, and unusual pickups;
- stiffness dispersion and frequency-dependent loss;
- geometric nonlinearity for pitch glide, tension coupling, and phantom partials;
- distributed collisions against frets, fingerboard, bridge, nut, and neighboring structures;
- moving endpoints and afterlengths;
- energy-stable discretization at live rates.

Digital waveguides, modal coordinates, finite differences, and structure-preserving reduced models are all admissible representations. “Karplus–Strong” is not forbidden; treating its delay/filter loop as the complete ontology of a string is.

### 4.3 Bridge, neck, and body

The body should be a passive multiport mechanical admittance, not a scalar body mix.

- Ports exist at bridge saddles, nut, neck joint, tremolo pivots, tailpiece, body contact points, and optional sensors.
- A reduced model may be fitted from measured admittance matrices, but the fit must remain positive real/passive.
- Cross-polarization and cross-string transfer are first-class.
- Modes are spatial vectors with measured or authored port shapes, not only frequencies and decay times.
- Solid-body electric guitars still require this region: it controls energy leakage, dead spots, sustain, polarization rotation, and feedback thresholds even when direct acoustic radiation is weak.

The existing eight modes are an authored hypothesis. They are valid as a novel Ariadne body, not evidence that a particular electric guitar has been captured.

### 4.4 Magnetic/electromechanical pickup

A pickup is not an EQ curve and not merely a finite-aperture readout.

It requires a reciprocal transducer region containing:

- magnet/string field geometry and finite aperture;
- displacement- and velocity-dependent flux linkage;
- coil resistance, inductance, parasitic capacitance, eddy-current loss, and optional nonlinear magnetic state;
- cable capacitance and the actual pedal/amp input load;
- multiple coils, pole pieces, humbucking polarity, switching, and continuously variable routing;
- optional magnetic back-action when the use case needs pickup pull or a sustainer.

The finite-aperture machinery in the Jazzmaster spec is the correct geometric beginning. It must terminate in a loaded electrical port rather than directly becoming an audio sample.

### 4.5 Powered electrical material

The current three-stage source cascade should be deleted after its replacement is proven.

Its successor is a graph of charge/flux/polarization/thermal/supply states. It may be parameterized to reproduce triodes, pentodes, BJTs, JFETs, MOSFETs, diodes, transformers, rectifiers, bias networks, power supplies, and loudspeaker loading, but these are all instances of the same material vocabulary.

Critical behaviors then emerge from state and coupling:

- bias shift and blocking;
- supply sag and recovery;
- dynamic transconductance;
- grid/gate conduction;
- transformer magnetization and hysteresis;
- power-stage/speaker interaction;
- frequency-dependent source and load impedance;
- thermal drift;
- feedback-loop movement and instability.

No stage may be specified as “gain, waveshape, tone.” Those can remain user-facing gestures compiled into material parameters and couplings.

### 4.6 Electrodynamic radiator, enclosure, and room

The cabinet filter must be replaced by a reciprocal loudspeaker/enclosure region.

The first landed cell, `electrodynamic_radiator.rs`, contains:

- coil flux and copper loss;
- cone momentum and displacement;
- nonlinear suspension potential;
- position-dependent force factor;
- reciprocal back electromotive force;
- enclosure acoustic compliance and leakage;
- explicit radiation loss and an outward acoustic port;
- exact local energy closure under the discrete update.

This is deliberately only the first radiator primitive. A comprehensive graph additionally needs frequency-dependent radiation impedance, cone breakup/rocking modes, voice-coil inductance versus position/current, port and open-back fields, cabinet panel modes, multiple drivers, microphone diaphragms, and spatial directivity. These must be added as storage and ports, not as an impulse response after the fact.

The existing room fields are a strong foundation. Their source should become the radiator’s acoustic port, and their return should couple to cone, body, strings, and pickup/microphone states according to geometry.

## 5. What “every electric guitar” can honestly mean

No finite model with unknown parameters can identify every physical guitar from an arbitrary mono recording. The universal claim should be about **model class**, not omniscient inference.

Ariadne can plausibly span every conventional electric guitar when:

1. its primitives cover the relevant physical mechanisms;
2. a guitar definition supplies geometry, material, topology, and measured/calibrated parameters;
3. the solver preserves the power structure of those primitives;
4. validation is against independent measurements and playing gestures, not only tone matching.

Three product boundaries must be named honestly.

### Instrument renderer

Ariadne owns the string and player state. This can be the full world and can synthesize instruments that do not exist, including an electric harp whose strings, frame, pickups, powered matter, speakers, and room are continuously coupled.

### VST or digital insert

A normal insert receives terminal voltage samples after the real guitar, pickup, cable, and front-end loading have already acted. It can faithfully model the downstream powered material, speaker, enclosure, room, and feedback paths available through the host, but it cannot recover the exact upstream string/body/pickup state. Market this boundary as **Ariadne Terminal**, not as a perfect model of the input guitar.

### Pedal or amplifier hardware

Hardware can get materially closer to the full world. A high-impedance, calibrated analog front end can expose the actual pickup/cable port; controllable input impedance can make loading part of the model; a power stage and current/voltage sensing can close the loudspeaker loop; accelerometers, magnetic sensors, or a body transducer can expose additional ports. Hardware should be designed as an Ariadne boundary instrument, not merely a box running the VST.

## 6. Required migration

### Phase G0 — constitutional landing (this change)

- Add domain-explicit `PowerPort`/`PowerWave` coordinates.
- Add power-preserving transformer, gyrator, and orthogonal two-port junctions.
- Add passive wave termination.
- Add a reciprocal electrodynamic radiator with local receipts and tests.
- Document that the existing source/cabinet chain is transitional.

This phase does **not** change the plug-in sound. It lands the replacement substrate first.

### Phase G1 — remove the cabinet surrogate

- Replace `CABINET_SECTIONS` with one or more electrodynamic radiator cells plus an acoustic radiation field.
- Feed room fields from the acoustic port, not from a post-filter sample.
- Return room pressure to the cone and any enabled instrument/body ports.
- Retain a compatibility switch only for A/B validation; remove it from physical-fidelity mode.

Acceptance: back-EMF changes the upstream powered material; speaker impedance and enclosure parameters alter distortion before radiation; zero-input energy cannot grow.

### Phase G2 — replace the powered source cascade

- Introduce charge, flux, polarization, thermal, and finite-supply storage cells.
- Compile user gestures into topology and constitutive parameters.
- Replace all three memoryless contacts and interstage one-zero filters.
- Add magnetic-core and power-supply regions.
- Couple the output electrical port directly to the radiator electrical port.

Acceptance: the source has no call site equivalent to `sample = f(sample)` as its nonlinear mechanism; distortion disappears when supply work and nonlinear constitutive terms are removed; every gain event is paid.

### Phase G3 — loaded pickup and measured body

- Add reciprocal pickup field/coil/cable/load ports.
- Add passive multiport bridge/body admittance import and fitting.
- Add two-polarization plus longitudinal string solver and distributed collision graph.
- Retire the scalar “body mix” and direct readout shortcuts in reference mode.

Acceptance: changing input impedance changes the pickup resonance and upstream electromechanical state; changing bridge admittance alters string decay and body response reciprocally.

### Phase G4 — calibration and guitar atlas

A capture bundle should include:

- bridge/nut/body admittance matrices at declared ports;
- string geometry, mass, stiffness, loss, winding, and tension;
- fret/neck geometry and contact parameters;
- pickup magnetic aperture and impedance versus frequency/load;
- control and switching topology;
- powered-region voltage/current trajectories under multitone and transient probes;
- loudspeaker electrical impedance, cone displacement, and acoustic near/far-field measurements;
- uncertainty bounds and provenance.

Fits must be constrained to passive or explicitly powered model classes. A lower-error unstable or nonreciprocal fit is a failed fit unless the measured device itself supplies energy or is nonreciprocal.

## 7. No-cheating battery

The following tests should become release gates.

1. **Global closure:** old storage + external work − dissipation − radiation − new storage closes within a declared numerical tolerance.
2. **Zero-input non-growth:** every passive graph loses or preserves energy for adversarial initial states.
3. **Reciprocity:** bridge/body, pickup, transformer, and speaker ports exhibit the expected reverse coupling.
4. **Load sensitivity:** pickup, power stage, transformer, and speaker behavior changes under physically changed loads.
5. **No hidden direct path:** reference mode has no unreceipted dry path, cabinet IR, post-EQ, sample clipper, or output waveshaper.
6. **No semantic contamination:** key tracker, scale logic, shimmer pitch policy, and other compositional controllers are disabled in physical-reference renders.
7. **Structure-preserving reduction:** reduced admittances remain passive and preserve port identities.
8. **Rate consistency:** stored energy, decay, resonances, and bifurcation thresholds agree across supported sample rates.
9. **Spatial necessity:** moving a pickup, bridge port, microphone, player contact, or boundary changes the result through geometry, not a macro mapping.
10. **Independent validation:** parameters fitted on one probe family predict held-out probes and playing gestures.

A static architectural check should also reject new production uses of `tanh`, generic waveshaper helpers, “cabinet sections,” or post-hoc EQ inside physical-reference modules. Such functions are not mathematically immoral; they are simply outside this product’s constitutional promise.

## 8. Product form

The honor-system VST can remain simple while the engine is rigorous.

- **Ariadne Terminal:** live guitar input into the downstream multi-domain world.
- **Ariadne Instrument:** rendered strings and complete authored instruments.
- **Ariadne World:** speaker/room/body feedback and physically impossible but lawful spaces.

The UI should expose gestures—distance, compliance, supply, aperture, coupling, boundary, weave—not a parade of virtual knobs labeled after legacy equipment. Conventional captured instruments can be offered as named worlds, but the core value is that an artist can continuously move beyond them without crossing an algorithm boundary.

The `$25` honor price is compatible with this ambition precisely because the code is a gift rather than a scarcity play. The stronger promise is not “we copied every famous amp.” It is: **this is one lawful, inspectable world in which guitars, amplifiers, speakers, rooms, and new instruments can be built from the same matter.**

## 9. Primary technical references

- A. J. van der Schaft and B. M. Maschke, “Port-Hamiltonian Systems on Graphs,” *SIAM Journal on Control and Optimization* 51(2), 2013. DOI: `10.1137/110840091`.
- B. Bank and M. Karjalainen, “Passive Admittance Matrix Modeling for Guitar Synthesis,” DAFx-10, 2010.
- G. Evangelista and J. O. Smith III, “Structurally Passive Scattering Element for Modelling Guitar Pluck Action,” DAFx-10, 2010.
- G. Evangelista, “Physical Model of the String-Fret Interaction,” DAFx-11, 2011.
- M. Ducceschi and S. Bilbao, “Simulation of the Geometrically Exact Nonlinear String via Energy Quadratisation,” 2021, arXiv:`2112.15008`.
- S. Bilbao, A. Torin, and V. Chatziioannou, “Numerical Modeling of Collisions in Musical Instruments,” 2014, arXiv:`1405.2589`.
- R. C. D. Paiva, J. Pakarinen, and V. Välimäki, “Acoustics and Modeling of Pickups,” *Journal of the Audio Engineering Society* 60(10), 2012.
- J. Pakarinen, M. Tikander, and M. Karjalainen, “Wave Digital Modeling of the Output Chain of a Vacuum-Tube Amplifier,” DAFx-09, 2009.
- R. Ravaud, G. Lemarquand, V. Lemarquand, and T. Roussel, “Ranking of the Nonlinearities of Electrodynamic Loudspeakers,” *Archives of Acoustics* 35(1), 2010.
