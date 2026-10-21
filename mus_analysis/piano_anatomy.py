"""Piano anatomy compiler: a grand piano's strings, hammers, dampers and soundboard as Ariadne data.

This is the reference (oracle) half of the piano described in
``SPEC-ARIADNE-PIANO.md``. It writes ``profiles/ariadne-piano-v1.anatomy.json``
and its derivation receipt. ``mus render`` composes, for each score, one
``ariadne-world-plan-v3`` from this anatomy (``mus-engine`` ``ariadne_piano.rs``):

* for every note the score plays, its unison (one to three strings), each a
  ``modal_space`` body whose map is ``stiff_wire``: the runtime computes the
  partials from the speaking length, core diameter, linear density, Young's
  modulus and tension written here (``mus-ariadne`` ``configuration/space.rs``),
  with a felt damper resting on it (``law/damper.rs``);
* one hammer body per note (``law/hammer.rs``) striking its unison;
* the soundboard: the lowest modes of a ribbed orthotropic spruce plate on the
  outline of the same piano, with its two bridges, computed here by finite
  differences and split 24 modes per Ariadne body (exact: a linear plate's
  modes are orthogonal, they meet only through the bridges);
* the bridge: one reciprocal edge per string and board body, whose rate is
  derived (the string's moving end, ``StiffWireSpec::moving_end_weights``,
  against the board's mass-normalized mode shapes at the string's bridge pin).

Claim discipline
----------------
It is a piano-shaped Ariadne world, not a model of a particular piano. The
scale (speaking lengths, strike distances, string counts, linear densities,
wire diameters) is the published scale of the Estonia "Parlour" grand
(Engelbrecht, Magi & Stulov 1999, Table 1), labelled ``manufacturer``; the
soundboard outline is traced from that paper's Fig. 1 (``design-hypothesis``);
the soundboard wood, thickness, rib spacing and loss are Ege, Boutillon &
Rebillat 2013 (``measured``); hammer masses and head radii are Stulov 1995
Table 1 (``measured``); the C4 felt law (K, p) is the one Stulov 1995 quotes
from Chaigne & Askenfelt (``measured``); everything else is labelled ``fit``
or ``design-hypothesis`` in the receipt.
"""

from __future__ import annotations

import argparse
import json
import math
from dataclasses import asdict, dataclass
from hashlib import sha256
from pathlib import Path

import numpy as np

try:
    from scipy import sparse
    from scipy.sparse import linalg as splinalg
except ImportError as exc:  # pragma: no cover - only without the audio extra
    raise ImportError("piano anatomy needs the project 'audio' extra (numpy, scipy)") from exc

MODE_COUNT = 24          # mus-ariadne ir::MODE_COUNT
SPACE_COURSES = 8
SAMPLE_RATE = 48_000.0
SPEED_OF_SOUND = 343.0
AIR_DENSITY = 1.204
STEEL_DENSITY = 7860.0   # Engelbrecht et al. 1999, eq. 1
LN_1000 = math.log(1000.0)

# ---------------------------------------------------------------- the scale

# Engelbrecht, Magi & Stulov, "Grand piano manufacturing in Estonia: the problem
# of piano scaling", Proc. Estonian Acad. Sci. Eng. 5 (1999) 155-167,
# doi:10.3176/eng.1999.2.06, Table 1 "Scale of the medium-sized piano Parlour",
# transcribed from the scanned page: n, L (mm), l (mm, hammer to the nearest
# string end), N (strings in the choir), T (N, per string), mu (g/m), d1 (core,
# mm), d2, d3 (winding wires, mm; 0 = none).
SCALE = [
    (1, 1415.0, 175.8, 1, 1306.8, 215.75, 1.350, 1.000, 1.700),
    (2, 1399.0, 173.8, 1, 1292.9, 194.49, 1.300, 0.900, 1.650),
    (3, 1383.0, 171.6, 1, 1261.2, 172.98, 1.300, 0.700, 1.650),
    (4, 1366.0, 169.5, 1, 1268.2, 158.90, 1.250, 0.650, 1.600),
    (5, 1350.0, 167.5, 1, 1246.0, 142.36, 1.225, 0.600, 1.600),
    (6, 1333.0, 165.4, 1, 1228.3, 128.23, 1.225, 0.500, 1.450),
    (7, 1316.0, 163.1, 1, 1202.0, 114.72, 1.225, 0.400, 1.400),
    (8, 1298.0, 160.8, 1, 1197.2, 104.66, 1.200, 1.700, 0.0),
    (9, 1281.0, 158.7, 1, 1188.7, 95.05, 1.175, 1.600, 0.0),
    (10, 1263.0, 156.5, 1, 1188.9, 87.11, 1.175, 1.500, 0.0),
    (11, 1251.0, 154.8, 2, 840.7, 55.93, 1.025, 1.150, 0.0),
    (12, 1233.0, 152.6, 2, 801.2, 48.89, 1.000, 1.050, 0.0),
    (13, 1215.0, 150.4, 2, 821.0, 45.96, 1.000, 1.000, 0.0),
    (14, 1196.0, 148.0, 2, 821.5, 42.28, 0.975, 0.950, 0.0),
    (15, 1178.0, 145.6, 2, 781.3, 36.93, 0.975, 0.850, 0.0),
    (16, 1159.0, 143.3, 2, 790.0, 34.37, 0.975, 0.800, 0.0),
    (17, 1140.0, 140.9, 2, 796.1, 31.89, 0.975, 0.750, 0.0),
    (18, 1121.0, 138.6, 2, 781.1, 28.83, 0.950, 0.700, 0.0),
    (19, 1102.0, 136.0, 2, 780.5, 26.56, 0.950, 0.650, 0.0),
    (20, 1083.0, 133.7, 2, 776.8, 24.38, 0.950, 0.600, 0.0),
    (21, 1273.0, 157.2, 2, 763.5, 15.45, 0.975, 0.350, 0.0),
    (22, 1261.0, 155.7, 2, 750.7, 13.79, 0.975, 0.300, 0.0),
    (23, 1248.0, 154.1, 2, 731.8, 12.23, 0.975, 0.250, 0.0),
    (24, 1233.0, 152.0, 2, 705.7, 10.76, 0.975, 0.200, 0.0),
    (25, 1218.0, 150.2, 2, 743.8, 10.36, 0.950, 0.200, 0.0),
    (26, 1201.0, 148.1, 3, 612.2, 7.81, 1.125, 0.0, 0.0),
    (27, 1180.0, 145.5, 3, 605.7, 7.13, 1.075, 0.0, 0.0),
    (28, 1153.0, 142.0, 3, 649.1, 7.13, 1.075, 0.0, 0.0),
    (29, 1120.0, 137.9, 3, 625.1, 6.49, 1.025, 0.0, 0.0),
    (30, 1080.0, 133.0, 3, 652.4, 6.49, 1.025, 0.0, 0.0),
    (31, 1034.0, 127.3, 3, 638.8, 6.17, 1.000, 0.0, 0.0),
    (32, 985.0, 121.3, 3, 650.7, 6.17, 1.000, 0.0, 0.0),
    (33, 939.0, 115.5, 3, 631.0, 5.87, 0.975, 0.0, 0.0),
    (34, 890.0, 109.5, 3, 636.3, 5.87, 0.975, 0.0, 0.0),
    (35, 840.0, 103.3, 3, 636.2, 5.87, 0.975, 0.0, 0.0),
    (36, 793.0, 97.5, 3, 636.4, 5.87, 0.975, 0.0, 0.0),
    (37, 752.0, 92.5, 3, 642.4, 5.87, 0.975, 0.0, 0.0),
    (38, 713.0, 87.6, 3, 648.2, 5.87, 0.975, 0.0, 0.0),
    (39, 675.0, 82.9, 3, 652.1, 5.87, 0.975, 0.0, 0.0),
    (40, 638.0, 78.4, 3, 654.0, 5.87, 0.975, 0.0, 0.0),
    (41, 604.0, 74.2, 3, 624.6, 5.57, 0.950, 0.0, 0.0),
    (42, 570.0, 69.9, 3, 624.4, 5.57, 0.950, 0.0, 0.0),
    (43, 541.0, 66.4, 3, 631.4, 5.57, 0.950, 0.0, 0.0),
    (44, 511.0, 62.7, 3, 632.2, 5.57, 0.950, 0.0, 0.0),
    (45, 481.0, 58.9, 3, 628.8, 5.57, 0.950, 0.0, 0.0),
    (46, 455.0, 54.4, 3, 631.6, 5.57, 0.950, 0.0, 0.0),
    (47, 428.0, 49.8, 3, 627.3, 5.57, 0.950, 0.0, 0.0),
    (48, 408.0, 46.4, 3, 639.8, 5.57, 0.950, 0.0, 0.0),
    (49, 381.0, 42.3, 3, 626.3, 5.57, 0.950, 0.0, 0.0),
    (50, 363.0, 39.3, 3, 638.1, 5.57, 0.950, 0.0, 0.0),
    (51, 345.0, 36.5, 3, 647.0, 5.57, 0.950, 0.0, 0.0),
    (52, 328.0, 34.0, 3, 656.4, 5.57, 0.950, 0.0, 0.0),
    (53, 312.0, 31.6, 3, 632.1, 5.28, 0.925, 0.0, 0.0),
    (54, 298.0, 29.1, 3, 647.2, 5.28, 0.925, 0.0, 0.0),
    (55, 284.0, 26.7, 3, 659.8, 5.28, 0.925, 0.0, 0.0),
    (56, 270.0, 24.5, 3, 669.4, 5.28, 0.925, 0.0, 0.0),
    (57, 256.0, 22.5, 3, 639.4, 5.00, 0.900, 0.0, 0.0),
    (58, 244.0, 20.8, 3, 652.0, 5.00, 0.900, 0.0, 0.0),
    (59, 231.0, 19.1, 3, 656.0, 5.00, 0.900, 0.0, 0.0),
    (60, 218.0, 17.4, 3, 655.7, 5.00, 0.900, 0.0, 0.0),
    (61, 208.0, 16.2, 3, 633.4, 4.73, 0.875, 0.0, 0.0),
    (62, 197.0, 14.9, 3, 637.7, 4.73, 0.875, 0.0, 0.0),
    (63, 187.0, 13.7, 3, 645.0, 4.73, 0.875, 0.0, 0.0),
    (64, 177.0, 12.6, 3, 648.6, 4.73, 0.875, 0.0, 0.0),
    (65, 167.0, 11.6, 3, 648.1, 4.73, 0.875, 0.0, 0.0),
    (66, 158.0, 10.7, 3, 651.2, 4.73, 0.875, 0.0, 0.0),
    (67, 150.0, 9.9, 3, 658.8, 4.73, 0.875, 0.0, 0.0),
    (68, 141.0, 9.1, 3, 653.4, 4.73, 0.875, 0.0, 0.0),
    (69, 132.0, 8.3, 3, 642.8, 4.73, 0.875, 0.0, 0.0),
    (70, 125.0, 7.7, 3, 647.0, 4.73, 0.875, 0.0, 0.0),
    (71, 119.0, 7.2, 3, 658.2, 4.73, 0.875, 0.0, 0.0),
    (72, 112.0, 6.6, 3, 654.4, 4.73, 0.875, 0.0, 0.0),
    (73, 106.0, 6.1, 3, 658.0, 4.73, 0.875, 0.0, 0.0),
    (74, 101.0, 5.7, 3, 670.5, 4.73, 0.875, 0.0, 0.0),
    (75, 95.0, 5.2, 3, 665.8, 4.73, 0.875, 0.0, 0.0),
    (76, 90.0, 4.9, 3, 670.8, 4.73, 0.875, 0.0, 0.0),
    (77, 84.0, 4.5, 3, 655.9, 4.73, 0.875, 0.0, 0.0),
    (78, 79.0, 4.1, 3, 651.2, 4.73, 0.875, 0.0, 0.0),
    (79, 74.0, 3.8, 3, 641.3, 4.73, 0.875, 0.0, 0.0),
    (80, 70.0, 3.5, 3, 644.1, 4.73, 0.875, 0.0, 0.0),
    (81, 66.0, 3.2, 3, 606.6, 4.46, 0.850, 0.0, 0.0),
    (82, 63.0, 3.0, 3, 620.4, 4.46, 0.850, 0.0, 0.0),
    (83, 59.0, 2.8, 3, 575.3, 4.20, 0.825, 0.0, 0.0),
    (84, 56.0, 2.6, 3, 581.7, 4.20, 0.825, 0.0, 0.0),
    (85, 54.0, 2.5, 3, 571.0, 3.95, 0.800, 0.0, 0.0),
    (86, 52.0, 2.4, 3, 594.3, 3.95, 0.800, 0.0, 0.0),
    (87, 51.0, 2.3, 3, 602.2, 3.71, 0.775, 0.0, 0.0),
    (88, 50.0, 2.2, 3, 649.7, 3.71, 0.775, 0.0, 0.0),
]

NOTE_NAMES = ["C", "C#", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"]


def note_name(midi: int) -> str:
    return f"{NOTE_NAMES[midi % 12]}{midi // 12 - 1}"


def key_frequency(n: int) -> float:
    """Equal temperament, A4 = 440 Hz (key n = 49)."""
    return 440.0 * 2.0 ** ((n - 49) / 12.0)


@dataclass(frozen=True)
class PianoDesign:
    """Every number that is not the published scale. Units SI."""

    # Wire (design-hypothesis unless noted).
    youngs_modulus: float = 2.0e11              # steel music wire (handbook value)
    decay_constant: float = 0.25                # sigma = a + b omega^2, the form of Chaigne & Askenfelt 1994
    decay_quadratic: float = 6.0e-9
    band_limit: float = 0.45 * SAMPLE_RATE      # partials kept (the runtime's band is 0.48 fs)
    unison_cents: tuple[float, ...] = (0.0, 0.6, -0.5)   # a tuner's unison, within a cent
    # Hammer: Stulov 1995 Table 1 (measured): head mass 13 / 10.6 / 8.2 g and head
    # radius 17 / 11 / 5 mm at keys 1 / 37 / 73, both linear in key number. The table's C4
    # column is the hammer whose felt law (below) was measured, 8.9 g: the masses follow the
    # line's shape scaled to pass through it (design-hypothesis).
    hammer_mass_points: tuple[tuple[int, float], ...] = ((1, 13.0e-3), (37, 10.6e-3), (73, 8.2e-3))
    hammer_mass_c4: float = 8.9e-3
    hammer_radius_points: tuple[tuple[int, float], ...] = ((1, 17.0e-3), (37, 11.0e-3), (73, 5.0e-3))
    # C4 felt law, F = K delta^p with K = 142.3 N/mm^2.5, p = 2.5 (Stulov 1995 quoting
    # Chaigne & Askenfelt, measured). Elsewhere: p rises 0.5 over three octaves toward the
    # treble and the force at 1 mm follows the Hertz-like geometry factor sqrt(rR/(r+R))/R of
    # Stulov's eq. 18 (design-hypothesis).
    felt_force_c4_at_1mm: float = 142.3
    felt_exponent_c4: float = 2.5
    felt_exponent_per_key: float = 0.5 / 36.0
    felt_exponent_range: tuple[float, float] = (2.2, 3.2)
    felt_relaxation: float = 1.0e-5             # alpha in K alpha d(delta^p)/dt (design-hypothesis)
    contact_depth: float = 0.5e-3               # felt width = chord of the head at this compression
    letoff: float = 2.0e-3                      # escapement below the string (design-hypothesis)
    catch: float = 8.0e-3                       # backcheck distance (design-hypothesis)
    # Damper (design-hypothesis): the pad centre at 1.32 x the strike distance puts a node
    # under it every 7th partial on A#4, and its resistance damps the fundamental at
    # 10 /s (amplitude, time constant 0.1 s): both after Bank 2000, Sec. 6.1, whose measured
    # A#4 damping takes the partials in 0.05 to 0.15 s and leaves every 7th ringing.
    damper_offset: float = 1.32
    damper_length: float = 0.015
    damper_fundamental_rate: float = 10.0
    damper_points: int = 4
    top_damped_key: int = 69                    # F6: the top 19 keys have no dampers (common practice)


# ---------------------------------------------------------------- the soundboard

@dataclass(frozen=True)
class Soundboard:
    """A ribbed orthotropic spruce plate on the Parlour outline. Units SI."""

    # Ege, Boutillon & Rebillat, J. Sound Vib. 332 (2013), arXiv:1212.3068, Table 1 (spruce,
    # after Berthaut 2004) and text: ribs every 13 cm, loss factor ~2 % below 1.2 kHz. Their
    # upright's board is 8 mm; a grand's is about 9 mm at its centre (design-hypothesis, taper
    # ignored).
    e_long: float = 11.5e9          # along the grain (y here)
    e_radial: float = 0.47e9        # across the grain (x here)
    g_lr: float = 0.5e9
    nu_lr: float = 0.005
    density: float = 392.0
    thickness: float = 0.009
    rib_spacing: float = 0.13
    loss_factor: float = 0.02
    # Ribs run across the grain (along x), spruce glued to the board: each bends with the strip
    # of board under it (a composite section, the board transformed by E_R/E_L). 25 x 25 mm,
    # the section of a grand's ribs at their centre, taper ignored (design-hypothesis). (Ege et
    # al. find their upright's board isotropic below 1.1 kHz; a grand's ribs are deeper.)
    rib_width: float = 0.025
    rib_height: float = 0.025
    # Bridge (design-hypothesis): a laminated maple beam, 32 x 35 mm, glued along the string ends.
    bridge_width: float = 0.032
    bridge_height: float = 0.035
    bridge_modulus: float = 12.0e9
    bridge_density: float = 700.0
    # Grid: across the grain (x) and along it (y).
    dx: float = 0.010
    dy: float = 0.013
    # Outline traced from Engelbrecht et al. 1999 Fig. 1 (1480 x 1320 mm), x from the spine
    # side, y from the front (keyboard) edge toward the tail; design-hypothesis.
    outline: tuple[tuple[float, float], ...] = (
        (0.040, 0.000), (0.000, 1.110), (0.040, 1.215), (0.184, 1.302), (0.365, 1.320),
        (0.563, 1.311), (0.726, 1.276), (0.852, 1.197), (0.942, 1.058), (1.014, 0.830),
        (1.105, 0.656), (1.213, 0.516), (1.339, 0.411), (1.430, 0.341), (1.466, 0.271),
        (1.480, 0.000),
    )
    # String layout (design-hypothesis, after Fig. 1): the hammers strike on the board's front
    # edge (y = 0), each string's agraffe its strike distance in front of it; plain strings
    # (notes 26..88) are struck from x = 0.31 to 1.40 m and run toward the tail, the longest
    # leaning 8 degrees toward the spine (none from note 50 up); the overstrung bass (notes
    # 1..25) ends on its own bridge.
    treble_x: tuple[float, float] = (0.310, 1.402)
    treble_lean_deg: float = 8.0
    treble_lean_until: int = 50
    bass_bridge: tuple[tuple[float, float], tuple[float, float]] = ((0.560, 1.130), (0.780, 0.840))
    unison_pitch: float = 0.0045    # string spacing within a choir at the bridge
    # Radiation: the board's own (baffled Rayleigh integral) on every second grid point.
    radiation_stride: int = 2
    # The listener (design-hypothesis): two far-field points 1 m away, 30 degrees off the
    # board's normal, leaning to the bass and to the treble side and a little toward the
    # keyboard (unit vectors' components in the board's plane).
    microphone_distance: float = 1.0
    microphone_directions: tuple[tuple[float, float], tuple[float, float]] = ((-0.447, -0.224), (0.447, -0.224))


def plate_rigidities(board: Soundboard) -> tuple[float, float]:
    """(D across the grain, D along it) of the bare plate (N m)."""
    nu_rl = board.nu_lr * board.e_radial / board.e_long
    den = 12.0 * (1.0 - board.nu_lr * nu_rl)
    return board.e_radial * board.thickness ** 3 / den, board.e_long * board.thickness ** 3 / den


def rib_rigidity(board: Soundboard) -> float:
    """Bending stiffness (N m^2) a glued rib adds: the rib plus the strip of board it carries
    (one rib spacing wide, transformed to the rib's modulus by E_R / E_L), about their common
    neutral axis, less the strip's own across-grain rigidity (the plate already counts it)."""
    b, h = board.rib_width, board.rib_height
    strip = board.rib_spacing * board.e_radial / board.e_long
    t = board.thickness
    area_rib, area_strip = b * h, strip * t
    # Heights from the board's lower face; the rib sits on top of the board.
    y_rib, y_strip = t + h / 2.0, t / 2.0
    y_bar = (area_rib * y_rib + area_strip * y_strip) / (area_rib + area_strip)
    inertia = (b * h ** 3 / 12.0 + area_rib * (y_rib - y_bar) ** 2
               + strip * t ** 3 / 12.0 + area_strip * (y_strip - y_bar) ** 2)
    d_x, _ = plate_rigidities(board)
    return board.e_long * inertia - d_x * board.rib_spacing


def _inside(points: np.ndarray, polygon: np.ndarray) -> np.ndarray:
    """Even-odd point-in-polygon test."""
    x, y = points[:, 0], points[:, 1]
    inside = np.zeros(len(points), bool)
    for (x0, y0), (x1, y1) in zip(polygon, np.roll(polygon, -1, axis=0)):
        crosses = ((y0 > y) != (y1 > y)) & (x < (x1 - x0) * (y - y0) / (y1 - y0 + 1e-300) + x0)
        inside ^= crosses
    return inside


def string_layout(board: Soundboard) -> dict[int, list[tuple[float, float]]]:
    """Bridge pin (x, y) of every string, note by note."""
    layout = {}
    for n, length_mm, strike_mm, strings, *_ in SCALE:
        length, strike = length_mm / 1000.0, strike_mm / 1000.0
        if n >= 26:
            x0, x1 = board.treble_x
            x = x0 + (x1 - x0) * (n - 26) / (88 - 26)
            lean = math.radians(board.treble_lean_deg) * max(0.0, (board.treble_lean_until - n) / (board.treble_lean_until - 26))
            # From the strike point: back to the agraffe (strike distance), then the full length.
            ax, ay = x + strike * math.sin(lean), -strike * math.cos(lean)
            centre = (ax - length * math.sin(lean), ay + length * math.cos(lean))
            direction = (math.cos(lean), math.sin(lean))
        else:
            (bx0, by0), (bx1, by1) = board.bass_bridge
            t = (n - 1) / 24.0
            centre = (bx0 + t * (bx1 - bx0), by0 + t * (by1 - by0))
            direction = (bx1 - bx0, by1 - by0)
            norm = math.hypot(*direction)
            direction = (direction[0] / norm, direction[1] / norm)
        offsets = [(s - (strings - 1) / 2.0) * board.unison_pitch for s in range(strings)]
        layout[n] = [(centre[0] + o * direction[0], centre[1] + o * direction[1]) for o in offsets]
    return layout


def _bilinear(points: np.ndarray, xs: np.ndarray, ys: np.ndarray, index: np.ndarray) -> sparse.csr_matrix:
    """Rows: bilinear interpolation of the grid (w = 0 off the board) at `points`."""
    dx, dy = xs[1] - xs[0], ys[1] - ys[0]
    rows, cols, data = [], [], []
    for k, (px, py) in enumerate(points):
        i = int(math.floor((px - xs[0]) / dx))
        j = int(math.floor((py - ys[0]) / dy))
        fx, fy = (px - xs[0]) / dx - i, (py - ys[0]) / dy - j
        for di, dj, w in ((0, 0, (1 - fx) * (1 - fy)), (1, 0, fx * (1 - fy)), (0, 1, (1 - fx) * fy), (1, 1, fx * fy)):
            ii, jj = i + di, j + dj
            if 0 <= jj < index.shape[0] and 0 <= ii < index.shape[1] and index[jj, ii] >= 0 and w > 0:
                rows.append(k)
                cols.append(index[jj, ii])
                data.append(w)
    return sparse.csr_matrix((data, (rows, cols)), shape=(len(points), int(index.max()) + 1))


def board_modes(board: Soundboard, count: int, layout: dict[int, list[tuple[float, float]]]):
    """The lowest `count` modes of the ribbed board with its bridges, mass-normalized."""
    polygon = np.array(board.outline)
    xs = np.arange(polygon[:, 0].min(), polygon[:, 0].max() + board.dx, board.dx)
    ys = np.arange(polygon[:, 1].min(), polygon[:, 1].max() + board.dy, board.dy)
    X, Y = np.meshgrid(xs, ys)
    grid = np.column_stack([X.ravel(), Y.ravel()])
    # Nodes on or outside the rim are clamped (w = 0): keep nodes half a cell inside.
    inside = _inside(grid, polygon).reshape(X.shape)
    shrink = np.ones_like(inside)
    for sx, sy in ((0.5, 0), (-0.5, 0), (0, 0.5), (0, -0.5)):
        shifted = grid + np.array([sx * board.dx, sy * board.dy])
        shrink &= _inside(shifted, polygon).reshape(X.shape)
    inside &= shrink
    index = -np.ones(X.shape, int)
    index[inside] = np.arange(inside.sum())
    n = int(inside.sum())
    rows, cols = np.nonzero(inside)

    def difference(dr: int, dc: int, h: float, only: np.ndarray | None = None) -> sparse.csr_matrix:
        """Second difference along (dr, dc). The rim is clamped: a node just off the board
        holds w = 0, and its own curvature, with the node beyond it mirrored (zero slope),
        is 2 w_inside / h^2 (an extra row per rim node)."""
        data, ri, ci = [], [], []
        ghosts: dict[tuple[int, int], int] = {}
        for k, (r, c) in enumerate(zip(rows, cols)):
            if only is not None and not only[k]:
                continue
            for off, w in ((-1, 1.0), (0, -2.0), (1, 1.0)):
                rr, cc = r + off * dr, c + off * dc
                j = index[rr, cc] if 0 <= rr < X.shape[0] and 0 <= cc < X.shape[1] else -1
                if j >= 0:
                    data.append(w / h ** 2); ri.append(k); ci.append(j)
                elif off != 0:
                    row = ghosts.setdefault((rr, cc), n + len(ghosts))
                    data.append(2.0 / h ** 2); ri.append(row); ci.append(k)
        return sparse.csr_matrix((data, (ri, ci)), shape=(n + len(ghosts), n))

    def cross() -> sparse.csr_matrix:
        data, ri, ci = [], [], []
        for k, (r, c) in enumerate(zip(rows, cols)):
            for dr, dc, w in ((1, 1, 1), (1, -1, -1), (-1, 1, -1), (-1, -1, 1)):
                rr, cc = r + dr, c + dc
                j = index[rr, cc] if 0 <= rr < X.shape[0] and 0 <= cc < X.shape[1] else -1
                if j >= 0:
                    data.append(w / (4 * board.dx * board.dy)); ri.append(k); ci.append(j)
        return sparse.csr_matrix((data, (ri, ci)), shape=(n, n))

    dxx = difference(0, 1, board.dx)
    dyy = difference(1, 0, board.dy)
    dxy = cross()
    h = board.thickness
    nu_rl = board.nu_lr * board.e_radial / board.e_long
    den = 12.0 * (1.0 - board.nu_lr * nu_rl)
    d_y = board.e_long * h ** 3 / den          # grain along y
    d_x = board.e_radial * h ** 3 / den
    d_1 = board.nu_lr * d_x
    d_k = board.g_lr * h ** 3 / 12.0
    area = board.dx * board.dy
    # The Poisson term is taken on the board's own nodes only.
    sxx, syy = dxx[:n], dyy[:n]
    stiffness = area * (d_x * (dxx.T @ dxx) + d_1 * (sxx.T @ syy + syy.T @ sxx) + d_y * (dyy.T @ dyy)
                        + 4.0 * d_k * (dxy.T @ dxy))
    mass = np.full(n, board.density * h * area)
    # Ribs: beams along x on the grid rows nearest every rib line.
    rib_area = board.rib_width * board.rib_height
    rib_ei = rib_rigidity(board)
    rib_rows = []
    y = ys[0] + board.rib_spacing / 2.0
    while y < ys[-1]:
        rib_rows.append(int(round((y - ys[0]) / board.dy)))
        y += board.rib_spacing
    on_rib = np.isin(rows, rib_rows)
    dxx_rib = difference(0, 1, board.dx, on_rib)
    stiffness = stiffness + board.dx * rib_ei * (dxx_rib.T @ dxx_rib)
    mass[on_rib] += board.density * rib_area * board.dx
    # Bridges: beams through the string ends, sampled every 5 mm.
    bridge_ei = board.bridge_modulus * board.bridge_width * board.bridge_height ** 3 / 12.0
    bridge_mass = board.bridge_density * board.bridge_width * board.bridge_height
    treble = np.array([np.mean(layout[k], axis=0) for k in range(26, 89)])
    bass = np.array([np.mean(layout[k], axis=0) for k in range(1, 26)])
    for path in (treble, bass):
        seg = np.cumsum(np.r_[0.0, np.linalg.norm(np.diff(path, axis=0), axis=1)])
        s = np.arange(0.0, seg[-1], 0.005)
        pts = np.column_stack([np.interp(s, seg, path[:, 0]), np.interp(s, seg, path[:, 1])])
        interp = _bilinear(pts, xs, ys, index)
        step = s[1] - s[0]
        second = sparse.diags([1.0, -2.0, 1.0], [0, 1, 2], shape=(len(pts) - 2, len(pts))) / step ** 2
        curvature = second @ interp
        stiffness = stiffness + bridge_ei * step * (curvature.T @ curvature)
        mass += np.asarray(interp.T @ np.full(len(pts), bridge_mass * step)).ravel()
    stiffness = sparse.csr_matrix(stiffness)
    w2, phi = splinalg.eigsh(stiffness, k=count, M=sparse.diags(mass), sigma=0.0, which="LM")
    order = np.argsort(w2)
    w2, phi = w2[order], phi[:, order]
    # eigsh with M returns M-orthonormal vectors: phi^T M phi = I (1/sqrt(kg)).
    points = np.column_stack([X[inside], Y[inside]])
    return dict(w2=w2, phi=phi, mass=mass, points=points, xs=xs, ys=ys, index=index, area=area,
                plate=dict(d_x=d_x, d_y=d_y, d_k=d_k, rib_ei=rib_ei, rib_rows=len(rib_rows),
                           grid_points=n, board_area=float(n * area), rho_h=board.density * h,
                           total_mass=float(mass.sum())))


def microphones(board: Soundboard, modes: dict, freq: np.ndarray) -> dict:
    """Far-field pressure (Pa at `microphone_distance`) at each listening direction as a
    readout of the modal state: with D_k = sum_i phi_k(x_i) exp(-i k_k u.x_i) dA (u: the
    direction's component in the board's plane, baffled Rayleigh integral),
    p = -(rho / (2 pi r)) sum_k (omega_k^2 Re(D_k) eta_k + omega_k Im(D_k) eta_k'), and Ariadne's
    state is eta = sqrt(dt) q, eta' = sqrt(dt) m. Each mode is taken at its own frequency."""
    points, area = modes["points"], modes["area"]
    omega = 2.0 * math.pi * freq
    k = omega / SPEED_OF_SOUND
    scale = -AIR_DENSITY * math.sqrt(1.0 / SAMPLE_RATE) / (2.0 * math.pi * board.microphone_distance)
    out = {}
    for side, (ux, uy) in zip(("left", "right"), board.microphone_directions):
        phase = np.exp(-1j * np.outer(points[:, 0] * ux + points[:, 1] * uy, k))
        d = (modes["phi"] * phase).sum(axis=0) * area
        out[side] = dict(displacement=scale * omega ** 2 * d.real, momentum=scale * omega * d.imag)
    return out


def radiation_sigma(board: Soundboard, modes: dict) -> np.ndarray:
    """Amplitude decay rate of each mode by its own radiation (baffled Rayleigh integral):
    sigma = rho c k^2 / (4 pi) sum_ij phi_i phi_j sinc(k r_ij) dA^2."""
    stride = board.radiation_stride
    index = modes["index"]
    keep = np.zeros(index.max() + 1, bool)
    rr, cc = np.nonzero(index >= 0)
    sel = (rr % stride == 0) & (cc % stride == 0)
    keep[index[rr[sel], cc[sel]]] = True
    pts = modes["points"][keep]
    d_area = modes["area"] * stride * stride
    r = np.sqrt(((pts[:, None, :] - pts[None, :, :]) ** 2).sum(-1))
    sigmas = np.empty(modes["w2"].size)
    for m, w2 in enumerate(modes["w2"]):
        k = math.sqrt(w2) / SPEED_OF_SOUND
        v = modes["phi"][keep, m]
        kernel = np.sinc(k * r / math.pi)
        sigmas[m] = AIR_DENSITY * SPEED_OF_SOUND * k * k / (4 * math.pi) * float(v @ kernel @ v) * d_area ** 2
    return sigmas


# ---------------------------------------------------------------- the anatomy

def _interp_points(points: tuple[tuple[int, float], ...], n: int) -> float:
    keys = [k for k, _ in points]
    values = [v for _, v in points]
    if n <= keys[0]:
        slope = (values[1] - values[0]) / (keys[1] - keys[0])
        return values[0] + slope * (n - keys[0])
    if n >= keys[-1]:
        slope = (values[-1] - values[-2]) / (keys[-1] - keys[-2])
        return values[-1] + slope * (n - keys[-1])
    return float(np.interp(n, keys, values))


def tuned_tension(design: PianoDesign, f1: float, length: float, mu: float, core: float) -> float:
    """The tension at which the pinned wire's first partial, f0 sqrt(1 + B), sounds `f1`
    (a tuner sets the pitch heard, not the flexible string's). With f0^2 = T / (4 mu L^2) and
    B = c / T, c = pi^3 E d^4 / (64 L^2): f1^2 = (T + c) / (4 mu L^2), so T = mu (2 L f1)^2 - c."""
    c = math.pi ** 3 * design.youngs_modulus * core ** 4 / (64.0 * length ** 2)
    return mu * (2.0 * f1 * length) ** 2 - c


def hammer_for(design: PianoDesign, n: int, outer_diameter: float) -> dict:
    """Head mass, felt law and felt width for key n."""
    mass = _interp_points(design.hammer_mass_points, n) * design.hammer_mass_c4 / _interp_points(design.hammer_mass_points, 40)
    radius = max(_interp_points(design.hammer_radius_points, n), 2.0e-3)
    lo, hi = design.felt_exponent_range
    p = min(max(design.felt_exponent_c4 + design.felt_exponent_per_key * (n - 40), lo), hi)

    def geometry(key: int, d_out: float) -> float:
        r, big = 0.5 * d_out, max(_interp_points(design.hammer_radius_points, key), 2.0e-3)
        return math.sqrt(r * big / (r + big)) / big

    c4_outer = SCALE[39][6] / 1000.0
    force_at_1mm = design.felt_force_c4_at_1mm * geometry(n, outer_diameter) / geometry(40, c4_outer)
    stiffness = force_at_1mm / (1.0e-3) ** p
    depth = design.contact_depth
    width = 2.0 * math.sqrt(max(2.0 * radius * depth - depth * depth, 0.0))
    return dict(mass_kg=mass, felt_stiffness=stiffness, felt_exponent=p,
                felt_relaxation_s=design.felt_relaxation, contact_width_m=width,
                letoff_m=design.letoff, catch_m=design.catch, head_radius_m=radius,
                felt_force_at_1mm_n=force_at_1mm)


def piano_anatomy(design: PianoDesign, board: Soundboard, exact_bodies: int = 9, sampled_bodies: int = 5):
    layout = string_layout(board)
    total = 30 * MODE_COUNT   # the 720 lowest plate modes (to about 6.4 kHz)
    modes = board_modes(board, total, layout)
    f_all = np.sqrt(modes["w2"]) / (2 * math.pi)
    exact = exact_bodies * MODE_COUNT
    # Above the exact band every `stride`-th mode stands for its neighbours: its bridge shape
    # carries sqrt(stride), so the band's mean bridge conductance and radiated power are kept.
    rest = len(f_all) - exact
    stride = max(1, rest // (sampled_bodies * MODE_COUNT))
    keep = list(range(exact)) + [exact + stride * j + stride // 2 for j in range(sampled_bodies * MODE_COUNT)]
    keep = [k for k in keep if k < len(f_all)]
    keep = keep[: (len(keep) // MODE_COUNT) * MODE_COUNT]
    weight = np.array([1.0 if k < exact else math.sqrt(stride) for k in keep])
    phi = modes["phi"][:, keep]
    freq = f_all[keep]
    sub = dict(modes, w2=modes["w2"][keep], phi=phi)
    sigma_rad = radiation_sigma(board, sub)
    sigma_wood = math.pi * freq * board.loss_factor
    t60 = LN_1000 / sigma_wood
    mics = microphones(board, sub, freq)
    bodies = []
    for b in range(len(keep) // MODE_COUNT):
        part = slice(b * MODE_COUNT, (b + 1) * MODE_COUNT)
        bodies.append(dict(
            id=f"board/{b}",
            frequency_hz=[float(x) for x in freq[part]],
            t60_seconds=[float(x) for x in t60[part]],
            radiation_rates_per_s=[float(x) for x in sigma_rad[part]],
            microphones={side: {q: [float(x) for x in mics[side][q][part]] for q in ("displacement", "momentum")}
                         for side in mics},
            stands_for=int(stride if b >= exact_bodies else 1),
        ))
    pins = [(n, s, xy) for n in range(1, 89) for s, xy in enumerate(layout[n])]
    at_pins = _bilinear(np.array([xy for _, _, xy in pins]), modes["xs"], modes["ys"], modes["index"]) @ phi
    at_pins = np.asarray(at_pins) * weight
    shapes = {(n, s): at_pins[i] for i, (n, s, _) in enumerate(pins)}
    notes = []
    for n, length_mm, strike_mm, strings, table_tension, mu_g, d1, d2, d3 in SCALE:
        length, mu = length_mm / 1000.0, mu_g / 1000.0
        f0 = key_frequency(n)
        tension = tuned_tension(design, f0, length, mu, d1 / 1000.0)
        outer = (d1 + 2.0 * d2 + 2.0 * d3) / 1000.0
        hammer = hammer_for(design, n, outer)
        damper = None
        if n <= design.top_damped_key:
            centre = design.damper_offset * strike_mm / length_mm
            # A viscous pad R over the fundamental's shape: sigma_1 = R <psi_1^2> / 2 with
            # psi_1^2 = 2 / (mu L) sin^2(pi x / L), averaged over the pad's points.
            half = 0.5 * design.damper_length / length
            xs = [centre - half + (j + 0.5) * 2 * half / design.damper_points for j in range(design.damper_points)]
            psi2 = np.mean([2.0 / (mu * length) * math.sin(math.pi * x) ** 2 for x in xs])
            damper = dict(center_fraction=centre, length_m=design.damper_length,
                          resistance_kg_s=2.0 * design.damper_fundamental_rate / psi2,
                          points=design.damper_points)
        wires = []
        for s in range(strings):
            cents = design.unison_cents[s]
            wires.append(dict(
                tension_n=tension * 2.0 ** (2.0 * cents / 1200.0),
                cents=cents,
                bridge_pin_m=[float(v) for v in layout[n][s]],
                board_shapes=[[float(v) for v in shapes[(n, s)][b * MODE_COUNT:(b + 1) * MODE_COUNT]]
                              for b in range(len(bodies))],
            ))
        notes.append(dict(
            key=n, midi=n + 20, name=note_name(n + 20), frequency_hz=f0,
            speaking_length_m=length, strike_fraction=strike_mm / length_mm,
            core_diameter_m=d1 / 1000.0, outer_diameter_m=outer, linear_density_kg_m=mu,
            table_tension_n=table_tension, wound=d2 > 0.0,
            hammer=hammer, damper=damper, wires=wires,
        ))
    anatomy = dict(
        schema="mus.ariadne.piano-anatomy.v1",
        sample_rate_hz=SAMPLE_RATE,
        wire=dict(youngs_modulus_pa=design.youngs_modulus, decay_constant_per_s=design.decay_constant,
                  decay_quadratic_s=design.decay_quadratic, band_limit_hz=design.band_limit,
                  air_density_kg_m3=AIR_DENSITY, speed_of_sound_m_s=SPEED_OF_SOUND),
        notes=notes,
        board=dict(bodies=bodies, room_weight=0.0, microphone_distance_m=board.microphone_distance,
                   microphone_directions=[list(d) for d in board.microphone_directions]),
        room=_room(),
    )
    return anatomy, modes, dict(freq=freq, sigma_rad=sigma_rad, sigma_wood=sigma_wood, stride=stride,
                                exact_top_hz=float(f_all[exact - 1]), sampled_top_hz=float(freq[-1]),
                                all_modes=int(len(f_all)))


def _normalized(v) -> list[float]:
    v = np.asarray(v, float)
    return [float(x) for x in v / np.linalg.norm(v)]


def _room() -> dict:
    """The steel-string's small room (mus_analysis/steel_string_anatomy.py), unchanged."""
    return dict(delays_seconds=[0.0061, 0.0093, 0.0127, 0.0171, 0.0219, 0.0283, 0.0397, 0.0613],
                scattering_direction=_normalized([1.0, -0.73, 0.88, -0.61, 0.79, -0.52, 0.69, -0.44]),
                radiation_left=_normalized([1.0, 0.62, 0.18, -0.31, -0.70, -0.54, 0.08, 0.48]),
                radiation_right=_normalized([0.17, -0.51, -0.72, -0.24, 0.36, 0.61, 0.31, -0.22]))


def receipt_for(design: PianoDesign, board: Soundboard, anatomy: dict, modes: dict, extra: dict) -> dict:
    d_x, d_y = plate_rigidities(board)
    rib_ei = rib_rigidity(board)
    notes = []
    for note in anatomy["notes"]:
        t = note["wires"][0]["tension_n"]
        b = math.pi ** 3 * design.youngs_modulus * note["core_diameter_m"] ** 4 / (64 * t * note["speaking_length_m"] ** 2)
        notes.append(dict(key=note["key"], name=note["name"], strings=len(note["wires"]),
                          tension_n=round(t, 1), table_tension_n=note["table_tension_n"],
                          tension_vs_table=round(t / note["table_tension_n"] - 1.0, 4),
                          inharmonicity_b=b, strike_ratio=round(1.0 / note["strike_fraction"], 2),
                          hammer_mass_g=round(note["hammer"]["mass_kg"] * 1e3, 2),
                          felt_exponent=round(note["hammer"]["felt_exponent"], 3),
                          felt_force_at_1mm_n=round(note["hammer"]["felt_force_at_1mm_n"], 1),
                          damper=note["damper"] is not None))
    freq = extra["freq"]
    density = {}
    for lo, hi in ((0, 250), (250, 500), (500, 1000), (1000, 1500), (1500, 2000), (2000, 3000), (3000, 4500), (4500, 6500)):
        f_all = np.sqrt(modes["w2"]) / (2 * math.pi)
        density[f"{lo}-{hi}"] = round(float(((f_all >= lo) & (f_all < hi)).sum()) / (hi - lo), 4)
    return dict(
        schema="mus.ariadne.piano-anatomy-receipt.v1",
        sources=dict(
            scale="Engelbrecht, Magi & Stulov, Proc. Estonian Acad. Sci. Eng. 5 (1999) 155-167, Table 1 (Estonia 'Parlour')",
            outline="same paper, Fig. 1 (1480 x 1320 mm), traced",
            soundboard="Ege, Boutillon & Rebillat, J. Sound Vib. 332 (2013), arXiv:1212.3068",
            hammers="Stulov, Proc. Estonian Acad. Sci. Eng. 1 (1995) 172-182, Table 1; C4 felt K, p quoted there from Chaigne & Askenfelt",
            hammer_law="Chabassier, Chaigne & Joly, ESAIM M2AN 48 (2014) 1241-1278, eqs. 4.5-4.8",
            damper="Bank, MSc thesis, HUT Acoustics Report 54 (2000), Sec. 6.1",
        ),
        evidence=dict(
            manufacturer=["speaking lengths", "strike distances", "strings per choir", "linear densities", "core and winding diameters"],
            measured=["soundboard E_L, E_R, G_LR, nu_LR, density, thickness, rib spacing, loss factor",
                      "hammer head masses and radii (keys 1, 37, 73)", "C4 felt K and p"],
            law=["tension from f0, L, mu", "partials, B and mode shapes (runtime stiff_wire)", "board modes (finite differences)",
                 "board radiation (baffled Rayleigh integral, diagonal in the modes)",
                 "far-field pressure at the listening points (Rayleigh integral per mode)",
                 "string radiation (thin-cylinder dipole, ka << 1)",
                 "bridge edge rate (moving string end against the board's mode shapes)"],
            fit=[],
            design_hypothesis=["Young's modulus of the wire", "string loss law", "unison detuning", "outline trace and string layout",
                               "rib section (glued, composite)", "bridge section and wood", "felt exponent and force across the keyboard", "felt relaxation",
                               "felt width", "letoff and catch", "damper position, length and resistance", "dampers stop at F6",
                               "every k-th board mode above the exact band", "listening points"],
        ),
        limits=[f"board: {len(anatomy['board']['bodies'])} Ariadne bodies, the {extra['all_modes']} lowest plate modes solved, "
                f"all below {extra['exact_top_hz']:.0f} Hz kept, every {extra['stride']}th above to {extra['sampled_top_hz']:.0f} Hz",
                "one transverse polarization; no longitudinal waves, no phantom partials",
                "no tension modulation (linear strings)", "no hammer shank, no action, no key bounce",
                "no sympathetic strings beyond the notes a score plays", "board flat: no crown, no downbearing",
                "rim clamped and rigid; case, lid and frame rigid; no lid",
                "radiation baffled; diagonal in the modes (no cross-radiation)", "no room: the World is heard dry"],
        board=dict(plate_rigidity_across_n_m=d_x, plate_rigidity_along_n_m=d_y, rib_rigidity_n_m2=rib_ei,
                   rib_smeared_rigidity_n_m=d_x + rib_ei / board.rib_spacing,
                   d_over_rho_h_m4_s2=d_y / (board.density * board.thickness),
                   grid=modes["plate"], lowest_hz=[round(float(x), 2) for x in np.sqrt(modes["w2"][:12]) / (2 * math.pi)],
                   modal_density_per_hz=density, exact_top_hz=extra["exact_top_hz"], sampled_top_hz=extra["sampled_top_hz"],
                   sampled_stride=extra["stride"],
                   radiation_share_by_band={f"{lo}-{hi}": round(float(np.mean(
                       (extra["sigma_rad"] / (extra["sigma_rad"] + extra["sigma_wood"]))[(freq >= lo) & (freq < hi)])), 4)
                       for lo, hi in ((0, 250), (250, 500), (500, 1000), (1000, 2000), (2000, 4000), (4000, 7000))
                       if ((freq >= lo) & (freq < hi)).any()}),
        design=asdict(design),
        soundboard=asdict(board),
        notes=notes,
    )


def canonical_digest(obj) -> str:
    return "sha256:" + sha256(json.dumps(obj, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("-o", "--out", default="profiles/ariadne-piano-v1.anatomy.json")
    args = ap.parse_args(argv)
    design, board = PianoDesign(), Soundboard()
    anatomy, modes, extra = piano_anatomy(design, board)
    receipt = receipt_for(design, board, anatomy, modes, extra)
    receipt["anatomy_digest"] = canonical_digest(anatomy)
    out = Path(args.out)
    out.write_text(json.dumps(anatomy, separators=(",", ":")) + "\n")
    out.with_name(out.name.replace(".anatomy.json", ".receipt.json")).write_text(json.dumps(receipt, indent=1) + "\n")
    print(f"wrote {out}: {len(anatomy['notes'])} notes, {len(anatomy['board']['bodies'])} board bodies, "
          f"board modes to {extra['sampled_top_hz']:.0f} Hz (exact to {extra['exact_top_hz']:.0f} Hz), "
          f"lowest {receipt['board']['lowest_hz'][:3]} Hz")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
