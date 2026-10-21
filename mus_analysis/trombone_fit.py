"""The trombone World's per-sample map in numpy, and the fit that set the bell.

``Sim`` is a line-by-line port of one Ariadne World sample for the trombone
plan's bodies: ``mus-rs/crates/mus-ariadne/src/law/aperture.rs``
(``CompliantApertureBoundary::process``), ``field.rs`` (``propagate``, ``damp``,
``radiate``), ``coupling.rs`` (Givens on momentum projections), in the order of
``shared_world/process.rs`` (every body's local laws, then couplings, then
radiation; the space is off). Checked against the Rust World on a one-body
probe while building it: stored energy after 0.1 s of blowing 0.184326
(numpy) against 0.18439 (Rust, logged one sample later). Linearizing it gives
what this module is for:

* ``passive_modes``: the coupled resonances of cup + throat + bore;
* ``seen``: the response of the cup's momentum to a momentum injection at the
  lips, i.e. the bore as the lips see it through the throat;
* ``fit_bell``: Nelder-Mead on (total length, flare, flare_s0) so the coupled
  resonances 2..8 sit on n x B-flat 1. Its result is the ``fit`` values of
  ``TromboneGeometry`` (tail_length 0.201, flare 1.406, flare_s0 0.194:
  -46/-13/-8/-2/-11/-10/-12 cents for n = 2..8).

usage: python -m mus_analysis.trombone_fit seen|modes|fit [plan.json]
"""
from __future__ import annotations

import json
import math
import sys
from dataclasses import dataclass, field

import numpy as np

TAU = 2 * math.pi


def givens(a, b, ang):
    s, c = math.sin(ang), math.cos(ang)
    return c * a - s * b, s * a + c * b


@dataclass
class Aperture:
    f_nat: float
    quartic: float = 4.1e6
    asym: float = 0.028
    xeq: float = 0.0
    bias: float = 0.055
    damp: float = 7.9
    s2a: float = 0.0
    s2f: float = 0.0
    sweep: float = 0.0
    active: float = 0.0
    target: float = 0.334

    def dgrad(self, old, new):
        w = TAU * self.f_nat
        o, n = old - self.xeq, new - self.xeq
        quad = 0.5 * w * w + self.quartic * self.asym**2
        cub = self.quartic * self.asym
        qua = 0.25 * self.quartic
        return quad * (o + n) + cub * (o * o + o * n + n * n) + qua * (o**3 + o * o * n + o * n * n + n**3)

    def dgrad_d(self, old, new):
        w = TAU * self.f_nat
        o, n = old - self.xeq, new - self.xeq
        quad = 0.5 * w * w + self.quartic * self.asym**2
        cub = self.quartic * self.asym
        qua = 0.25 * self.quartic
        return quad + cub * (o + 2 * n) + qua * (o * o + 2 * o * n + 3 * n * n)


@dataclass
class Modal:
    freq: np.ndarray
    t60: np.ndarray
    src_dir: np.ndarray  # aperture direction
    rad_dir: np.ndarray
    ap: Aperture | None
    rad_fraction: float = 0.0


def unit(v):
    v = np.asarray(v, float)
    return v / np.linalg.norm(v)


class Sim:
    """State vector: per body [x, pa, q(24), P(24)]."""

    def __init__(self, bodies: list[Modal], couplings, sr=48000.0):
        self.b = bodies
        self.c = couplings  # list of (li, ri, ldir, rdir, rate_hz)
        self.sr = sr
        self.dt = 1.0 / sr
        for m in bodies:
            m.src_dir = unit(m.src_dir)
            m.rad_dir = unit(m.rad_dir)
        self.n = 50 * len(bodies)

    def step(self, X, S):
        X = X.copy()
        dt = self.dt
        emitted = np.zeros(len(self.b))
        for k, m in enumerate(self.b):
            o = 50 * k
            x, pa = X[o], X[o + 1]
            q = X[o + 2 : o + 26]
            P = X[o + 26 : o + 50]
            w = TAU * m.freq
            ap = m.ap
            s_in = S[k]
            if ap is not None:
                v = m.src_dir
                eff = float(np.dot(v, w * q))
                opening = max(ap.bias + x, 0.0)
                of = opening / (1 + opening)
                th1 = TAU * ap.s2a * dt * of
                s, pa = givens(s_in, pa, th1)
                d = float(np.dot(P, v))
                th2 = TAU * ap.s2f * dt * of
                ret, d2 = givens(s, d, th2)
                P = P + v * (d2 - d)
                d = d2
                geo = math.tanh(2.5 * x + 0.2 * eff)
                th3 = TAU * ap.sweep * dt * (0.2 + 0.8 * of) * geo
                pa2, d2 = givens(pa, d, th3)
                pa = pa2
                P = P + v * (d2 - d)
                d = d2
                if ap.active > 0 and s_in != 0 and abs(pa) > 1e-15:
                    A = ap.target
                    sync = 1 + 0.4 * math.tanh(-d * pa / (0.01 + abs(pa)))
                    sw = dt * ap.active * abs(s_in) * of * sync * pa * pa * (1 - pa * pa / (A * A))
                    if sw > 0:
                        pa = math.copysign(math.sqrt(pa * pa + 2 * sw / dt), pa)
                    elif sw < 0:
                        avail = 0.5 * dt * pa * pa
                        dis = min(-sw, avail)
                        pa = math.copysign(math.sqrt(max(pa * pa - 2 * dis / dt, 0.0)), pa)
                # propagate aperture (discrete gradient, Newton 8)
                x0, p0 = x, pa
                x1 = x0 + dt * p0
                for _ in range(8):
                    g = ap.dgrad(x0, x1)
                    eq = 2 * (x1 - x0) / dt - 2 * p0 + dt * g
                    der = 2 / dt + dt * ap.dgrad_d(x0, x1)
                    x1 -= max(-4.0, min(4.0, eq / der))
                pa = 2 * (x1 - x0) / dt - p0
                x = x1
                pa *= math.exp(-ap.damp * dt)
            # field propagate
            c, s_ = np.cos(w * dt), np.sin(w * dt)
            q, P = c * q + s_ * P / w, c * P - s_ * w * q
            ret_ = np.exp(-3 * math.log(10) / (m.t60 * self.sr))
            q = q * ret_
            P = P * ret_
            X[o], X[o + 1] = x, pa
            X[o + 2 : o + 26] = q
            X[o + 26 : o + 50] = P
        for li, ri, ld, rd, rate in self.c:
            ld, rd = unit(ld), unit(rd)
            Pl = X[50 * li + 26 : 50 * li + 50]
            Pr = X[50 * ri + 26 : 50 * ri + 50]
            a, b = float(Pl @ ld), float(Pr @ rd)
            a2, b2 = givens(a, b, TAU * rate * dt)
            X[50 * li + 26 : 50 * li + 50] = Pl + ld * (a2 - a)
            X[50 * ri + 26 : 50 * ri + 50] = Pr + rd * (b2 - b)
        for k, m in enumerate(self.b):
            o = 50 * k
            P = X[o + 26 : o + 50]
            proj = float(P @ m.rad_dir)
            f = m.rad_fraction
            emitted[k] = math.sqrt(f) * proj
            X[o + 26 : o + 50] = P - (1 - math.sqrt(1 - f)) * m.rad_dir * proj
        return X, emitted

    def settle(self, S, n=48000, X=None):
        X = np.zeros(self.n) if X is None else X
        for _ in range(n):
            X, _ = self.step(X, S)
        return X

    def fixed_point(self, S, X0=None, iters=30):
        """Newton on F(X) = step(X) - X."""
        X = self.settle(S, 4800) if X0 is None else X0
        for _ in range(iters):
            J = self.jac(X, S)
            F = self.step(X, S)[0] - X
            dx = np.linalg.lstsq(J - np.eye(self.n), -F, rcond=None)[0]
            X = X + dx
            if np.linalg.norm(dx) < 1e-14 * (1 + np.linalg.norm(X)):
                break
        return X

    def jac(self, X, S, h=1e-7):
        f0 = self.step(X, S)[0]
        J = np.zeros((self.n, self.n))
        for i in range(self.n):
            e = np.zeros(self.n)
            hh = h * max(1.0, abs(X[i]))
            e[i] = hh
            J[:, i] = (self.step(X + e, S)[0] - f0) / hh
        return J

    def unstable(self, S, X=None):
        X = self.fixed_point(S, X)
        lam = np.linalg.eigvals(self.jac(X, S))
        f = np.angle(lam) * self.sr / TAU
        g = np.log(np.abs(lam) + 1e-300) * self.sr  # growth rate 1/s (amplitude)
        order = np.argsort(-g)
        return X, [(float(f[i]), float(g[i])) for i in order if f[i] >= 0][:6]


def _lerp_ap(l, r, a):
    return {k: l[k] + a * (r[k] - l[k]) for k in l}


def from_plan(plan, receipt, lip_amount, slide_amount, sr=None):
    """Build a Sim from an ariadne-world-plan-v2 JSON (lips, bore) at given amounts."""
    sr = sr or plan["internal_sample_rate_hz"]
    bodies = []
    amounts = {"lips": lip_amount, "bore": slide_amount}
    rf = receipt["radiation_fraction"]
    for b in plan["bodies"]:
        path = b["body"]["configuration_path"]
        a = amounts[b["id"]]
        L, R = path["left"], path["right"]
        fl, fr = np.array(L["metric"]["frequency_hz"]), np.array(R["metric"]["frequency_hz"])
        freq = np.exp(np.log(fl) + a * (np.log(fr) - np.log(fl)))
        t60 = np.array(L["metric"]["t60_seconds"]) + a * (np.array(R["metric"]["t60_seconds"]) - np.array(L["metric"]["t60_seconds"]))
        rad = unit((1 - a) * np.array(L["radiation_direction"]) + a * np.array(R["radiation_direction"]))
        ap = _lerp_ap(L["aperture"], R["aperture"], a)
        aper = None
        if ap["source_to_field_rate_hz"] > 0 or ap["sweep_rate_hz"] > 0:
            aper = Aperture(f_nat=ap["natural_frequency_hz"], quartic=ap["quartic_stiffness"], asym=ap["asymmetry"],
                            xeq=ap["equilibrium_displacement"], bias=ap["opening_bias"], damp=ap["damping_per_second"],
                            s2a=ap["source_to_aperture_rate_hz"], s2f=ap["source_to_field_rate_hz"], sweep=ap["sweep_rate_hz"],
                            active=ap["active_rate_per_second"], target=ap["active_target_momentum"])
        frac = 0.0
        if b["id"] == "bore":
            frac = rf["first"] + a * (rf["seventh"] - rf["first"])
        bodies.append(Modal(freq, t60, np.array(b["body"]["source_direction"]), rad, aper, frac))
    ids = [b["id"] for b in plan["bodies"]]
    cps = [(ids.index(c["left_body"]), ids.index(c["right_body"]), np.array(c["left_direction"]),
            np.array(c["right_direction"]), c["rate_hz"] * c["weight"]) for c in plan["direct_couplings"]]
    return Sim(bodies, cps, sr)


def passive_modes(sim):
    """Eigen-frequencies and amplitude decay rates of the passive linear map at zero state."""
    X = np.zeros(sim.n)
    lam = np.linalg.eigvals(sim.jac(X, [0.0] * len(sim.b)))
    f = np.angle(lam) * sim.sr / TAU
    g = np.log(np.abs(lam) + 1e-300) * sim.sr
    keep = f > 1.0
    return sorted(zip(f[keep], g[keep]))


def seen(plan, receipt, slide=0.0, f=None):
    """|H| of cup momentum per unit momentum injected at the cup, passive World."""
    sim = from_plan(plan, receipt, 0.0, slide)
    for b in sim.b:
        b.ap = None
    n = sim.n
    A = sim.jac(np.zeros(n), [0.0] * len(sim.b))
    f = np.arange(20, 1500, 0.5) if f is None else f
    B = np.zeros(n)
    B[26] = 1.0
    H = np.array([np.linalg.solve(np.exp(1j * TAU * ff / sim.sr) * np.eye(n) - A, B)[26] for ff in f])
    return f, H


def fit_bell(x0=(2.75, 1.0, 0.07), maxfev=60):
    from scipy.optimize import minimize

    from mus_analysis import trombone_anatomy as ta

    bb1 = 440 * 2 ** ((34 - 69) / 12)

    def coupled(g):
        seg = ta.bore_profile(g, 0.0)
        f = np.arange(8.0, 1700.0, 0.5)
        pk = ta._peaks(f, ta.input_impedance(seg, f))[:24]
        freq = np.array([p[0] for p in pk])
        q = np.array([p[2] for p in pk])
        a = freq * np.array([p[1] for p in pk]) / q
        e0 = np.zeros(24)
        e0[0] = 1
        cf = np.array([g.cup_hz] + [1000 + 100 * i for i in range(1, 24)])
        ct = np.array([g.cup_t60] + [1.0] * 23)
        sim = Sim([Modal(cf, ct, e0, e0, None, 0.0), Modal(freq, math.log(1000) / (math.pi * freq / q), np.ones(24), np.ones(24), None, 0.0)],
                  [(0, 1, e0, np.sqrt(a * freq ** (-g.throat_tilt)), g.throat_rate_hz)])
        m = passive_modes(sim)
        return np.array([1200 * math.log2(min(m, key=lambda t: abs(math.log(t[0] / freq[n - 1])))[0] / (n * bb1)) for n in range(2, 9)])

    def obj(x):
        total, flare, s0 = x
        g0 = ta.TromboneGeometry(flare=flare, flare_s0=s0)
        tail = total - (g0.leadpipe_length + 2 * g0.slide_leg + g0.crook_length) - ta.bell_length(g0)
        if not (0.2 < tail < 1.5 and 0.5 < flare < 2.0 and 0.01 < s0 < 0.4):
            return 1e9
        c = coupled(ta.TromboneGeometry(tail_length=tail, flare=flare, flare_s0=s0))
        print(f"total {total:.3f} tail {tail:.3f} flare {flare:.3f} s0 {s0:.4f} -> {' '.join(f'{v:+.0f}' for v in c)}", flush=True)
        return float(np.sum(c * c))

    return minimize(obj, list(x0), method="Nelder-Mead", options=dict(maxfev=maxfev, xatol=1e-3, fatol=1))


if __name__ == "__main__":
    what = sys.argv[1] if len(sys.argv) > 1 else "seen"
    path = sys.argv[2] if len(sys.argv) > 2 else "profiles/ariadne-trombone-v0.plan.json"
    plan = json.load(open(path))
    receipt = json.load(open(path.replace(".plan.json", ".receipt.json")))
    bb1 = 440 * 2 ** ((34 - 69) / 12)
    if what == "seen":
        f, H = seen(plan, receipt)
        m = np.abs(H)
        for i in [i for i in range(1, len(f) - 1) if m[i] > m[i - 1] and m[i] >= m[i + 1]][:12]:
            n = max(1, round(f[i] / bb1))
            print(f"{f[i]:7.1f} Hz |H| {m[i]:.3f}  vs {n} x Bb1 {1200 * math.log2(f[i] / (n * bb1)):+.0f} c")
    elif what == "modes":
        for f, g in passive_modes(from_plan(plan, receipt, 0.0, 0.0))[:16]:
            print(f"{f:8.2f} Hz  decay {g:+7.2f} /s")
    else:
        print(fit_bell())
