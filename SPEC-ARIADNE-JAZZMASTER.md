# Ariadne Jazzmaster — native offset electric string instrument

**Status:** implementation contract, draft 0.1  
**Base:** `agent/trajectory-persistent-bodies` at `17174d5c3a2e0caabc68937b9e21af9d7eb3f6c2`  
**Profile id:** `ariadne-jazzmaster-v0`  
**Claim:** a powered, self-radiating Ariadne string body that occupies Jazzmaster-like musical territory without simulating pickup, pedal, amplifier, transformer, or loudspeaker circuits.

## 0. Thesis

The instrument is not:

```text
string model -> pickup EQ -> distortion -> compressor -> cabinet IR
```

It is one persistent dynamical graph:

```text
player attacks and contacts
  -> six two-polarization speaking strings
  <-> rocking bridge
  <-> six resonant behind-bridge string segments
  <-> floating spring tremolo plate
  <-> passive neck/body/support modes
  -> finite-aperture geometric readouts
  -> powered nonlinear Ariadne mode graph with a finite energy reservoir
  <-> spatially extended radiator
  <-> causal room field
  -> delayed acoustic feedback into support and strings
```

The familiar electric-guitar behaviors must arise inside that graph:

- touch-sensitive clean response from string mechanics and readout geometry;
- chime and metallic after-ring from the behind-bridge segments;
- vibrato from shared bridge/tremolo geometry, not a pitch shifter;
- compression, edge, crunch, fuzz, sag, bloom, and sustain from stored nonlinear state and a finite active reservoir;
- chord grind or clarity from where course signals merge before nonlinear mode coupling;
- cabinet-like spectral and spatial identity from actual radiator modes;
- feedback from a causal acoustic loop;
- stereo, HOA, and binaural output as observations of the same radiator state.

No promoted preset may rely on `drive=`, `dist=`, post-EQ, post-compression, cabinet convolution, stereo widening, or a final neural residual for its defining behavior. A final delivery hardener is allowed only as an explicitly separate transport-safety layer.

## 1. Reference instrument and nonclaims

The conservative anchor is the classic offset Jazzmaster family:

- six steel courses at a 25.5-inch / 648 mm scale;
- a separate floating bridge and anchored floating vibrato/tremolo system;
- a bridge intended to move with tremolo action rather than force every string to slide across a stationary saddle;
- a tremolo lock;
- a substantial bridge-to-tailpiece afterlength;
- two string-sensing regions and lead/rhythm routing identities;
- a solid-body, bolt-on-neck support structure.

The initial profile uses the vintage-oriented `.010-.046` gauge family as its conservative string-set hypothesis. Geometry and setup constants not fixed by the sources are marked `design-hypothesis` or `fit`, never presented as measurements.

Reference sources for the implementation team:

- Clarence L. Fender, US2972923A, *Floating tremolo and bridge construction for lute-type musical instruments*.
- Fender, *History of the Fender Jazzmaster*.
- Fender, Vintera III Mid '60s Jazzmaster specifications.
- Fender, Pure Vintage '65 Jazzmaster pickup-set specifications.

This work does **not** claim:

- Fender endorsement;
- electromagnetic pickup or passive-circuit simulation;
- circuit-faithful amplifier, pedal, transformer, speaker, or cabinet emulation;
- exact reproduction of one serial-numbered instrument;
- that every sound historically made with a Jazzmaster is latent in passive wood and wire.

The behavioral claim is narrower and more interesting:

> One Ariadne-native physical constitution should traverse clean, compressed, edge-of-breakup, saturated, sustaining, feedback-rich, surf-like, shoegaze-dense, and spatial electric-guitar regions while retaining one causal state graph and explicit work accounting.

## 2. Compatibility law

The existing `pluck` and `weave` paths remain exact compatibility anchors.

```text
anatomy absent
  -> current StringNetworkVoice path, byte laws unchanged

anatomy=ariadne-jazzmaster-v0
  -> new JazzmasterVoice path
```

The implementation must not add another source-rewrite layer to the existing `pluck.rs` build stitch. The first engineering task is a private-module decomposition that preserves every current byte pin. The new instrument may reuse extracted course, allpass, damping, contact, trajectory, and work primitives, but it must not mutate the historical render path by default.

## 3. State constitution

A `JazzmasterVoice` owns one persistent state for the declared body lifetime.

```rust
struct JazzmasterVoice {
    courses: [OffsetCourse; 6],
    bridge: FloatingBridge,
    tremolo: FloatingTremolo,
    support: PassiveSupportBody,
    readouts: FieldReadoutBank,
    power_body: PoweredModeGraph,
    radiator: RadiatorBank,
    feedback: AcousticFeedbackNetwork,
    controls: JazzmasterControlState,
    ledger: JazzmasterEnergyLedger,
}
```

### 3.1 Course state

Each principal course has two transverse polarizations and at least two connected propagation segments:

```text
speaking segment    nut/fret -> saddle
behind-bridge       saddle -> tremolo anchor
```

A later phase may add a nut-to-tuner/headstock segment. It must not be invented as post-filter combing.

For course `i` and polarization `p`:

```text
u_s[i,p]  speaking displacement/velocity wave state
u_a[i,p]  afterlength displacement/velocity wave state
```

Both segments use the existing phase-compensated damping, dispersion, retune, contact, and work-accounting discipline. The afterlength is a real resonator. It is neither a delayed copy nor a decorative noise layer.

### 3.2 Bridge state

The bridge is a shared multiport junction with explicit translation and rocking coordinates:

```text
q_bridge        common bridge displacement
phi_bridge      bridge rocking angle
p_bridge        conjugate momenta
c_saddle[i]     per-course contact/slip state
```

The junction receives effort/flow from both speaking and afterlength segments, support modes, and tremolo geometry. Static scattering must be passive. Moving geometry performs or extracts work and must be receipted.

### 3.3 Tremolo state

The tremolo plate is a shared spring-loaded coordinate:

```text
theta_trem      plate angle/displacement
p_trem          plate momentum
lock_state      open | engaging | locked | releasing
arm_input       torque-owned | position-owned
```

A minimal potential is:

\[
U_T(\theta)=\frac12 k_T\theta^2+\frac14\beta_T\theta^4,
\qquad \beta_T\ge0.
\]

All six anchor points move with `theta_trem`. Resulting pitch motion emerges from string geometry and tension. The tremolo is not mapped to `pitch_ratio` after the fact.

The arm has two mutually exclusive ownership modes:

```text
arm_torque(t)    performer supplies torque; position emerges
arm_position(t)  active servo follows position; servo work is explicit
```

Authoring both is a refusal.

Tremolo-lock engagement is a topology/state constraint. The compiler must choose one disposition:

```text
refuse_if_moving
transfer_to_support
explicitly_dissipate
```

Silently zeroing tremolo state is forbidden.

### 3.4 Passive support body

The solid body, neck, bridge posts, tremolo mounting, and player contact are represented by passive modes with multiple coupling ports:

```text
frequency
T60 / damping
bridge residue
nut residue
plate residue
player-contact residue
weak acoustic-radiation residue
modal normalization / impedance
```

This body primarily changes string evolution and decay. It may radiate weakly, but it must not be confused with the powered radiator that creates the amplified-instrument-scale output.

## 4. Geometric readout — the native replacement for pickup behavior

The model does not simulate coils or circuits. It samples the evolving string field through finite spatial apertures.

For aperture `a`, course `i`, and polarization `p`:

\[
r_{aip}(t)=\int_0^{L_i} w_{aip}(x)\,\partial_t u_{ip}(x,t)\,dx.
\]

`w` declares:

- position along the speaking or afterlength segment;
- finite width;
- shape and polarity;
- polarization sensitivity;
- per-course weighting.

The default anatomy has three readout regions:

```text
bridge      narrow/medium speaking-string aperture near the saddle
neck        wider speaking-string aperture farther from the saddle
chime       low-gain aperture on the behind-bridge segments
```

The finite aperture must create its own note- and position-dependent modal cancellation. A fixed EQ is not an acceptable substitute.

### 4.1 Circuit identity without electrical simulation

`circuit=lead|rhythm` selects a constitutive routing, not an output filter.

```text
lead
  bridge / both / neck selector active
  broad powered-mode rank
  higher nonlinear-mode participation
  full radiator set

rhythm
  neck aperture only
  reduced-rank slow powered-mode manifold
  lower upper-mode drive and lower spatial directivity order
  independent reservoir/headroom calibration
```

A rhythm setting may sound darker because it excites a different dynamical subspace. It must not be implemented as `lowpass(output)`.

### 4.2 Course merge topology

Where courses merge relative to the nonlinearity is a first-class physical coordinate.

```text
mono
  sum courses, then excite the nonlinear body
  strongest cross-string intermodulation

hex
  each course excites its own nonlinear mode group before merging
  strongest chord clarity

grouped
  declared course groups merge before separate nonlinear mode groups

woven
  course readouts traverse a sparse ordered Givens graph before and between
  nonlinear mode groups; ordering, chirality, orbit, and state curvature matter
```

`woven` is the default Ariadne-native region. It should include the familiar `mono` and `hex` cases as continuous or typed limits rather than unrelated algorithms.

## 5. Powered nonlinear body

The powered body is a fictional active material. It is the native source of amplified scale, compression, breakup, sustain, and bloom.

Let `q,p` be powered modal coordinates. A bounded stored-energy model is:

\[
H_P(q,p)=
\frac12 p^T M^{-1}p+
\frac12 q^T Kq+
\sum_j\left(\frac{\alpha_j}{3}q_j^3+
\frac{\beta_j}{4}q_j^4\right)+
\frac12\sum_{j<k}\gamma_{jk}q_j^2q_k^2.
\]

Requirements:

- every `beta_j >= 0`;
- the quartic cross form is bounded below on the admitted control set;
- passive inter-mode routing is orthogonal, symplectic, or explicitly contractive;
- active force is separately attributed to the supply reservoir;
- no memoryless output clipper is part of the native claim.

### 5.1 Finite energy reservoir

The supply state `rho` is explicit:

\[
0\le\rho\le\rho_{max}
\]

\[
\dot\rho=
\frac{\rho_0-\rho}{\tau_{recover}}
-P_{active}
-D_\rho(\rho).
\]

For active force `f_a` and modal velocity `v`:

\[
P_{active}=f_a^Tv.
\]

Positive work reduces the reservoir. Extracted work may recharge it only under a declared efficiency. Active force must tend continuously to zero as the available reservoir tends to zero.

This state owns:

```text
headroom      reservoir capacity and nonlinear thresholds
sag           reservoir depletion and recovery
compression   supply limitation plus amplitude-dependent physical loss
sustain       bounded negative damping paid by the reservoir
bloom         slow transfer from driven to radiating modes
cleanup       lower drive remaining in the linear/high-headroom region
```

### 5.2 Breakup mechanisms

The profile may combine:

```text
cubic/quartic modal stiffness
amplitude-dependent passive damping
state-dependent sparse coupling
unilateral or bilateral contact stops
reservoir-limited active damping/anti-damping
```

A fuzz-like contact law may use:

\[
F_c(q)=k_+[q-q_+]_+^n-k_-[-q-q_-]_+^n.
\]

It acts on modal state before the integration step. It is not a hard clip of the emitted waveform.

### 5.3 Native control meanings

```text
drive       readout-to-powered-body coupling
headroom    capacity and saturation scale
bias        equilibrium/asymmetry of nonlinear modes
sag         recovery time and shared supply coupling
bloom       slow-mode transfer strength
sustain     low-level active anti-damping, reservoir limited
interlace   course-merge graph depth
chirality   ordered routing bias
motion      orbit rate/depth inside the mode graph
curvature   state-dependent routing metric
```

All controls must have exact zero limits and bounded domains.

## 6. Jazzmaster mechanics as Ariadne controls

### 6.1 Behind-bridge chime

`chime` is not a spectral enhancer. It controls physical coupling and observation of the six afterlength segments:

```text
bridge transfer into afterlengths
afterlength damping/contact
chime readout weight
routing from afterlength readouts into powered modes
```

A normal speaking-string attack may excite afterlengths only through the bridge. A dedicated `behind_bridge` attack addresses them directly and has its own work receipt.

### 6.2 Rocking bridge

The bridge should naturally rock under shared tremolo movement. The conservative default minimizes string-on-saddle slip. A separate contact model admits:

```text
stick
rock
micro-slip
full slip
saddle rattle / buzz
```

Frictional transitions must dissipate or transfer energy explicitly. They must not randomly retune the delay line.

### 6.3 Tremolo path dependence

The tremolo moves all six courses and their afterlengths through one shared coordinate. In the Ariadne region it may also deform the course-merge and powered-mode coupling graph, but only through declared physical couplings.

The same final arm position may therefore produce different internal state after different arm paths. The claim is dynamical path dependence. Do not label it geometric holonomy unless the stronger `SPEC-PROTECTION.md` hypotheses are actually established.

### 6.4 Circuit switching as state transport

A live `lead <-> rhythm` switch changes which powered subspace is driven and observed. The transition must be one of:

```text
instantaneous typed topology edit with explicit impulse/work receipt
finite cross-transport over a declared duration
refusal while stored power-body energy exceeds a threshold
```

A hidden crossfade on completed audio is forbidden.

## 7. Spatial radiator and feedback

The powered body drives an extended radiator, not one anonymous mono tap.

For radiator mode `m`:

```text
modal source state
radiation center and orientation
radiation resistance
frequency-banded spherical-harmonic coefficients
valid distance/frequency domain
provenance: designed | fit | measured
```

The field may be observed as:

```text
stereo direct feeds
ACN/SN3D FOA
higher-order Ambisonics
HRTF/BRIR binaural
microphone arrays
```

The renderer must preserve one physical radiator render and derive multiple observation targets from it. Raw Ambisonic channels are transports, not direct speaker feeds.

### 7.1 Propagation and Doppler

Every radiator-to-listener or radiator-to-guitar path has causal propagation delay. For emission time `t_e` and receive time `t_r`:

\[
t_r=t_e+\frac{\lVert x_r(t_r)-x_s(t_e)\rVert}{c}.
\]

Doppler emerges from the varying retarded-time map. No pitch-shift stage is added.

### 7.2 Near field and order

Use the smallest valid representation:

```text
very close extended source     direct radiator-patch propagation
moderate finite distance       near-field HOA encoding
far field                      ordinary HOA plane-wave approximation
```

FOA must remain the exact `order=1` compatibility limit of the HOA implementation.

### 7.3 Acoustic feedback

The radiator field returns to the support body and strings through a delayed directional path:

\[
f_{fb}(t)=C_{fb}\,p_{room}(x_{guitar},t-\tau).
\]

Feedback gain, delay, orientation, room modes, played pitch, support modes, and reservoir state jointly determine instability. This is the source of controlled sustain and harmonic takeover. A post-resonator is not.

The feedback loop must distinguish:

```text
subcritical decay
bounded self-sustain
mode capture / harmonic takeover
unstable growth -> typed refusal or protective retirement
```

## 8. Energy and work constitution

The declared stored energy is:

\[
E=
E_{speaking}+E_{afterlength}+E_{bridge}+E_{tremolo}+
E_{support}+E_{powered}+\rho+E_{feedback/room}.
\]

The frame ledger is:

\[
\Delta E=
W_{attack}+W_{finger}+W_{arm}+W_{supply}+W_{topology}
-D_{string}-D_{contact}-D_{bridge}-D_{support}-D_{power}
-P_{radiated}-P_{escaped}+\epsilon.
\]

Every term is signed and content-addressed. Required subreceipts:

```text
RearticulationReceipt       existing per-attack work
TremoloWorkReceipt          arm and geometry work
BridgeContactReceipt        slip/rattle loss and impulses
TopologyTransportReceipt    circuit/merge/lock transition
SupplyReceipt               reservoir work, recharge, minimum state
RadiationReceipt            energy leaving each radiator mode
FeedbackExchangeReceipt     room/body/string exchange
ClosureReceipt              stored delta, sources, losses, residual
DeliveryReceipt             digital/spatial/playback-bound safety
```

`observer_off` production may omit expensive diagnostic decomposition only if the promoted claim does not depend on it. The Jazzmaster release path requires at least fixed-size accumulators for attack, arm, supply, radiated energy, feedback exchange, and total closure.

## 9. Delivery and speaker protection

Native dynamics and transport safety are separate layers.

The physical model owns musical density. The delivery layer owns:

```text
finite-sample validation
subsonic high-pass when not already physically guaranteed
reconstructed/true-peak ceiling
100 ms and 1 s RMS ceilings
low-band short-window energy ceiling
shared attenuation across channels
FOA/HOA decoder-norm contracts
optional calibrated voltage, thermal-power, and excursion bounds
```

No audition WAV, spatial render, probe, or example may bypass the delivery receipt. Research analyses may inspect raw state/audio only when the artifact is clearly marked `analysis-only:not-for-playback`.

A generic file cannot guarantee an arbitrary amplifier/speaker chain. Calibrated electroacoustic assurance requires a named playback profile. The uncalibrated profile guarantees only the declared digital and decoder-bound transport envelope.

## 10. Authoring surface

The target notation is illustrative until the parser and registry land.

```mus
# instruments:
#   jm = offset loom (
#     treble,
#     synth=weave,
#     anatomy=ariadne-jazzmaster-v0,
#     body_lifetime=track,
#     circuit=lead,
#     readout=both,
#     merge=woven,
#     drive=0.34,
#     headroom=0.72,
#     sag=0.28,
#     bloom=0.36,
#     chime=0.42,
#     feedback=0.08,
#     field=hoa3
#   )

bar 1: jm={p} E2q[jm_arm=0@0->-0.05@0.45->0@1] Rq B2q Rq
bar 2: jm={f} [E2,B2,E3]h[acc] Rh
```

### 10.1 Static profile coordinates

```text
anatomy
string_set
speaking_length
afterlength ratios
support mode table
readout aperture definitions
powered-mode graph
radiator geometry
feedback topology
```

Changing these is a reconstruction or topology edit, not ordinary automation.

### 10.2 Continuous coordinates

Initial registry:

```text
jm.tremolo.arm_torque_nm
jm.tremolo.arm_position_rad
jm.tremolo.spring_bias
jm.bridge.rock_damping
jm.bridge.contact_pressure
jm.afterlength.coupling
jm.afterlength.readout_weight
jm.readout.bridge_weight
jm.readout.neck_weight
jm.readout.chime_weight
jm.readout.aperture_position
jm.readout.aperture_width
jm.merge.interlace
jm.merge.chirality
jm.merge.orbit_hz
jm.merge.orbit_depth
jm.merge.curvature
jm.power.drive
jm.power.headroom
jm.power.bias
jm.power.sag
jm.power.bloom
jm.power.sustain
jm.feedback.gain
jm.feedback.orientation
```

Each coordinate needs unit, finite bounds, derivative limits, work account, and state semantics in `mus-trajectory`.

### 10.3 Typed edits

```text
jm.tremolo.lock
jm.circuit
jm.readout.selector
jm.merge.topology
jm.feedback.path
jm.radiator.order
```

Each edit needs a state disposition. A selector is not a string parameter pretending to be continuous.

## 11. First-page performance controls

The expert surface remains complete in MUS. The playable page should expose:

```text
Arm        physical tremolo gesture
Chime      behind-bridge participation
Aperture   bridge/neck field and spatial width of readout
Interlace  mono <-> grouped <-> hex <-> woven merge depth
Drive      coupling into powered modes
Headroom   reservoir capacity and nonlinear threshold
Bloom      slow modal transfer and sustain character
Field      radiator spread/directivity/spatial order
```

Switches:

```text
Circuit    lead | rhythm
Selector   bridge | both | neck
Lock       tremolo open | locked
```

Advanced controls:

```text
Sag, Bias, Sustain, Chirality, Motion, Curvature, Feedback, Bridge Contact
```

All macros resolve to scientific coordinates and the receipt records both layers.

## 12. Conservative and extended regions

One connected parameter space should contain:

### `offset_clean`

```text
mono/grouped merge
linear powered modes
high reservoir
low chime
low feedback
static radiator
```

### `offset_surf`

```text
bridge-biased aperture
moderate chime
gentle arm motion
bright radiator participation
causal room, low active drive
```

### `offset_rhythm`

```text
rhythm circuit
neck aperture
reduced-rank slow power manifold
low upper-radiator drive
```

### `offset_edge`

```text
moderate drive
first nonlinear modes active
mild reservoir depletion
mono/grouped merge
```

### `offset_wall`

```text
woven merge
afterlength participation
staggered nonlinear thresholds
slow bloom
bounded feedback
multi-radiator HOA field
```

### `offset_labyrinth`

```text
moving ordered merge graph
arm-dependent coupling geometry
state curvature
path-dependent breakup and radiation
```

The clean-to-wall transition must use one state graph. Preset changes may move parameters or typed topology, but may not swap in unrelated effect algorithms.

## 13. Rust architecture

Do not grow `pluck.rs` further. Target layout:

```text
mus-rs/crates/mus-dsp/src/string_core/
  course.rs
  excitation.rs
  retune.rs
  contact.rs
  energy.rs
  legacy_adapter.rs

mus-rs/crates/mus-dsp/src/jazzmaster/
  mod.rs
  profile.rs
  course.rs
  bridge.rs
  tremolo.rs
  support.rs
  readout.rs
  routing.rs
  power_body.rs
  reservoir.rs
  radiator.rs
  feedback.rs
  controls.rs
  energy.rs
  receipt.rs
```

Proposed public seam:

```rust
pub struct JazzmasterVoice;

impl JazzmasterVoice {
    pub fn new(
        profile: &JazzmasterProfile,
        tuning: &[f64; 6],
        duration_samples: usize,
        attacks: AttackSchedule,
        trajectories: JazzmasterTrajectoryProgram,
    ) -> Result<Self, JazzmasterError>;

    pub fn next_radiator_frame(&mut self) -> JazzmasterRadiatorFrame;
    pub fn receipt(&self) -> &JazzmasterReceipt;
}
```

`JazzmasterRadiatorFrame` contains physical radiator-mode outputs, not final stereo.

The scene renderer consumes those modes directly or through a content-addressed stem bundle. Score pan and Haas are refused for native spatial authority.

## 14. Implementation stages

### JM0 — decompose without drift

- extract private course/retune/contact primitives from the monolith;
- keep legacy `pluck` and `weave` APIs and bytes exact;
- replace build-source stitching only after pins pass;
- no Jazzmaster audio claim yet.

Stop condition: complete current release workspace tests and exact byte pins.

### JM1 — two-segment course

- speaking and afterlength segments;
- passive energy-preserving bridge junction;
- direct afterlength attack;
- no tremolo motion, no active body.

Stop conditions:

- zero afterlength coupling equals the extracted single-segment course;
- static junction closure residual within declared f64 tolerance;
- afterlength frequencies follow physical length;
- no post-comb substitute.

### JM2 — rocking bridge and floating tremolo

- bridge translation/rock modes;
- common plate and spring;
- arm torque and position ownership;
- lock disposition;
- exact arm/geometry work.

Stop conditions:

- zero arm path equals JM1;
- six courses retune through one shared plate coordinate;
- return-to-neutral residual and hysteresis are measured;
- no pitch-shift postprocess;
- lock never deletes energy silently.

### JM3 — finite-aperture readout

- bridge, neck, and afterlength apertures;
- selector and circuit routing;
- mono/hex/grouped merge controls;
- no active nonlinearity yet.

Stop conditions:

- analytic sinusoid/aperture tests;
- readout position changes note-dependent harmonic nulls;
- one fixed profile generalizes across held-out notes and pluck positions;
- rhythm identity arises from subspace routing, not output filtering.

### JM4 — powered body and reservoir

- bounded nonlinear mode graph;
- finite reservoir, recovery, supply work;
- compression, edge, sag, bloom, sustain;
- woven merge topology.

Stop conditions:

- supply zero removes active work exactly;
- nonlinearity zero removes added harmonic generation exactly;
- active force vanishes at empty reservoir;
- corner grid remains finite and bounded;
- exact or bounded closure receipt;
- no defining post effect.

### JM5 — persistent score integration

- reuse track-owned lifetime;
- fixed six-course topology and exact attack work;
- per-attack articulation payload;
- arm/readout/power trajectories;
- warm-service receipt propagation.

Stop conditions:

- one body across attacks and bars;
- no per-note body reconstruction;
- all physical sources and edits receipted;
- window rendering equals full-render slice under declared state snapshots.

### JM6 — radiator, propagation, and feedback

- radiator modes and directivity;
- stereo/FOA/HOA/HRTF observations;
- causal propagation and Doppler;
- acoustic return to support/strings;
- delivery hardening.

Stop conditions:

- FOA is the exact order-1 HOA limit;
- stationary geometry produces no Doppler;
- feedback zero removes return-path work exactly;
- decoded outputs obey the delivery contract;
- raw Ambisonics never masquerades as direct speaker feeds.

### JM7 — calibration and musical validation

Fit one parameter set across a factorial reference corpus:

```text
six strings
several frets
several pluck positions
at least six attack-energy levels
single notes, dyads, and triads
open, muted, and behind-bridge attacks
arm gestures
lead/rhythm and bridge/both/neck routes
clean, edge, and sustained reference behaviors
several radiator/microphone angles
```

Hold out notes, chords, dynamics, and gestures. Promotion requires generalization, not one-note waveform matching.

## 15. Experiment matrix

### Mechanics

- afterlength/speaking energy ratio by note and attack position;
- bridge rock versus slip phase portrait;
- tremolo position, tension, pitch, and return error;
- arm work versus string/support energy change;
- lock engagement dispositions.

### Readout

- harmonic-null location versus aperture position/width;
- bridge/neck cross-note generalization;
- polarization response;
- chime-readout isolation;
- selector/circuit state-transport residual.

### Power body

Sweep physical attack work and measure:

```text
output energy
harmonic growth
intermodulation energy
crest factor
50 ms / 100 ms / 1 s density
reservoir minimum and recovery
modal occupancy
closure residual
```

### Chords

Compare mono, grouped, hex, and woven merge:

```text
note audibility
harmonic/intermodulation ratio
roughness
path dependence
level and energy match
```

### Memory

Alternate loud and soft attacks. Measure the soft attack after:

```text
silence
a loud single note
a loud chord
a tremolo gesture
near-feedback sustain
```

### Feedback and space

Sweep note, pose, distance, orientation, room, reservoir, and feedback gain. Record:

```text
instability threshold
selected mode/harmonic
growth rate
bounded steady amplitude
radiator mode occupancy
feedback exchange work
listener-field result
```

## 16. No-cheating acceptance tests

The promoted instrument must pass these ablations:

```text
remove afterlength
  -> behind-bridge chime and its feedback contribution disappear

move aperture
  -> note-dependent harmonic nulls move; one fixed EQ cannot reproduce all cases

power supply = 0
  -> no active sustain, sag, compression, or net active work

nonlinearity = 0
  -> no powered-body harmonic generation

feedback = 0
  -> no acoustic self-excitation

merge mono -> hex
  -> intermodulation changes while attack count, source work, and output level are controlled

arm path = constant
  -> no tremolo retune work or motion sidebands

radiator order = 1
  -> exact FOA compatibility
```

Forbidden evidence:

- a postprocessed render used to prove native behavior;
- per-example normalization hiding output differences;
- one filter fit per pitch;
- parameter retuning for every reference note;
- a neural residual carrying the recognizable profile;
- a speaker-unsafe raw research artifact presented for audition.

## 17. Performance requirements

The first scalar implementation is the oracle. Optimize only after it is green and profiled.

Hard requirements:

```text
no allocation in next-sample / next-frame kernels
fixed-capacity six-course and mode state
monotone attack, control, and topology cursors
no score-duration excitation arrays
shared physical render for stereo/HOA/binaural targets
content-addressed immutable profile data
```

Benchmark against the current persistent Weave path:

```text
JM1 passive mechanics
JM4 powered body
JM6 spatial/feedback
```

Report:

```text
wall time and p95
real-time factor
cycles/sample/course
allocations
peak resident memory
physical render passes
spatial target fan-out cost
closure observer cost
```

Relative release targets are set only after the scalar profiler exists. SIMD, Rayon, and structure-of-arrays work must preserve the scalar receipt and zero limits.

## 18. Initial profile status

`profiles/ariadne-jazzmaster-v0.json` is an **uncalibrated design seed**.

Evidence labels mean:

```text
manufacturer       externally specified geometry/string set
patent             mechanism topology described in the patent
converted          exact unit conversion
physics-derived    follows from declared geometry/equations
fit                intended to be estimated from data
 design-hypothesis  musically motivated starting value only
```

No `fit` or `design-hypothesis` field may be described as a measured Jazzmaster property.

## 19. Product language

Defensible:

> Ariadne Jazzmaster is a powered digital string body inspired by the mechanics and performance logic of the offset Jazzmaster. Six persistent strings, a rocking bridge, resonant afterlengths, a shared floating tremolo, finite-aperture string readouts, an energy-limited nonlinear body, and a spatial radiator form one instrument. It reaches familiar clean, broken-up, sustaining, feedback-rich electric-guitar regions without simulating an amplifier circuit or applying those behaviors as a chain of effects.

Withheld until experiments:

- indistinguishable from a vintage Jazzmaster rig;
- circuit-accurate;
- physically exact to one real instrument;
- globally passive under active drive;
- protected/topological;
- speaker-safe without a named delivery or playback profile;
- perceptually superior to existing guitar models.
