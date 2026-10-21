"""Trombone v1: one N-D Ariadne body whose modes are computed from the bore.

Compiles ``profiles/ariadne-trombone-v1.plan.json`` (``ariadne-world-plan-v3``)
and its derivation receipt. One ``modal_space`` body:

* map ``tube_network``: the declared tenor-trombone bore of
  ``trombone_anatomy.TromboneGeometry`` (v0's profile and its fitted bell),
  preceded by a declared mouthpiece, as five tubes in series (mouthpiece,
  leadpipe, slide, bell tail, bell mouth; 124 volumes). The runtime computes
  the 24 lowest modes of the lossless Webster network at every point; the
  bell mouth is the radiating end (its conductance is the runtime's law);
* coordinate ``slide_extension_m``: each slide leg's extension, 0 .. 0.72 m.
  The slide tube's length is ``1.19 m + 2 x`` (a ``tube_length`` term: this
  needs ``patches/mus-ariadne-tube-length.patch``). Positions are solved, not
  chosen (receipt ``slide.positions_m``);
* connection ``mass_momentum`` (the air state is held across a slide move,
  the work billed; energy the retained modes cannot hold is radiated);
* law ``tube_network { valve }``: the lips are the flow valve (a mass on a
  tension spring with contact, a Bernoulli + Poiseuille slit jet into the
  cup volume), paid from the lungs (the body's reservoir, refilled as source
  work). Lip tension is the valve's tuning, moved with
  ``World::set_body_valve_tuning`` (billed as control work).

Numbers carry ``manufacturer``, ``fit``, ``design-hypothesis``, ``law`` or
``inherited`` labels in the receipt.
"""

from __future__ import annotations

import argparse
import json
import math
from dataclasses import asdict, dataclass, field
from pathlib import Path

import numpy as np

from mus_analysis.trombone_anatomy import TromboneGeometry, _bell, bell_length

PLAN_SCHEMA = "ariadne-world-plan-v3"
# fit (Nelder-Mead on this network's modes 2..8 against n x B-flat 1; four starts agree):
# -14, +4, +25, +19, -2, -13, -18 cents. Bore and bell diameters stay v0's manufacturer values.
V1_BORE = TromboneGeometry(tail_length=0.2229, flare=0.7031, flare_s0=0.0252)
BB1 = 440.0 * 2.0 ** ((34 - 69) / 12.0)
C = 343.0
RHO = 1.204
ETA = 1.81e-5
GAMMA = 1.402
PRANDTL = 0.71


@dataclass(frozen=True)
class Mouthpiece:
    """Design-hypothesis tenor mouthpiece (class-typical shape, nobody's model)."""

    cup_diameter: float = 0.0254      # rim inner diameter
    cup_depth: float = 0.009
    throat_diameter: float = 0.0066
    throat_length: float = 0.006
    backbore_exit: float = 0.0092     # = the leadpipe entry
    backbore_length: float = 0.073


@dataclass(frozen=True)
class Lips:
    """The flow valve as lips. design-hypothesis unless noted."""

    width_m: float = 0.012            # vibrating length of the slit
    thickness_m: float = 0.0015            # fit: with mass, keeps low tensions from being blown open
    mass_kg: float = 4.0e-4                # fit: partial 2 locks in every position (TROMBONE-WORLD.md)
    damping_ratio: float = 0.15
    mucosal_delay_s: float = 1.0e-3        # fit: the lip-tension sweep locks partial after partial (TROMBONE-WORLD.md)
    contact_stiffness_ratio: float = 3.0   # inherited (ovillo-tract glottis)
    contact_damping_ratio: float = 1.0     # inherited
    rest_half_gap_open_m: float = 5.0e-4
    rest_half_gap_closed_m: float = 2.0e-5
    min_tuning_hz: float = 25.0
    max_tuning_hz: float = 1000.0
    initial_tuning_hz: float = 120.0
    initial_adduction: float = 0.1
    max_mouth_pressure_pa: float = 15_000.0
    sag_fraction: float = 0.10       # inherited
    lung_capacity_j: float = 15.0
    lung_refill_w: float = 3.0


@dataclass(frozen=True)
class Grid:
    mouthpiece: int = 12
    leadpipe: int = 8
    slide: int = 56
    bell_tail: int = 28
    bell_mouth: int = 20
    mouth_split_m: float = 0.35      # bell_mouth = the last 0.35 m of the flare
    attenuation_reference_hz: float = 150.0
    max_extension_m: float = 0.72


def _kirchhoff_alpha(radius: float, f: float) -> float:
    w = 2 * math.pi * f
    return math.sqrt(w * ETA / (2 * RHO)) / (radius * C) * (1 + (GAMMA - 1) / math.sqrt(PRANDTL))


def _sample(profile, length, n):
    """Section-centre radii of a radius profile r(s), s in [0, length]."""
    return [profile((i + 0.5) / n * length) for i in range(n)]


def tubes(g: TromboneGeometry, mp: Mouthpiece, grid: Grid):
    rb = g.bore_diameter / 2
    # mouthpiece: cup (quarter-ellipse to the throat), throat, backbore cone
    cup_r, thr_r = mp.cup_diameter / 2, mp.throat_diameter / 2
    mp_len = mp.cup_depth + mp.throat_length + mp.backbore_length

    def mouthpiece(s):
        if s < mp.cup_depth:
            u = s / mp.cup_depth
            return thr_r + (cup_r - thr_r) * math.sqrt(max(0.0, 1 - u * u))
        if s < mp.cup_depth + mp.throat_length:
            return thr_r
        u = (s - mp.cup_depth - mp.throat_length) / mp.backbore_length
        return thr_r + (mp.backbore_exit / 2 - thr_r) * u

    lead = lambda s: g.leadpipe_entry / 2 + (rb - g.leadpipe_entry / 2) * s / g.leadpipe_length
    slide_len = 2 * g.slide_leg + g.crook_length
    # bell: the tail cone then the Bessel flare (v0's), split near the mouth
    flare = list(_bell(TromboneGeometry(**{**asdict(g), "grid": 1e-4})))
    s_acc, prof = 0.0, []
    for length, r in flare:
        prof.append((s_acc + 0.5 * length, r))
        s_acc += length
    flare_len = bell_length(g)
    prof_s = np.array([p[0] for p in prof])
    prof_r = np.array([p[1] for p in prof])

    def bell(s):  # s from the bell-tail start
        if s < g.tail_length:
            return rb + (g.tail_exit / 2 - rb) * s / g.tail_length
        return float(np.interp(s - g.tail_length, prof_s, prof_r))

    tail_total = g.tail_length + flare_len - grid.mouth_split_m
    out = [
        ("mouthpiece", mp_len, _sample(mouthpiece, mp_len, grid.mouthpiece)),
        ("leadpipe", g.leadpipe_length, _sample(lead, g.leadpipe_length, grid.leadpipe)),
        ("slide", slide_len, [rb] * grid.slide),
        ("bell_tail", tail_total, _sample(bell, tail_total, grid.bell_tail)),
        ("bell_mouth", grid.mouth_split_m, _sample(lambda s: bell(tail_total + s), grid.mouth_split_m, grid.bell_mouth)),
    ]
    return out


def network(g: TromboneGeometry, mp: Mouthpiece, grid: Grid):
    ts = tubes(g, mp, grid)
    spec_tubes = []
    alphas = {}
    for name, length, radii in ts:
        mean_r = float(np.mean(radii))
        alpha = _kirchhoff_alpha(mean_r, grid.attenuation_reference_hz)
        alphas[name] = alpha
        spec_tubes.append(dict(id=name, length_m=length, base_log_area_m2=[math.log(math.pi * r * r) for r in radii],
                               attenuation_np_per_m=alpha))
    branches = [dict(trunk=i, trunk_section=len(ts[i][2]) - 1, branch=i + 1) for i in range(len(ts) - 1)]
    slide_len = 2 * g.slide_leg + g.crook_length
    area_terms = [dict(kind="tube_length", tube=2, length_m=dict(constant=slide_len, terms=[[0, 2.0]]))]
    areas = [math.pi * r * r for _, _, radii in ts for r in radii]
    net = dict(speed_of_sound_m_s=C, air_density_kg_m3=RHO, area_floor_m2=min(areas) * 0.5,
               area_ceiling_m2=max(areas) * 2.0, tubes=spec_tubes, branches=branches, area_terms=area_terms,
               radiating_ends=[dict(tube=4, end_correction_ratio=0.61)])
    return net, ts, alphas


def valve(lips: Lips):
    return dict(tube=0, section=0, length_m=lips.width_m, thickness_m=lips.thickness_m, mass_kg=lips.mass_kg,
                damping_ratio=lips.damping_ratio, mucosal_delay_s=lips.mucosal_delay_s,
                contact_stiffness_ratio=lips.contact_stiffness_ratio, contact_damping_ratio=lips.contact_damping_ratio,
                rest_half_gap_open_m=lips.rest_half_gap_open_m, rest_half_gap_closed_m=lips.rest_half_gap_closed_m,
                min_tuning_hz=lips.min_tuning_hz, max_tuning_hz=lips.max_tuning_hz,
                initial_tuning_hz=lips.initial_tuning_hz, initial_adduction=lips.initial_adduction,
                max_supply_pressure_pa=lips.max_mouth_pressure_pa, sag_fraction=lips.sag_fraction,
                air_viscosity_pa_s=ETA)


def _space_port_zero():
    rows = []
    for course in range(8):
        d = [0.0] * 24
        d[(3 * course) % 24] = 1.0
        rows.append(d)
    return dict(body_directions=rows, coupling_weights=[0.0] * 8)


def _room():
    def nz(v):
        v = np.asarray(v, float)
        return [float(x) for x in v / np.linalg.norm(v)]
    return dict(delays_seconds=[0.0061, 0.0093, 0.0127, 0.0171, 0.0219, 0.0283, 0.0397, 0.0613],
                scattering_direction=nz([1.0, -0.73, 0.88, -0.61, 0.79, -0.52, 0.69, -0.44]),
                radiation_left=nz([1.0, 0.62, 0.18, -0.31, -0.70, -0.54, 0.08, 0.48]),
                radiation_right=nz([0.17, -0.51, -0.72, -0.24, 0.36, 0.61, 0.31, -0.22]))


def network_modes(net, extension=0.0, count=24):
    """The runtime's mode solve (configuration/network.rs) for series networks, in numpy:
    C_i = A dx / (rho c^2); columns rho (dx_a/2)/A_a + rho (dx_b/2)/A_b, a branch column the
    branch half only, the radiating end its half + 0.61 r; K = D^T L^-1 D; K phi = w^2 C phi."""
    from scipy.linalg import eigh
    rho, c = net["air_density_kg_m3"], net["speed_of_sound_m_s"]
    tubes = net["tubes"]
    lens = [t["length_m"] for t in tubes]
    for term in net["area_terms"]:
        if term["kind"] == "tube_length":
            lens[term["tube"]] = term["length_m"]["constant"] + sum(k * extension for _, k in term["length_m"]["terms"])
    A, node_dx, off, dxs = [], [], [], []
    for t, L in zip(tubes, lens):
        off.append(len(A))
        n = len(t["base_log_area_m2"])
        A += [min(max(math.exp(v), net["area_floor_m2"]), net["area_ceiling_m2"]) for v in t["base_log_area_m2"]]
        dxs.append(L / n)
        node_dx += [L / n] * n
    A = np.array(A)
    cols = []
    for i, t in enumerate(tubes):
        for s_ in range(len(t["base_log_area_m2"]) - 1):
            a = off[i] + s_
            cols.append((a, a + 1, rho * 0.5 * dxs[i] / A[a] + rho * 0.5 * dxs[i] / A[a + 1]))
    for b in net["branches"]:
        bn = off[b["branch"]]
        cols.append((off[b["trunk"]] + b["trunk_section"], bn, rho * 0.5 * dxs[b["branch"]] / A[bn]))
    for e in net["radiating_ends"]:
        nd_ = off[e["tube"]] + len(tubes[e["tube"]]["base_log_area_m2"]) - 1
        r = math.sqrt(A[nd_] / math.pi)
        cols.append((nd_, None, rho * 0.5 * dxs[e["tube"]] / A[nd_] + rho * e["end_correction_ratio"] * r / A[nd_]))
    D = np.zeros((len(cols), len(A)))
    linv = np.zeros(len(cols))
    for j, (a, b, L) in enumerate(cols):
        D[j, a] = 1.0
        if b is not None:
            D[j, b] = -1.0
        linv[j] = 1.0 / L
    K = D.T @ (linv[:, None] * D)
    w2 = eigh(K, np.diag(A * np.array(node_dx) / (rho * c * c)), eigvals_only=True, subset_by_index=[0, count - 1])
    return np.sqrt(w2) / (2 * math.pi)


def slide_positions(net, grid: Grid, partials=range(2, 9)):
    """Extension x_k (m) at which the mean log of modes 2..8 is k-1 semitones below first position."""
    idx = [p - 1 for p in partials]
    mean = lambda x: float(np.mean(np.log(network_modes(net, x)[idx])))
    base = mean(0.0)
    out = [0.0]
    for k in range(2, 8):
        target = base - (k - 1) * math.log(2) / 12
        lo, hi = out[-1], grid.max_extension_m
        for _ in range(50):
            mid = 0.5 * (lo + hi)
            lo, hi = (mid, hi) if mean(mid) > target else (lo, mid)
        out.append(0.5 * (lo + hi))
    return out


def plan(g=V1_BORE, mp=Mouthpiece(), lips=Lips(), grid=Grid(), sample_rate=48_000.0, initial_extension=0.0):
    net, ts, alphas = network(g, mp, grid)
    positions = slide_positions(net, grid)
    table_x = [round(grid.max_extension_m * i / 60, 6) for i in range(61)]
    table = [[float(f) for f in network_modes(net, x)] for x in table_x]
    first = table[0]
    body = dict(
        anatomy_name="ovillo-trombone-nd-v1",
        configuration_space=dict(
            coordinates=[dict(name="slide_extension_m", lower=0.0, upper=grid.max_extension_m, initial=initial_extension)],
            map=dict(kind="tube_network", network=net),
            connection=dict(kind="mass_momentum", disposal="radiate"),
        ),
        laws=dict(kind="tube_network", valve=valve(lips)),
        reservoir=dict(capacity=lips.lung_capacity_j),
    )
    world = dict(schema_version=PLAN_SCHEMA, world_name="ovillo-trombone-v1", internal_sample_rate_hz=sample_rate,
                 bodies=[dict(id="trombone", constitution=dict(kind="modal_space", body=body),
                              space_port=_space_port_zero(), pose=dict(position=[0.0, 0.0, 0.0], orientation=[0.0, 0.0, 0.0, 1.0]))],
                 shared_space=_room(), direct_couplings=[])
    receipt = dict(
        schema="ovillo.trombone-nd-anatomy-receipt.v1",
        slide=dict(positions_m=positions, note="position k: modes 2..8 (mean log) k-1 semitones below first position"),
        mode_table=dict(extension_m=table_x, frequency_hz=table,
                        note="the runtime's 24 modes (numpy port of network.rs, same assembly) at each extension"),
        bb_series_cents_first=[float(1200 * math.log2(first[n - 1] / (n * BB1))) for n in range(2, 9)],
        bore=asdict(g), mouthpiece=asdict(mp), lips=asdict(lips), grid=asdict(grid),
        tubes=[dict(id=n, length_m=l, sections=len(r), dx_m=l / len(r), radius_min_m=min(r), radius_max_m=max(r),
                    attenuation_np_per_m=alphas[n]) for n, l, r in ts],
        evidence=dict(
            manufacturer=["bore.bore_diameter", "bore.bell_diameter"],
            fit=["bore.tail_length", "bore.flare", "bore.flare_s0 (on this network's modes 2..8 vs n x Bb1)",
                 "lips.mucosal_delay_s", "lips.mass_kg", "lips.thickness_m"],
            design_hypothesis=["mouthpiece.*", "lips.* except as inherited", "grid.*",
                               "bore leadpipe/slide/crook/tail_exit (v0)", "wall loss at grid.attenuation_reference_hz"],
            inherited=["lips.contact_stiffness_ratio", "lips.contact_damping_ratio", "lips.sag_fraction (ovillo-tract glottis)"],
            law=["24 modes of the lossless Webster network (mus-ariadne configuration/network.rs)",
                 "radiation conductance at the bell mouth (network.rs)",
                 "wall attenuation: Kirchhoff boundary layer, alpha = sqrt(w eta / 2 rho)/(a c)(1 + (gamma-1)/sqrt(Pr))"],
        ),
        limits=[
            "plane waves on a staggered grid of 124 volumes (dx 7 mm in the mouthpiece to 21-43 mm in the slide): "
            "the grid's dispersion flattens the upper modes",
            "each series junction joins two tubes through the downstream half-section only (the runtime's branch column)",
            "wall attenuation is frequency independent per tube (the runtime's alpha), Kirchhoff's at 150 Hz",
            "the slide stretches all slide sections uniformly (a rubber slide, not a telescope); the air state is held",
            "the lips are the runtime's flow valve (mechanics copied from the ovillo-tract glottis), its aerodynamic work "
            "booked to the lungs (the runtime's declared approximation)",
            "the bell radiates through the runtime's low-ka conductance, no directivity; the World is heard dry",
        ],
    )
    return world, receipt


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("-o", "--out", default="profiles/ariadne-trombone-v1.plan.json")
    ap.add_argument("--set", action="append", default=[], help="section.name=value, e.g. lips.mass_kg=2e-4")
    args = ap.parse_args(argv)
    parts = dict(bore={}, mouthpiece={}, lips={}, grid={})
    for kv in args.set:
        key, value = kv.split("=", 1)
        sect, name = key.split(".", 1)
        parts[sect][name] = int(value) if sect == "grid" and value.isdigit() else float(value)
    w, r = plan(TromboneGeometry(**{**asdict(V1_BORE), **parts["bore"]}), Mouthpiece(**parts["mouthpiece"]), Lips(**parts["lips"]), Grid(**parts["grid"]))
    out = Path(args.out)
    out.write_text(json.dumps(w, indent=1) + "\n")
    out.with_name(out.name.replace(".plan.json", ".receipt.json")).write_text(json.dumps(r, indent=1) + "\n")
    print("positions (m):", " ".join(f"{x:.4f}" for x in r["slide"]["positions_m"]))
    print("vs n x Bb1 (n=2..8):", " ".join(f"{c:+.0f}" for c in r["bb_series_cents_first"]))
    for t in r["tubes"]:
        print(f"{t['id']:11s} {t['length_m']:.3f} m  {t['sections']:3d} x {1000 * t['dx_m']:5.1f} mm  r {1000 * t['radius_min_m']:.1f}-{1000 * t['radius_max_m']:.1f} mm  alpha {t['attenuation_np_per_m']:.3f} Np/m")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
