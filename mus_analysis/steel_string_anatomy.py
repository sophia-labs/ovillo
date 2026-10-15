"""Steel-string guitar anatomy compiler: a guitar-shaped object as an Ariadne world plan.

This is the reference (oracle) half of the steel-string instrument described in
``SPEC-ARIADNE-STEEL-STRING.md``. It derives, from declared geometry and
materials, a canonical ``ariadne-world-plan-v2`` with seven persistent bodies:

* six string bodies, one per course, each 24 partials of a stiff string whose
  configuration path runs from the open string (amount 0) to the twelfth fret
  (amount 1), so stopping the string is lawful transport, not a new voice;
* one box body: the 24 lowest modes of an orthotropic spruce top on a
  dreadnought outline (soundhole cut out, bridge mass lumped on), coupled to the
  Helmholtz air of the cavity through the top's volume displacement;

and six reciprocal bridge edges (string bridge-force direction <-> the box's
displacement at the saddle). The plan is data: ``mus render`` loads it by path
and plays it from an annotated-tab score.

Claim discipline
----------------
The object is a *guitar-shaped Ariadne body*, not a model of a particular
guitar. Geometry and string data are labelled ``manufacturer`` (published
nominal values), ``design-hypothesis`` (plausible choices nobody measured), or
``fit`` (tuned so the lowest coupled modes land where steel-string literature
puts them: A0 near 100 Hz, T(1,1) near 190 Hz). The back plate and ribs are
rigid in v0, the top is clamped at the rim and at the soundhole rosette, and
bracing is folded into effective stiffnesses. Every one of these is a declared
limit, listed in the derivation receipt.
"""

from __future__ import annotations

import argparse
import json
import math
from dataclasses import asdict, dataclass, field
from hashlib import sha256
from pathlib import Path

import numpy as np

try:
    from scipy import linalg, sparse
except ImportError as exc:  # pragma: no cover - only without the audio extra
    raise ImportError("steel-string anatomy needs the project 'audio' extra (numpy, scipy)") from exc

MODE_COUNT = 24          # mus-ariadne ir::MODE_COUNT
ACTIVE_RANKS = 3
TRANSPORT_PAIRS = 12
SPACE_COURSES = 8
PLAN_SCHEMA = "ariadne-world-plan-v2"
SPEED_OF_SOUND = 343.0
AIR_DENSITY = 1.204
STANDARD_TUNING = (40, 45, 50, 55, 59, 64)   # E2 A2 D3 G3 B3 E4, low to high (string 6 .. 1)


@dataclass(frozen=True)
class StringCourse:
    name: str
    open_midi: int
    gauge_in: float          # outer diameter
    core_in: float           # steel core (equals gauge for plain strings)
    wound: bool


@dataclass(frozen=True)
class SteelStringGeometry:
    """Declared anatomy. Units SI unless the name says otherwise."""

    scale_length: float = 0.645                  # manufacturer: 25.4 in dreadnought scale
    body_length: float = 0.505                   # manufacturer: dreadnought nominal
    lower_bout: float = 0.395
    waist: float = 0.275
    upper_bout: float = 0.292
    depth: float = 0.112                         # mean side depth
    soundhole_radius: float = 0.050
    soundhole_center: float = 0.365              # from the tail block, along the body
    bridge_y: float = 0.218                      # saddle line: 14th fret at the body joint puts it scale - 0.358 m below the neck end
    bridge_halfwidth: float = 0.035              # span of the saddle contacting the top
    bridge_mass: float = 0.032                   # design-hypothesis: bridge + saddle + pins, kg
    top_thickness: float = 0.0027
    top_density: float = 430.0                   # Sitka spruce
    e_long: float = 11.6e9                       # along the grain (body axis)
    e_cross: float = 0.86e9
    g_shear: float = 0.72e9
    nu_long: float = 0.37
    brace_long: float = 5.9                      # fit: X-bracing folded into effective stiffness
    brace_cross: float = 14.8                    # fit
    grid: float = 0.0085                         # finite-difference spacing
    top_q: float = 55.0                          # design-hypothesis internal Q of the top
    air_q: float = 38.0                          # design-hypothesis wall/visco-thermal Q of A0
    coincidence_hz: float = 4000.0               # design-hypothesis: bending waves meet the speed of sound
    multipole: float = 0.12                      # design-hypothesis: edge/corner radiation vs the breathing mode
    bridge_tilt: float = 0.0                     # fit: saddle participation x (f/200 Hz)^tilt in wave coordinates
    bridge_rise: float = 2.0                     # fit: a box body's bridge rate grows as (mean mode Hz / 1.2 kHz)^rise
    strings: tuple[StringCourse, ...] = (        # manufacturer: phosphor-bronze light .012-.053
        StringCourse("E2", 40, 0.053, 0.022, True),
        StringCourse("A2", 45, 0.042, 0.019, True),
        StringCourse("D3", 50, 0.032, 0.016, True),
        StringCourse("G3", 55, 0.024, 0.013, True),
        StringCourse("B3", 59, 0.016, 0.016, False),
        StringCourse("E4", 64, 0.012, 0.012, False),
    )


def _midi_hz(midi: float) -> float:
    return 440.0 * 2.0 ** ((midi - 69.0) / 12.0)


def _normalized(v: np.ndarray) -> list[float]:
    n = float(np.linalg.norm(v))
    if not math.isfinite(n) or n < 1e-12:
        raise ValueError("degenerate direction")
    return [float(x) for x in v / n]


# ---------------------------------------------------------------- the box


def outline_halfwidth(g: SteelStringGeometry, y: np.ndarray) -> np.ndarray:
    """Half-width of the top at distance y from the tail block: two bouts and a waist."""
    lower = 0.5 * g.lower_bout * np.sqrt(np.clip(1 - ((y - 0.155) / 0.158) ** 2, 0, None))
    upper = 0.5 * g.upper_bout * np.sqrt(np.clip(1 - ((y - 0.392) / 0.115) ** 2, 0, None))
    waist = 0.5 * g.waist * np.exp(-((y - 0.292) / 0.085) ** 2)
    ends = np.sqrt(np.clip(y / 0.012, 0, 1) * np.clip((g.body_length - y) / 0.012, 0, 1))
    return np.maximum.reduce([lower, upper, waist]) * ends


def top_modes(g: SteelStringGeometry, count: int):
    """Orthotropic Kirchhoff top, clamped at the rim and the rosette. Returns modes M-normalized."""
    h = g.grid
    xs = np.arange(-g.lower_bout / 2, g.lower_bout / 2 + h, h)
    ys = np.arange(0, g.body_length + h, h)
    X, Y = np.meshgrid(xs, ys)
    inside = np.abs(X) < outline_halfwidth(g, Y) - 0.5 * h
    inside &= (X ** 2 + (Y - g.soundhole_center) ** 2) > (g.soundhole_radius + 0.004) ** 2
    index = -np.ones(X.shape, int)
    index[inside] = np.arange(inside.sum())
    n = int(inside.sum())
    rows, cols = np.nonzero(inside)

    def second_difference(dr: int, dc: int) -> sparse.csr_matrix:
        data, ri, ci = [], [], []
        for k, (r, c) in enumerate(zip(rows, cols)):
            for off, w in ((-1, 1.0), (0, -2.0), (1, 1.0)):
                rr, cc = r + off * dr, c + off * dc
                j = index[rr, cc] if 0 <= rr < X.shape[0] and 0 <= cc < X.shape[1] else -1
                if j >= 0:                                          # outside the top: clamped (w = 0)
                    data.append(w / h ** 2); ri.append(k); ci.append(j)
        return sparse.csr_matrix((data, (ri, ci)), shape=(n, n))

    def cross_difference() -> sparse.csr_matrix:
        data, ri, ci = [], [], []
        for k, (r, c) in enumerate(zip(rows, cols)):
            for dr, dc, w in ((1, 1, 1), (1, -1, -1), (-1, 1, -1), (-1, -1, 1)):
                rr, cc = r + dr, c + dc
                j = index[rr, cc] if 0 <= rr < X.shape[0] and 0 <= cc < X.shape[1] else -1
                if j >= 0:
                    data.append(w / (4 * h * h)); ri.append(k); ci.append(j)
        return sparse.csr_matrix((data, (ri, ci)), shape=(n, n))

    dxx, dyy, dxy = second_difference(0, 1), second_difference(1, 0), cross_difference()
    t = g.top_thickness
    nu_cross = g.nu_long * g.e_cross / g.e_long
    den = 12 * (1 - g.nu_long * nu_cross)
    d_y = g.brace_long * g.e_long * t ** 3 / den        # grain runs along the body (y)
    d_x = g.brace_cross * g.e_cross * t ** 3 / den
    d_1 = g.nu_long * d_x
    d_k = g.g_shear * t ** 3 / 12
    area = h * h
    stiffness = area * (d_x * (dxx.T @ dxx) + d_1 * (dxx.T @ dyy + dyy.T @ dxx)
                        + d_y * (dyy.T @ dyy) + 4 * d_k * (dxy.T @ dxy))
    mass = np.full(n, g.top_density * t * area)
    saddle = inside & (np.abs(Y - g.bridge_y) < 0.6 * h) & (np.abs(X) <= g.bridge_halfwidth)
    footprint = inside & (np.abs(Y - g.bridge_y) < 0.016) & (np.abs(X) <= 0.08)
    mass[index[footprint]] += g.bridge_mass / footprint.sum()
    w2, phi = linalg.eigh(stiffness.toarray(), np.diag(mass), subset_by_index=[0, count - 1])
    volume = phi.T @ np.full(n, area)                     # net volume displacement per mode
    at_saddle = phi[index[saddle]].mean(axis=0)
    top_area = float(inside.sum() * area + math.pi * g.soundhole_radius ** 2)
    points = np.column_stack([X[inside], Y[inside]])
    return w2, volume, at_saddle, top_area, n, phi, points


BOX_BODIES = 3          # the top's modes, 24 per Ariadne body: exact, since a linear plate's modes are orthogonal


def box_modes(g: SteelStringGeometry, count: int = BOX_BODIES * MODE_COUNT):
    """Top modes coupled to the Helmholtz air plug through the cavity's compressibility."""
    plate_count = count + 8
    w2, volume, at_saddle, top_area, points, phi, xy = top_modes(g, plate_count)
    hole_area = math.pi * g.soundhole_radius ** 2
    neck = g.top_thickness + 1.7 * g.soundhole_radius            # end-corrected plug length
    plug_mass = AIR_DENSITY * hole_area * neck
    cavity_volume = top_area * g.depth
    k_cavity = AIR_DENSITY * SPEED_OF_SOUND ** 2 / cavity_volume
    coupling = np.append(volume, hole_area)
    stiffness = np.diag(np.append(w2, 0.0)) + k_cavity * np.outer(coupling, coupling)
    mass = np.diag(np.append(np.ones(plate_count), plug_mass))
    lam, u = linalg.eigh(stiffness, mass)
    lam, u = lam[:count], u[:, :count]
    freq = np.sqrt(lam) / (2 * math.pi)
    saddle = u[:plate_count].T @ at_saddle                        # displacement at the saddle
    monopole = u[:plate_count].T @ volume + hole_area * u[plate_count]   # net radiating volume
    air_share = u[plate_count] ** 2 * plug_mass                   # kinetic share of the air plug
    # Internal (wood) loss only. The extra damping a real top shows at high frequency is radiation,
    # which the world books separately: each box body radiates at a rate that grows with its band.
    q = np.where(air_share > 0.5, g.air_q, g.top_q)
    t60 = 2.2 * q / freq
    helmholtz = math.sqrt(k_cavity * hole_area ** 2 / plug_mass) / (2 * math.pi)
    return dict(frequency_hz=freq, t60_seconds=t60, saddle=saddle, monopole=monopole,
                air_share=air_share, uncoupled_top_hz=np.sqrt(w2[:6]) / (2 * math.pi),
                top_shapes=phi @ u[:plate_count], top_points=xy,
                helmholtz_hz=helmholtz, cavity_volume_m3=cavity_volume, top_area_m2=top_area,
                grid_points=points)


# ---------------------------------------------------------------- the strings


def string_partials(g: SteelStringGeometry, course: StringCourse, length: float, f0: float):
    """Partials of a stiff steel string of the given speaking length."""
    rho_steel, e_steel = 7850.0, 200e9
    d = course.gauge_in * 0.0254
    core = course.core_in * 0.0254
    # linear density: a wound string's mass is roughly 0.82 of a solid rod of its outer diameter
    mu = rho_steel * math.pi * (d / 2) ** 2 * (0.82 if course.wound else 1.0)
    tension = mu * (2 * g.scale_length * _midi_hz(course.open_midi)) ** 2
    b = math.pi ** 3 * e_steel * core ** 4 / (64 * tension * length ** 2)
    n = np.arange(1, MODE_COUNT + 1)
    freq = n * f0 * np.sqrt(1 + b * n ** 2)
    sigma = 0.55 + 1.9e-7 * (2 * math.pi * freq) ** 2      # Chaigne-Askenfelt form; design-hypothesis
    return freq, 6.91 / sigma, tension, b, mu


def contact_aperture(hardness: float) -> dict:
    """The plucking contact: a fingertip (0) through a thumbpick (1)."""
    return dict(natural_frequency_hz=380.0 + 900.0 * hardness, quartic_stiffness=2.0e6,
                asymmetry=0.0, equilibrium_displacement=0.0, opening_bias=0.02,
                damping_per_second=60.0 - 30.0 * hardness, source_to_aperture_rate_hz=0.0,
                # the contact is a reciprocal port: while it is coupled it also absorbs the string,
                # so it is kept gentle (80 Hz) and the pluck pulse is scaled up to match
                source_to_field_rate_hz=80.0, sweep_rate_hz=0.0, active_rate_per_second=0.0,
                active_target_momentum=0.25)


def inert_aperture() -> dict:
    return dict(natural_frequency_hz=500.0, quartic_stiffness=0.0, asymmetry=0.0,
                equilibrium_displacement=0.0, opening_bias=0.0, damping_per_second=50.0,
                source_to_aperture_rate_hz=0.0, source_to_field_rate_hz=0.0, sweep_rate_hz=0.0,
                active_rate_per_second=0.0, active_target_momentum=0.25)


# ---------------------------------------------------------------- the plan


def _active_directions() -> list[list[float]]:
    x = np.linspace(0, 1, MODE_COUNT)
    return [_normalized(0.35 + 1.65 * x), _normalized(np.where(np.arange(MODE_COUNT) % 2 == 0, 1.0, -0.8)),
            _normalized(np.sin(math.pi * (2 * x + 0.15)))]


def _quiet_transport() -> dict:
    return dict(pairs=[[i, i + 1] for i in range(0, 2 * TRANSPORT_PAIRS, 2)], weights=[0.0] * TRANSPORT_PAIRS)


def _space_port(direction: list[float], weight: float) -> dict:
    d = np.array(direction)
    rows = []
    for course in range(SPACE_COURSES):
        rolled = np.roll(d, course) * (1.0 if course % 2 == 0 else -1.0)
        rows.append(_normalized(rolled + 0.15 * d))
    return dict(body_directions=rows, coupling_weights=[weight * 0.9 ** c for c in range(SPACE_COURSES)])


def _room() -> dict:
    return dict(delays_seconds=[0.0061, 0.0093, 0.0127, 0.0171, 0.0219, 0.0283, 0.0397, 0.0613],
                scattering_direction=_normalized(np.array([1.0, -0.73, 0.88, -0.61, 0.79, -0.52, 0.69, -0.44])),
                radiation_left=_normalized(np.array([1.0, 0.62, 0.18, -0.31, -0.70, -0.54, 0.08, 0.48])),
                radiation_right=_normalized(np.array([0.17, -0.51, -0.72, -0.24, 0.36, 0.61, 0.31, -0.22])))


@dataclass
class HandZone:
    """Where a hand meets the strings it plays: pluck point and contact hardness, plus a resting palm."""

    strings: tuple[int, ...]
    pos: float          # pluck point as a fraction of the speaking length from the bridge
    pick: float         # contact hardness, fingertip 0 .. thumbpick 1
    palm_mute: float = 0.0   # 0 = no palm, 1 = the palm sits right on the saddle


def steel_string_plan(g: SteelStringGeometry, zones: list[HandZone], sample_rate: float = 48_000.0,
                      bridge_rate_hz: float = 40.0):
    box = box_modes(g)
    zone_of = {s: z for z in zones for s in z.strings}
    bodies, edges, strings_receipt = [], [], []
    for number, course in zip(range(6, 0, -1), g.strings):
        zone = zone_of.get(number, HandZone((number,), 0.18, 0.4))
        f0 = _midi_hz(course.open_midi)
        open_f, open_t60, tension, b, mu = string_partials(g, course, g.scale_length, f0)
        twelfth_f, twelfth_t60, *_ = string_partials(g, course, g.scale_length / 2, 2 * f0)
        if zone.palm_mute > 0:                       # the resting palm: a lossy contact near the saddle
            extra = zone.palm_mute * (24.0 + 1.2e-5 * (2 * math.pi * open_f) ** 2)
            open_t60 = 6.91 / (6.91 / open_t60 + extra)
            extra12 = zone.palm_mute * (24.0 + 1.2e-5 * (2 * math.pi * twelfth_f) ** 2)
            twelfth_t60 = 6.91 / (6.91 / twelfth_t60 + extra12)
        n = np.arange(1, MODE_COUNT + 1)
        source = np.sin(math.pi * n * zone.pos) / n          # a released pluck: sin(n pi x) / n
        radiation = 1.0 / (1.0 + 0.02 * n ** 2)
        config = lambda f, t60: dict(metric=dict(frequency_hz=[float(x) for x in f], t60_seconds=[float(x) for x in t60]),
                                     aperture=contact_aperture(zone.pick), radiation_direction=_normalized(radiation))
        body_id = f"string/{number}"
        bodies.append(dict(id=body_id, body=dict(
            anatomy_name=f"steel-string-course-{course.name}",
            configuration_path=dict(coordinate="stopped_length", left_name="open", right_name="twelfth_fret",
                                    left=config(open_f, open_t60), right=config(twelfth_f, twelfth_t60),
                                    initial_amount=0.0),
            source_direction=_normalized(source), active_directions=_active_directions(),
            transport=_quiet_transport(), reservoir=dict(capacity=1.0e-6)),
            space_port=_space_port(_normalized(radiation), 0.05),
            pose=dict(position=[0.0, 0.0, 0.0], orientation=[0.0, 0.0, 0.0, 1.0])))
        # T dy/dx at the saddle. In energy-normalized wave coordinates every partial's slope there
        # carries the same weight (n from the slope cancels the n of omega), alternating in sign.
        bridge_force = np.where(n % 2 == 0, 1.0, -1.0)
        saddle = box["saddle"] * (box["frequency_hz"] / 200.0) ** g.bridge_tilt
        # The saddle's mobility rises across the plate's high band: a body holding higher modes
        # exchanges with the string faster (rank-1 coupling spreads thin over 24 x 24 pairs).
        band_rise = [max(1.0, float(box["frequency_hz"][k * MODE_COUNT:(k + 1) * MODE_COUNT].mean()) / 1200.0) ** g.bridge_rise
                     for k in range(BOX_BODIES)]
        saddle_total = float(np.linalg.norm(saddle))
        for k in range(BOX_BODIES):                   # one edge per box body, weighted by its share of the saddle
            part = saddle[k * MODE_COUNT:(k + 1) * MODE_COUNT]
            edges.append(dict(id=f"bridge/{number}/{k}", left_body=body_id, right_body=f"box/{k}",
                              left_direction=_normalized(bridge_force), right_direction=_normalized(part),
                              rate_hz=bridge_rate_hz * band_rise[k], weight=float(np.linalg.norm(part) / saddle_total)))
        strings_receipt.append(dict(string=number, course=course.name, open_hz=round(f0, 3),
                                    tension_n=round(tension, 1), inharmonicity_b=b, linear_density=mu,
                                    pluck_pos=zone.pos, pick=zone.pick, palm_mute=zone.palm_mute,
                                    t60_fundamental_s=round(float(open_t60[0]), 2)))
    # Net pumped volume (monopole) carries the low modes. Above ~1 kHz the top's modes cancel their
    # own volume but still radiate from edges and corners, with an efficiency that grows toward the
    # plate's coincidence frequency: a multipole term at a fixed fraction of the breathing mode's.
    f = box["frequency_hz"]
    breathing = float(np.abs(box["monopole"][1]))
    # Ariadne emits the momentum projected on this direction: a volume velocity. Far-field pressure
    # is its time derivative, so every weight carries one factor of frequency.
    radiating = (np.abs(box["monopole"])
                 + g.multipole * breathing * np.sqrt(np.minimum(f, g.coincidence_hz) / g.coincidence_hz)) * (f / 100.0)
    for k in range(BOX_BODIES):
        part = slice(k * MODE_COUNT, (k + 1) * MODE_COUNT)
        box_config = dict(metric=dict(frequency_hz=[float(x) for x in box["frequency_hz"][part]],
                                      t60_seconds=[float(x) for x in box["t60_seconds"][part]]),
                          aperture=inert_aperture(), radiation_direction=_normalized(radiating[part]))
        bodies.append(dict(id=f"box/{k}", body=dict(
            anatomy_name=f"steel-string-box-dreadnought-modes-{k * MODE_COUNT + 1}-{(k + 1) * MODE_COUNT}",
            configuration_path=dict(coordinate="static", left_name="as_built", right_name="as_built",
                                    left=box_config, right=box_config, initial_amount=0.0),
            source_direction=_normalized(box["saddle"][part]), active_directions=_active_directions(),
            transport=_quiet_transport(), reservoir=dict(capacity=1.0e-6)),
            space_port=_space_port(_normalized(radiating[part]), 0.6),
            pose=dict(position=[0.0, 0.0, 0.0], orientation=[0.0, 0.0, 0.0, 1.0])))
    plan = dict(schema_version=PLAN_SCHEMA, world_name="steel-string-dreadnought-v0",
                internal_sample_rate_hz=sample_rate, bodies=bodies, shared_space=_room(),
                direct_couplings=edges)
    receipt = dict(
        schema="mus.steel-string-anatomy-receipt.v0",
        geometry={k: v for k, v in asdict(g).items() if k != "strings"},
        evidence=dict(manufacturer=["scale_length", "body_length", "lower_bout", "waist", "upper_bout",
                                    "depth", "soundhole_radius", "string gauges"],
                      fit=["brace_long", "brace_cross", "bridge_tilt", "bridge_rise"],
                      design_hypothesis=["bridge_mass", "top_q", "air_q", "coincidence_hz", "multipole", "string loss law", "palm-mute contact law",
                                         "contact aperture", "bridge_rate_hz", "radiation weights", "room"]),
        limits=[f"the top's {BOX_BODIES * MODE_COUNT} lowest coupled modes, {MODE_COUNT} per Ariadne body",
                "back plate and ribs rigid", "top clamped at rim and rosette", "bracing as effective stiffness",
                "24 partials per string (low E reaches ~2 kHz)", "one fixed pluck point per string (from its hand)",
                "monopole-weighted radiation, no directivity"],
        box=dict(helmholtz_uncoupled_hz=round(box["helmholtz_hz"], 1),
                 top_uncoupled_hz=[round(float(x), 1) for x in box["uncoupled_top_hz"]],
                 coupled_hz=[round(float(x), 1) for x in box["frequency_hz"]],
                 air_share=[round(float(x), 3) for x in box["air_share"]],
                 cavity_volume_m3=round(box["cavity_volume_m3"], 4), top_area_m2=round(box["top_area_m2"], 4),
                 grid_points=box["grid_points"]),
        strings=strings_receipt)
    return plan, receipt


# ---------------------------------------------------------------- what a viewer needs


def visual_anatomy(g: SteelStringGeometry, box: dict) -> dict:
    """The same guitar as geometry: top mesh with every coupled mode's shape, sides, neck, strings.

    Mode shapes are the plate part of each coupled eigenvector (mass-normalized), evaluated on a
    triangulated top whose rim and rosette are clamped (shape 0 there), exactly as solved.
    """
    from scipy.spatial import Delaunay

    ys = np.linspace(0.0, g.body_length, 400)
    half = outline_halfwidth(g, ys)
    outline = np.concatenate([np.column_stack([half, ys]), np.column_stack([-half, ys])[::-1]])
    arc = np.r_[0.0, np.cumsum(np.linalg.norm(np.diff(outline, axis=0), axis=1))]
    even = np.arange(0.0, arc[-1], 0.004)                              # resample every 4 mm of rim
    outline = np.column_stack([np.interp(even, arc, outline[:, 0]), np.interp(even, arc, outline[:, 1])])
    angles = np.linspace(0, 2 * math.pi, 72, endpoint=False)
    rim = np.column_stack([g.soundhole_radius * np.cos(angles), g.soundhole_center + g.soundhole_radius * np.sin(angles)])
    interior = box["top_points"]
    points = np.vstack([interior, outline, rim])
    tri = Delaunay(points)
    centroid = points[tri.simplices].mean(axis=1)
    inside = np.abs(centroid[:, 0]) < outline_halfwidth(g, centroid[:, 1])
    inside &= np.hypot(centroid[:, 0], centroid[:, 1] - g.soundhole_center) > g.soundhole_radius
    faces = tri.simplices[inside]
    zeros = np.zeros((len(outline) + len(rim), box["top_shapes"].shape[1]))
    shapes = np.vstack([box["top_shapes"], zeros])
    nut_y = g.bridge_y + g.scale_length
    frets = [nut_y - g.scale_length * (1 - 2 ** (-k / 12)) for k in range(1, 21)]
    r = lambda v: [round(float(x), 5) for x in v]
    return dict(
        schema="mus.steel-string-visual.v0",
        note="metres; x across the top (bass side negative), y from the tail block toward the nut, z out of the top",
        body_length=g.body_length, depth=g.depth, top_thickness=g.top_thickness,
        soundhole=dict(center=[0.0, g.soundhole_center], radius=g.soundhole_radius),
        saddle_y=g.bridge_y, nut_y=nut_y, scale_length=g.scale_length,
        bridge=dict(y=g.bridge_y, halfwidth=0.08, depth=0.032),
        outline=[r(p) for p in outline],
        top=dict(vertices=[r(p) for p in points], faces=faces.tolist()),
        modes=dict(frequency_hz=r(box["frequency_hz"]), air_share=r(box["air_share"]),
                   saddle=r(box["saddle"]), shapes=[r(shapes[:, k]) for k in range(shapes.shape[1])]),
        neck=dict(joint_y=g.body_length, width_at_joint=0.056, width_at_nut=0.0445, frets_y=r(frets)),
        strings=[dict(number=n, course=c.name, open_midi=c.open_midi, gauge_m=round(c.gauge_in * 0.0254, 5),
                      saddle_x=round((n - 3.5) * -0.0105, 5), nut_x=round((n - 3.5) * -0.0072, 5))
                 for n, c in zip(range(6, 0, -1), g.strings)],
    )


def canonical_digest(obj) -> str:
    return "sha256:" + sha256(json.dumps(obj, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("-o", "--out", default="profiles/ariadne-steel-string-v0.plan.json")
    ap.add_argument("--sample-rate", type=float, default=48_000.0)
    ap.add_argument("--bridge-tilt", type=float, default=None, help="override the fitted bridge tilt (experiments)")
    ap.add_argument("--bridge-rise", type=float, default=None, help="override the fitted bridge rise (experiments)")
    ap.add_argument("--visual", default=None, help="also write the geometry and mode shapes for viewers (Blender)")
    args = ap.parse_args(argv)
    # Neutral hands: `mus render` folds each score's own hands (pos, pick, pm) into the strings
    # they play, so the shipped plan carries no palm and a mid pluck point.
    geometry = SteelStringGeometry()
    from dataclasses import replace
    if args.bridge_tilt is not None:
        geometry = replace(geometry, bridge_tilt=args.bridge_tilt)
    if args.bridge_rise is not None:
        geometry = replace(geometry, bridge_rise=args.bridge_rise)
    plan, receipt = steel_string_plan(geometry, [], args.sample_rate)
    receipt["plan_digest"] = canonical_digest(plan)
    if args.visual:
        visual = visual_anatomy(geometry, box_modes(geometry))
        visual["plan_digest"] = receipt["plan_digest"]
        Path(args.visual).write_text(json.dumps(visual, separators=(",", ":")) + "\n")
    out = Path(args.out)
    out.write_text(json.dumps(plan, indent=1) + "\n")
    out.with_name(out.name.replace(".plan.json", ".receipt.json")).write_text(json.dumps(receipt, indent=1) + "\n")
    box = receipt["box"]
    print(f"wrote {out}: A0/T11 {box['coupled_hz'][:3]} Hz (Helmholtz alone {box['helmholtz_uncoupled_hz']}, "
          f"top alone {box['top_uncoupled_hz'][:2]}), top {box['top_area_m2']} m2, {box['grid_points']} grid points")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
