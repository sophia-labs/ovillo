#!/usr/bin/env python3
"""Three more fictional Ariadne tab anatomies for Vera's farewell film: relatives
of the *Before the Law* set (`../before-the-law/`), built the same way and
with the same code (this script imports the laws and the plan assembler from
`../before-the-law/build_instruments.py`; nothing there changes).

    walker   the Waiting's low plucked courses, restrung as a four-course
             upright bass that walks (E1 A1 D2 G2, fretted over an octave)
    vibes    the Light's glass bars, recast as a chromatic bar instrument over
             tuned tubes (C4..A5: metal-like losses, a longer ring)
    kit      the Fleas' clicks and the Slam's membranes, as a small jazz kit:
             kick, snare, a cross-stick, closed and open hi-hat, a ride

Every number this script sets is a design hypothesis. Where a mode family
follows a textbook law (ideal membrane, free-free beam, thin ring, the stiff
string) the law is named in the recipe; the sizes and losses fed to it are
chosen, not measured. Nothing here reconstructs a real bass, vibraphone,
drum or cymbal; the "what it does not model" list is in README.md.

    python3 build_instruments.py        # writes <name>.plan.json + <name>.design.json
"""
import math
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "before-the-law"))
import build_instruments as btl  # noqa: E402  (the laws and the assembler)
from build_instruments import BEAM, BESSEL, N, CAP, cell, box, t60, norm, hz, note_name, ring, alternating, scattered, strung, scaled_baseline_boxes, BASE_BOXES  # noqa: E402


def walker():
    """THE WALKER: an upright bass that walks, from the Waiting's courses."""
    tuning = [28, 33, 38, 43]  # E1 A1 D2 G2, low to high (the orchestral bass's fourths)
    cells = []
    for m in tuning:
        # Less stiff than the Waiting's courses (B 2.5e-5 against 6e-5), a slower
        # low loss (fundamentals ring 7 s) and a much faster high loss (the top
        # partials are gone in 50 ms): a dark pizzicato.
        freq, tt = strung(m, 2.5e-5, 0.9, 6.0e-7)
        cells.append(cell(f"course {note_name(m)}", "stiff string f_n = n f1 sqrt((1+Bn^2)/(1+B)), B = 2.5e-5",
                          freq, tt, alternating(), fretted=True))
    cells = list(reversed(cells))
    # The body: the steel-string box scaled x0.6 in frequency (its air mode lands
    # near 64 Hz) and x1.0 in T60; a small club, not the stone hall.
    return dict(name="walker", world="rule-britannia-walker-v1", cells=cells, boxes="baseline:0.6:1.0",
                room_scale=2.0, position=-0.15, bridge_rate=[0.8, 0.8, 0.8], bridge_weight=1.0,
                intention="A walking upright bass: four plucked courses E1 A1 D2 G2 on a big wooden body, each fretted over an octave. Pizzicato only.")


VIBES_BARS = list(range(60, 82))  # C4 .. A5, chromatic (above A5 a bar has fewer than 24 modes under the runtime band)


def vibes():
    """THE VIBES: the Light's bars, chromatic, longer ringing, over tuned tubes."""
    rt, wt = 1.35, 3.2  # torsional/bending and width/thickness ratios of the bar section (the Light's)
    cells = []
    for m in VIBES_BARS:
        f1 = hz(m)
        # The Light's construction, unchanged in its mode families: undercut bars
        # tuned 1 : 4 : 10 (vibraphone and marimba practice), the free-free series
        # scaled to continue from 10 (invented), lateral bending, torsion; in
        # that index order, because a mallet just off the centre line barely
        # excites torsion. What changes is the loss: s0 0.30 against the Light's
        # 0.9 (a metal bar's slow low-frequency loss) and a weaker bridge to the
        # tubes and case, so the bars keep their energy longer.
        tuned = [1.0, 4.0, 10.0] + [10.0 * BEAM[k] / BEAM[2] for k in range(3, 20)]
        fam = [(f1 * r, 0) for r in tuned] + [(f1 * wt * r, 1) for r in BEAM[:8]] + [(f1 * rt * k, 2) for k in range(1, 30)]
        fam = sorted((f, kind) for f, kind in fam if f < CAP)
        assert len(fam) >= N, (m, len(fam))
        keep = sorted(fam[:N], key=lambda fk: (fk[1], fk[0]))
        freq = [f for f, _ in keep]
        reach = [(1.0, 0.4, 0.1)[kind] for _, kind in keep]
        cells.append(cell(f"bar {note_name(m)}", "undercut bar tuned 1:4:10, then lateral bending and torsion (ratio 1.35), in that index order",
                          freq, [t60(f, 0.30, 1.0e-8) for f in freq],
                          lambda j, reach=reach: norm([(-1) ** i * r for i, r in enumerate(reach)])))
    cells = list(reversed(cells))  # string 1 = A5 ... string 22 = C4
    # Two boxes of resonator tubes (closed-open: f, 3f, 5f), 11 bars each, since
    # a box holds 24 modes: the 11 fundamentals and the 13 lowest odd harmonics.
    # Each bar meets its own tube (weight 1/h) and the others barely (0.02).
    # box/0 takes strings 1-11 (A5 down to B4), box/1 strings 12-22 (Bb4 to C4).
    def tube_box(name, cell_indices):
        fund = [(hz(VIBES_BARS[::-1][c]), c, 1) for c in cell_indices]
        upper = sorted((hz(VIBES_BARS[::-1][c]) * h, c, h) for c in cell_indices for h in (3, 5))[: N - len(fund)]
        tubes = sorted(fund + upper)
        tfreq = [f for f, _, _ in tubes]
        tt = [6.91 * 120 / (math.pi * f) for f in tfreq]  # Q 120
        trad = [1.0 / h for _, _, h in tubes]
        tattach = lambda c: norm([(1.0 / h if k == c else 0.02) for _, k, h in tubes])
        return box(name, tfreq, tt, trad, tattach)
    upper = tube_box("tubes, upper", list(range(0, 11)))
    lower = tube_box("tubes, lower", list(range(11, 22)))
    case = scaled_baseline_boxes(BASE_BOXES, 1.0, 0.8, ["case", "case", "case"])[2]
    case["name"] = "case high"
    reach = [[1.0, 0.05, 0.3] if i < 11 else [0.05, 1.0, 0.3] for i in range(22)]
    return dict(name="vibes", world="rule-britannia-vibes-v1", cells=cells, boxes=[upper, lower, case], reach=reach,
                room_scale=2.5, position=0.30, bridge_rate=[0.08, 0.03, 0.1], bridge_weight=1.0,
                intention="A vibraphone-like bar instrument: twenty-two bars C4..A5 over tuned tubes, struck with a mallet just off centre; longer ringing than the Light's glass.")


def kit():
    """THE KIT: a small jazz kit from the Slam's membranes and the Fleas' clicks."""
    def membrane(f01, s0, s2):
        modes = sorted((j / BESSEL[(0, 1)], mn) for mn, j in BESSEL.items())[:N]
        freq = [f01 * r for r, _ in modes]
        return freq, [t60(f, s0, s2) for f in freq], modes

    def rings(f2, s0, s2, split=1.06):
        """Two thin rings' in-plane flexural modes n = 2..13 (one at x`split`,
        standing in for the out-of-plane family, as the Gate's chain does)."""
        freq = sorted(f2 * ring(n) * s for n in range(2, 14) for s in (1.0, split))
        return freq, [t60(f, s0, s2) for f in freq]

    cells = []
    # string 1: the cross-stick, a wooden stick laid across the snare's rim: a
    # free-free beam in two bending planes split 0.5 %, f1 chosen at F4 so that
    # twelve bending modes fit under the runtime band; gone in 0.15 s.
    f1 = hz(65)
    freq = sorted(f1 * BEAM[k] * s for k in range(12) for s in (1.0, 1.005))
    cells.append(cell("cross-stick", "free-free beam, two bending planes split 0.5 %", freq,
                      [t60(f, 45.0, 2.0e-8) for f in freq], alternating(0.02), note="stick on the rim"))
    # strings 2, 3: the hi-hat closed and open: the same pair of rings (f2 at
    # C4, 262 Hz; the n = 13 mode lands near 16.4 kHz), the closed one choked.
    for name, s0, s2 in (("hi-hat closed", 60.0, 4.0e-9), ("hi-hat open", 9.0, 3.0e-9)):
        freq, tt = rings(hz(60), s0, s2)
        cells.append(cell(name, "thin ring flexural modes n = 2..13, two rings split 6 % (no law for a cymbal's own modes)", freq, tt, scattered(21 if "closed" in name else 22)))
    # string 4: the ride: the same ring law at G3 (196 Hz; n = 13 near 12.3 kHz), long.
    freq, tt = rings(hz(55), 2.0, 1.5e-9)
    cells.append(cell("ride", "thin ring flexural modes n = 2..13, two rings split 6 % (no law for a cymbal's own modes)", freq, tt, scattered(23)))
    # string 5: the snare drum: an ideal membrane, f01 at G3, short.
    freq, tt, modes = membrane(hz(55), 18.0, 5.0e-7)
    cells.append(cell("snare", "ideal membrane (Bessel zeros); the wires are a radiator, not a contact", freq, tt,
                      lambda j, modes=modes: norm([0.5 + 0.5 * (mn[0] == 0) for _, mn in modes])))
    # string 6: the kick: an ideal membrane, f01 at F1 (43.7 Hz), one head.
    freq, tt, modes = membrane(hz(29), 6.0, 2.0e-6)
    cells.append(cell("kick", "ideal membrane (Bessel zeros), one head", freq, tt,
                      lambda j, modes=modes: norm([1.0 if mn[0] == 0 else 0.4 for _, mn in modes])))
    # Radiators: box/0 the shells (the steel-string box x0.5 in frequency, x0.6
    # in T60, as the Slam's riser); box/1 the snare wires, an invented dense
    # cluster 0.7-5 kHz that rings in sympathy (linear: no rattle, no contact);
    # box/2 a cymbal stand and bell, the box's high band x1.8. The wires'
    # cluster starts at 700 Hz so that the snare head's upper modes (to 1.2 kHz)
    # meet it.
    shells = scaled_baseline_boxes(BASE_BOXES, 0.5, 0.6, ["shells", "shells", "shells"])[0]
    wires_f = [700.0 * (1 + 0.09 * k + 0.006 * k * k) for k in range(N)]
    wires = box("snare wires", wires_f, [t60(f, 25.0, 3.0e-9) for f in wires_f], [(-1) ** k * math.sqrt(f / 1000) for k, f in enumerate(wires_f)], scattered(31))
    stand = scaled_baseline_boxes(BASE_BOXES, 1.8, 0.4, ["stand", "stand", "stand"])[2]
    stand["name"] = "cymbal stand"
    reach = [[0.5, 0.6, 0.3],   # cross-stick
             [0.02, 0.1, 1.0],  # hi-hat closed
             [0.02, 0.1, 1.0],  # hi-hat open
             [0.03, 0.05, 1.0],  # ride
             [0.6, 1.0, 0.1],   # snare
             [1.0, 0.05, 0.05]]  # kick
    return dict(name="kit", world="rule-britannia-kit-v1", cells=cells, boxes=[shells, wires, stand], reach=reach,
                room_scale=1.5, position=0.10, bridge_rate=[1.0, 1.0, 1.0], bridge_weight=1.0,
                intention="A small jazz kit in a club: kick, snare (brushed or struck), cross-stick, hi-hat closed and open, ride. The wires are a sympathetic radiator; nothing rattles.")


if __name__ == "__main__":
    btl.HERE = HERE  # the assembler writes <name>.plan.json / .design.json next to this script
    for make in (walker, vibes, kit):
        btl.build(make())
