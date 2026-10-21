"""Trombone anatomy compiler: a trombone-shaped object as an Ariadne world plan.

The reference half of the Ovillo trombone described in ``TROMBONE-WORLD.md``.
It derives, from declared geometry, a canonical ``ariadne-world-plan-v2`` with
two persistent bodies and one reciprocal edge:

* ``lips``: the player's lips as the body's compliant aperture (a pressure-driven
  valve: mouth pressure is the source wave, the opening modulates the flow it
  lets through, and the lips' motion sweeps volume), sitting on one field mode,
  the mouthpiece cup. Its configuration path is **lip tension**: the aperture's
  natural frequency runs from slack (amount 0) to taut (amount 1), so moving the
  lips is lawful transport with a work receipt.
* ``bore``: the 24 lowest input-impedance resonances of a tenor-trombone-shaped
  bore (leadpipe, slide, crook, bell section, Bessel flare), computed by
  plane-wave transfer matrices with visco-thermal wall loss and an unflanged
  radiation load at the bell. Its configuration path is the **slide**: first
  position (amount 0) to seventh (amount 1). Wall loss becomes each mode's T60
  (internal loss); the bell's radiation becomes the body's radiation direction
  and fraction (booked as radiation, rising with frequency).
* ``throat``: the mouthpiece throat, a lossless Givens exchange between the cup
  mode's momentum and the bore's momentum projected on its input (each mode
  weighted by the square root of its modal input impedance).

Claim discipline
----------------
The object is a *trombone-shaped Ariadne body*, not a model of a particular
trombone. Every number is labelled ``manufacturer`` (published nominal values of
the instrument class), ``design-hypothesis`` (plausible choices nobody measured
for this object), ``fit`` (tuned so it plays and its partials sit near the B-flat
harmonic series), or ``inherited`` (the canonical aperture law's own
coefficients, ``mus-ariadne/src/ir.rs`` ``brass_compliant_aperture``). Limits are
listed in the receipt.
"""

from __future__ import annotations

import argparse
import json
import math
from dataclasses import asdict, dataclass
from hashlib import sha256
from pathlib import Path

import numpy as np

MODE_COUNT = 24          # mus-ariadne ir::MODE_COUNT
TRANSPORT_PAIRS = 12
SPACE_COURSES = 8
PLAN_SCHEMA = "ariadne-world-plan-v2"
SPEED_OF_SOUND = 343.0   # m/s, 20 C
AIR_DENSITY = 1.204      # kg/m^3
AIR_VISCOSITY = 1.81e-5  # Pa s
GAMMA = 1.402
PRANDTL = 0.71
BB1 = 440.0 * 2.0 ** ((34 - 69) / 12.0)   # B-flat 1, 58.27 Hz: the tenor trombone's nominal fundamental


@dataclass(frozen=True)
class TromboneGeometry:
    """Declared anatomy, SI units. Input (mouthpiece shank) end first."""

    bore_diameter: float = 0.01334        # manufacturer: 0.525 in medium tenor bore (published class nominal)
    bell_diameter: float = 0.2096         # manufacturer: 8.25 in bell (published class nominal)
    leadpipe_length: float = 0.24         # design-hypothesis
    leadpipe_entry: float = 0.0092        # design-hypothesis: shank/venturi diameter
    slide_leg: float = 0.52               # design-hypothesis: each slide leg in first position
    crook_length: float = 0.15            # design-hypothesis
    tail_length: float = 0.201            # fit: gooseneck + tuning slide + bell tail (sets the overall pitch)
    tail_exit: float = 0.0145             # design-hypothesis: bell-tail diameter where the flare starts
    flare: float = 1.406                  # fit: Bessel-horn exponent, r(s) = r_mouth (s0/(s+s0))^flare
    flare_s0: float = 0.194               # fit: the horn scale at the mouth (m); sets the flare length
    grid: float = 0.004                   # staircase segment length (m)
    # the mouthpiece and lips (lips body)
    cup_hz: float = 600.0                 # design-hypothesis: cup/throat (popping) resonance
    cup_t60: float = 0.05                 # design-hypothesis
    throat_rate_hz: float = 2000.0        # fit: cup <-> bore exchange rate
    throat_tilt: float = 1.0              # fit: bore mode n joins the throat with weight sqrt(A_n / f_n^tilt)
    lip_slack_hz: float = 30.0            # design-hypothesis: lip tension range
    lip_taut_hz: float = 900.0
    lip_rest: float = -0.20               # fit: lips' rest offset (sets the sweep's linear lip-bore coupling)
    lip_gap: float = 0.0005               # fit: rest opening = opening_bias + lip_rest (nearly closed: they close each cycle)
    lip_damping_slack: float = 300.0      # fit: lip loss per second at slack tension (lips are heavily damped)
    lip_damping_taut: float = 40.0        # fit: ... and at taut; linear in between, as the path interpolates
    lip_quartic: float = 4.1e6            # inherited
    lip_asymmetry: float = 0.028          # inherited
    mouth_to_lip_hz: float = 5.0          # fit: mouth pressure's push on the lips
    mouth_to_cup_hz: float = 3000.0       # fit: flow through the opening into the cup
    sweep_hz: float = 1000.0              # fit: volume the moving lips sweep into the cup


EVIDENCE = {
    "manufacturer": ["bore_diameter", "bell_diameter"],
    "fit": ["tail_length", "flare", "flare_s0", "throat_rate_hz", "throat_tilt", "lip_rest", "lip_gap", "lip_damping_slack", "lip_damping_taut",
            "mouth_to_lip_hz", "mouth_to_cup_hz", "sweep_hz", "slide extensions (solved, not chosen)"],
    "design_hypothesis": ["leadpipe_length", "leadpipe_entry", "slide_leg", "crook_length", "tail_exit", "cup_hz", "cup_t60", "lip_slack_hz", "lip_taut_hz",
                          "unflanged radiation load (Norris-Sheng) at the bell"],
    "inherited": ["lip_quartic", "lip_asymmetry (mus-ariadne ir.rs brass_compliant_aperture)"],
}

LIMITS = [
    "plane waves in a staircase of short cylinders: no higher-order modes, the flare's curved wavefront ignored",
    "the bell radiates as an unflanged pipe end (Norris-Sheng), no directivity",
    "24 bore resonances, so nothing above the 24th (about 1.5 kHz in first position, 1.05 kHz in seventh)",
    "the slide path interpolates each resonance in log frequency between first and seventh position; "
    "receipt field slide.path_error_cents gives how far that is from the computed resonances at 2-6",
    "the mouthpiece is one cup mode and one throat exchange, not a resolved cup and backbore",
    "linear walls and air: no wave steepening (the 'brassy' shock of real fortissimo) inside the bore",
    "no room: the space ports are zero, so the World is heard dry",
]


def _normalized(v) -> list[float]:
    v = np.asarray(v, float)
    n = float(np.linalg.norm(v))
    if not math.isfinite(n) or n < 1e-12:
        raise ValueError("degenerate direction")
    return [float(x) for x in v / n]


# ---------------------------------------------------------------- the bore


def bore_profile(g: TromboneGeometry, extension: float = 0.0) -> list[tuple[float, float]]:
    """(length, radius) segments from the leadpipe entry to the bell mouth.
    ``extension`` lengthens each slide leg (m), so the tube grows by twice it."""
    seg: list[tuple[float, float]] = []

    def add(length, r0, r1):
        n = max(1, int(math.ceil(length / g.grid)))
        for i in range(n):
            x = (i + 0.5) / n
            seg.append((length / n, r0 + (r1 - r0) * x))

    rb = g.bore_diameter / 2
    add(g.leadpipe_length, g.leadpipe_entry / 2, rb)
    add(g.slide_leg + extension, rb, rb)
    add(g.crook_length, rb, rb)
    add(g.slide_leg + extension, rb, rb)
    add(g.tail_length, rb, g.tail_exit / 2)
    for length, r in _bell(g):
        seg.append((length, r))
    return seg


def bell_length(g: TromboneGeometry) -> float:
    """Where the Bessel flare r(s) = r_mouth (s0/(s+s0))^flare narrows to the tail exit."""
    rt, rm = g.tail_exit / 2, g.bell_diameter / 2
    return g.flare_s0 * ((rm / rt) ** (1 / g.flare) - 1)


def _bell(g: TromboneGeometry):
    rm = g.bell_diameter / 2
    length = bell_length(g)
    n = int(math.ceil(length / g.grid))
    for i in range(n):
        s = length * (1 - (i + 0.5) / n)        # distance from the mouth
        yield length / n, rm * (g.flare_s0 / (s + g.flare_s0)) ** g.flare


def input_impedance(seg, f: np.ndarray, walls: bool = True, radiation: str = "full") -> np.ndarray:
    """Plane-wave transfer matrices; visco-thermal wall loss (Kirchhoff boundary-layer
    attenuation); Norris-Sheng unflanged radiation load at the mouth."""
    w = 2 * np.pi * f
    k = w / SPEED_OF_SOUND
    A = np.ones_like(f, complex)
    B = np.zeros_like(f, complex)
    C = np.zeros_like(f, complex)
    D = np.ones_like(f, complex)
    for length, a in seg:
        zc = AIR_DENSITY * SPEED_OF_SOUND / (math.pi * a * a)
        if walls:
            alpha = (np.sqrt(w * AIR_VISCOSITY / (2 * AIR_DENSITY)) / (a * SPEED_OF_SOUND)
                     * (1 + (GAMMA - 1) / math.sqrt(PRANDTL)))
            G = 1j * k + (1 + 1j) * alpha
        else:
            G = 1j * k
        ch, sh = np.cosh(G * length), np.sinh(G * length)
        A, B, C, D = A * ch + B * sh / zc, A * zc * sh + B * ch, C * ch + D * sh / zc, C * zc * sh + D * ch
    a = seg[-1][1]
    zc = AIR_DENSITY * SPEED_OF_SOUND / (math.pi * a * a)
    ka = k * a
    mag = (1 + 0.2 * ka - 0.084 * ka**2) / (1 + 0.2 * ka + (0.5 - 0.084) * ka**2)
    delta = a * (0.6133 + 0.027 * ka**2) / (1 + 0.19 * ka**2)
    R = -mag * np.exp(-2j * k * delta)
    zr = zc * (1 + R) / (1 - R)
    if radiation == "reactive":
        zr = 1j * zr.imag
    return (A * zr + B) / (C * zr + D)


def _peaks(f: np.ndarray, z: np.ndarray):
    """Local maxima of |Z|: frequency, height, Q from the half-power bandwidth."""
    m = np.abs(z)
    out = []
    df = f[1] - f[0]
    for i in range(1, len(f) - 1):
        if not (m[i] > m[i - 1] and m[i] >= m[i + 1]):
            continue
        y0, y1, y2 = np.log(m[i - 1 : i + 2])
        d = 0.5 * (y0 - y2) / (y0 - 2 * y1 + y2)
        fp = f[i] + d * df
        peak = math.exp(y1 - 0.25 * (y0 - y2) * d)
        half = peak / math.sqrt(2)
        lo, hi = i, i
        while lo > 0 and m[lo] > half:
            lo -= 1
        while hi < len(f) - 1 and m[hi] > half:
            hi += 1
        if m[lo] > half or m[hi] > half:
            # overlapping peaks: Q from the Lorentzian curvature of 1/|Z|^2 at the peak,
            # 1/|Z|^2 ~ (1 + (2Q (f - fp)/fp)^2) / peak^2
            j = max(2, min(len(f) - 3, i))
            y = 1.0 / m[j - 2 : j + 3] ** 2
            curv = (-y[0] + 16 * y[1] - 30 * y[2] + 16 * y[3] - y[4]) / (12 * df * df)
            q = 0.5 * fp * math.sqrt(max(curv, 1e-300) * peak * peak / 2.0)
        else:
            f_lo = f[lo] + (half - m[lo]) / (m[lo + 1] - m[lo]) * df
            f_hi = f[hi - 1] + (half - m[hi - 1]) / (m[hi] - m[hi - 1]) * df
            q = fp / (f_hi - f_lo)
        out.append((fp, peak, q))
    return out


def bore_modes(g: TromboneGeometry, extension: float = 0.0, count: int = MODE_COUNT):
    """The bore's resonances at one slide extension.

    Returns dict of arrays: frequency (Hz), q_wall (walls + the bell's reactive
    load), q_rad (lossless walls + the full bell load), q_total, and the modal
    input weight A_n = |Z|_peak * omega_n / Q_n (Z ~ A j w / (w_n^2 - w^2 + j w w_n/Q))."""
    seg = bore_profile(g, extension)
    f = np.arange(8.0, 2600.0, 0.25)
    full = _peaks(f, input_impedance(seg, f))[:count]
    wall = _peaks(f, input_impedance(seg, f, radiation="reactive"))[:count]
    rad = _peaks(f, input_impedance(seg, f, walls=False))[:count]
    if min(len(full), len(wall), len(rad)) < count:
        raise ValueError(f"only {len(full)} bore resonances below 2.6 kHz")
    freq = np.array([p[0] for p in full])
    q_total = np.array([p[2] for p in full])
    q_wall = np.array([p[2] for p in wall])
    q_rad = np.array([p[2] for p in rad])
    weight = np.array([p[1] * 2 * math.pi * p[0] / p[2] for p in full])
    return dict(frequency=freq, q_total=q_total, q_wall=q_wall, q_rad=q_rad, input_weight=weight,
                length=float(sum(s[0] for s in seg)))


def _low_resonances(g: TromboneGeometry, extension: float, count: int = 8) -> np.ndarray:
    """Just the lowest resonance frequencies (coarse grid, parabolic peaks): for solving the slide."""
    f = np.arange(8.0, 620.0, 0.5)
    return np.array([p[0] for p in _peaks(f, input_impedance(bore_profile(g, extension), f))[:count]])


def slide_extensions(g: TromboneGeometry, positions: int = 7, partials=range(2, 9)):
    """Each position k lowers the mean (log) of resonances 2..8 by k-1 equal-tempered semitones
    from first position: solve each leg's extension by secant iteration."""
    idx = [p - 1 for p in partials]

    def mean_log(e):
        return float(np.mean(np.log(_low_resonances(g, e)[idx])))

    base = mean_log(0.0)
    out = [0.0]
    for k in range(2, positions + 1):
        target = base - (k - 1) * math.log(2) / 12
        e0, e1 = out[-1], out[-1] + 0.09
        v0, v1 = mean_log(e0) - target, mean_log(e1) - target
        for _ in range(12):
            if abs(v1) < 1e-7 or v1 == v0:
                break
            e0, e1, v0 = e1, e1 - v1 * (e1 - e0) / (v1 - v0), v1
            v1 = mean_log(e1) - target
        out.append(e1)
    return out


# ---------------------------------------------------------------- the plan


def _inert_aperture() -> dict:
    return dict(natural_frequency_hz=500.0, quartic_stiffness=0.0, asymmetry=0.0,
                equilibrium_displacement=0.0, opening_bias=0.0, damping_per_second=50.0,
                source_to_aperture_rate_hz=0.0, source_to_field_rate_hz=0.0, sweep_rate_hz=0.0,
                active_rate_per_second=0.0, active_target_momentum=0.25)


def lip_aperture(g: TromboneGeometry, natural_hz: float, damping: float) -> dict:
    return dict(natural_frequency_hz=natural_hz, quartic_stiffness=g.lip_quartic, asymmetry=g.lip_asymmetry,
                equilibrium_displacement=g.lip_rest, opening_bias=g.lip_gap - g.lip_rest,
                damping_per_second=damping, source_to_aperture_rate_hz=g.mouth_to_lip_hz,
                source_to_field_rate_hz=g.mouth_to_cup_hz, sweep_rate_hz=g.sweep_hz,
                # no reservoir-paid activity: every joule the lips spend comes in as mouth pressure
                active_rate_per_second=0.0, active_target_momentum=0.334)


def _active_directions() -> list[list[float]]:
    # required to be independent by the plan validator; unused (active coupling is zero)
    x = np.linspace(0, 1, MODE_COUNT)
    return [_normalized(0.35 + 1.65 * x), _normalized(np.where(np.arange(MODE_COUNT) % 2 == 0, 1.0, -0.8)),
            _normalized(np.sin(math.pi * (2 * x + 0.15)))]


def _quiet_transport() -> dict:
    return dict(pairs=[[i, i + 1] for i in range(0, 2 * TRANSPORT_PAIRS, 2)], weights=[0.0] * TRANSPORT_PAIRS)


def _no_space_port() -> dict:
    rows = []
    for course in range(SPACE_COURSES):
        d = np.zeros(MODE_COUNT)
        d[(3 * course) % MODE_COUNT] = 1.0
        rows.append(_normalized(d))
    return dict(body_directions=rows, coupling_weights=[0.0] * SPACE_COURSES)


def _room() -> dict:
    return dict(delays_seconds=[0.0061, 0.0093, 0.0127, 0.0171, 0.0219, 0.0283, 0.0397, 0.0613],
                scattering_direction=_normalized([1.0, -0.73, 0.88, -0.61, 0.79, -0.52, 0.69, -0.44]),
                radiation_left=_normalized([1.0, 0.62, 0.18, -0.31, -0.70, -0.54, 0.08, 0.48]),
                radiation_right=_normalized([0.17, -0.51, -0.72, -0.24, 0.36, 0.61, 0.31, -0.22]))


def _configuration(freq, t60, aperture, radiation) -> dict:
    return dict(metric=dict(frequency_hz=[float(x) for x in freq], t60_seconds=[float(x) for x in t60]),
                aperture=aperture, radiation_direction=radiation)


def bore_configuration(modes: dict, sample_rate: float):
    """Ariadne bore configuration from computed resonances.

    Wall loss -> T60 (amplitude rate sigma_wall = pi f / Q_wall, T60 = ln(1000)/sigma).
    Radiation -> one rank: radiate() removes fraction phi of the momentum projection on r
    each sample, so mode n loses amplitude at sigma_rad = phi r_n^2 sr / 4 (for well
    separated modes). Hence r_n^2 ~ pi f_n / Q_rad,n and phi = 4 sum(sigma_rad) / sr."""
    f = modes["frequency"]
    sigma_wall = math.pi * f / modes["q_wall"]
    sigma_rad = math.pi * f / modes["q_rad"]
    t60 = math.log(1000.0) / sigma_wall
    fraction = 4.0 * float(sigma_rad.sum()) / sample_rate
    if not 0.0 < fraction < 0.25:
        raise ValueError(f"radiation fraction {fraction} leaves the small-angle regime")
    return dict(t60=t60, radiation=_normalized(np.sqrt(sigma_rad)), fraction=fraction,
                sigma_wall=sigma_wall, sigma_rad=sigma_rad)


def trombone_plan(g: TromboneGeometry, sample_rate: float = 48_000.0):
    ext = slide_extensions(g)
    m1 = bore_modes(g, ext[0])
    m7 = bore_modes(g, ext[-1])
    c1 = bore_configuration(m1, sample_rate)
    c7 = bore_configuration(m7, sample_rate)
    inert = _inert_aperture()

    # path error: the World's log-interpolated slide against computed resonances at 2..6
    path_err = []
    ratio = np.log(m7["frequency"] / m1["frequency"])
    for k in range(1, 6):
        mk = bore_modes(g, ext[k])["frequency"]
        # amount that puts the mean of partials 2..8 on k semitones
        amount = k / 6
        world = m1["frequency"] * np.exp(amount * ratio)
        path_err.append([float(1200 * math.log2(world[i] / mk[i])) for i in range(MODE_COUNT)])

    bore = dict(
        id="bore",
        body=dict(
            anatomy_name="ovillo-trombone-bore-v0",
            configuration_path=dict(
                coordinate="slide",
                left_name="first_position",
                right_name="seventh_position",
                left=_configuration(m1["frequency"], c1["t60"], inert, c1["radiation"]),
                right=_configuration(m7["frequency"], c7["t60"], inert, c7["radiation"]),
                initial_amount=0.0,
            ),
            # the bore has no aperture of its own; its source direction is never driven
            source_direction=_normalized(np.ones(MODE_COUNT)),
            active_directions=_active_directions(),
            transport=_quiet_transport(),
            reservoir=dict(capacity=1e-6),
        ),
        space_port=_no_space_port(),
        pose=dict(position=[0.0, 0.0, 0.0], orientation=[0.0, 0.0, 0.0, 1.0]),
    )

    e0 = np.zeros(MODE_COUNT)
    e0[0] = 1.0
    cup_freq = np.array([g.cup_hz] + [1000.0 + 100.0 * i for i in range(1, MODE_COUNT)])
    cup_t60 = np.array([g.cup_t60] + [1.0] * (MODE_COUNT - 1))
    lips = dict(
        id="lips",
        body=dict(
            anatomy_name="ovillo-trombone-lips-v0",
            configuration_path=dict(
                coordinate="lip_tension",
                left_name="slack",
                right_name="taut",
                left=_configuration(cup_freq, cup_t60, lip_aperture(g, g.lip_slack_hz, g.lip_damping_slack), _normalized(e0)),
                right=_configuration(cup_freq, cup_t60, lip_aperture(g, g.lip_taut_hz, g.lip_damping_taut), _normalized(e0)),
                initial_amount=0.0,
            ),
            source_direction=_normalized(e0),     # the lips open into the cup
            active_directions=_active_directions(),
            transport=_quiet_transport(),
            reservoir=dict(capacity=1e-6),
        ),
        space_port=_no_space_port(),
        pose=dict(position=[0.0, 0.0, 0.0], orientation=[0.0, 0.0, 0.0, 1.0]),
    )

    # the throat: cup momentum <-> the bore's input. In energy-normalized coordinates the bore's
    # input flow is sum_n sqrt(A_n) P_n. The World can only exchange momentum with momentum, and
    # through that exchange the lips see bore peak n grow ~ f_n (H_n ~ c^2 b_n^2 w_n^2 / (sigma_n
    # (w_c^2 - w^2)^2)); a real mouthpiece passes low frequencies freely. The tilt divides it back
    # out (a fit, like steel-string's bridge_rise). First-position weights: the edge is not on a path.
    throat_weights = np.sqrt(m1["input_weight"] * m1["frequency"] ** (-g.throat_tilt))
    throat = dict(id="throat", left_body="lips", right_body="bore", left_direction=_normalized(e0),
                  right_direction=_normalized(throat_weights), rate_hz=g.throat_rate_hz, weight=1.0)

    plan = dict(schema_version=PLAN_SCHEMA, world_name="ovillo-trombone-v0", internal_sample_rate_hz=sample_rate,
                bodies=[lips, bore], shared_space=_room(), direct_couplings=[throat])

    receipt = dict(
        schema="ovillo.trombone-anatomy-receipt.v0",
        geometry=asdict(g),
        evidence=EVIDENCE,
        limits=LIMITS,
        bore=dict(
            length_first_m=m1["length"],
            length_seventh_m=m7["length"],
            frequency_first=[float(x) for x in m1["frequency"]],
            frequency_seventh=[float(x) for x in m7["frequency"]],
            q_total_first=[float(x) for x in m1["q_total"]],
            q_wall_first=[float(x) for x in m1["q_wall"]],
            q_rad_first=[float(x) for x in m1["q_rad"]],
            q_rad_seventh=[float(x) for x in m7["q_rad"]],
            input_weight_first=[float(x) for x in m1["input_weight"]],
            radiation_fraction_first=c1["fraction"],
            radiation_fraction_seventh=c7["fraction"],
            radiated_share_first=[float(x) for x in c1["sigma_rad"] / (c1["sigma_rad"] + c1["sigma_wall"])],
            bb_series_cents_first=[float(1200 * math.log2(m1["frequency"][n - 1] / (n * BB1))) for n in range(2, 9)],
        ),
        slide=dict(
            leg_extension_m=ext,
            note="position k lowers the mean of resonances 2..8 by k-1 semitones; amount = (k-1)/6",
            path_error_cents=path_err,
            path_error_max_cents=float(max(abs(x) for row in path_err for x in row[:12])),
        ),
        radiation_fraction=dict(first=c1["fraction"], seventh=c7["fraction"],
                                note="the bore's radiation control takes the mean; the direction moves with the slide"),
    )
    return plan, receipt


def canonical_digest(obj) -> str:
    return "sha256:" + sha256(json.dumps(obj, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("-o", "--out", default="profiles/ariadne-trombone-v0.plan.json")
    ap.add_argument("--sample-rate", type=float, default=48_000.0)
    ap.add_argument("--set", action="append", default=[], help="geometry override name=value")
    args = ap.parse_args(argv)
    over = {}
    for kv in args.set:
        k, v = kv.split("=", 1)
        over[k] = float(v)
    g = TromboneGeometry(**over)
    plan, receipt = trombone_plan(g, args.sample_rate)
    receipt["plan_digest"] = canonical_digest(plan)
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(plan, indent=1) + "\n")
    out.with_name(out.name.replace(".plan.json", ".receipt.json")).write_text(json.dumps(receipt, indent=1) + "\n")
    b = receipt["bore"]
    print(f"bore {b['length_first_m']:.3f} m -> {b['length_seventh_m']:.3f} m; "
          f"legs +{', '.join(f'{100*e:.1f}' for e in receipt['slide']['leg_extension_m'][1:])} cm")
    print("first position Hz:", " ".join(f"{x:.1f}" for x in b["frequency_first"][:10]))
    print("vs n*Bb1 (n=2..8) cents:", " ".join(f"{x:+.0f}" for x in b["bb_series_cents_first"]))
    print(f"radiation fraction {b['radiation_fraction_first']:.4f} / {b['radiation_fraction_seventh']:.4f}; "
          f"path error max {receipt['slide']['path_error_max_cents']:.1f} c (modes 1-12)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
