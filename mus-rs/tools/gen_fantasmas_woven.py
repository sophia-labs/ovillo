#!/usr/bin/env python
"""Los Fantasmas del Telar — the WOVEN version.

Same piece, same braid mathematics, same timeline (imported from
gen_fantasmas.build_timeline — one source of truth), but the voices are
the real instrument: sustained bowed cello/violin-bodied weave tones,
mandolin-bodied breath halos, an extreme-weave breath sombra, and weave
plucks for the Z3 cells and the reveal — all rendered by the production
engine via `mus render --stems`, looped with equal-power crossfades, and
voiced by the holonomy envelopes.

Prereq (tone bank):
  mus render <scratch>/fantasmas_tones.mus --base aigua \
      --stems <scratch>/fantasmas_tone_stems
Run:
  .venv/bin/python mus-rs/tools/gen_fantasmas_woven.py \
      --tones <scratch>/fantasmas_tone_stems [--out DIR]
Then:
  mus scene-render <out>/fantasmas_tejido_scene.json --root <out>/stems
"""

from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path

import numpy as np
import soundfile as sf

sys.path.insert(0, str(Path(__file__).resolve().parent))
from gen_fantasmas import SR, VOICE_HZ, build_timeline, render_envelope  # noqa: E402

# tone-bank stem per logical voice
VOICE_TONE = {f"telar_{v}": f"t{v}" for v in range(8)}
VOICE_TONE["sombra"] = "sb"
STATE_PLUCK = {0: "zD", 1: "zF", 2: "zA"}  # +, -, link (D4, F4, A3)


def load_tone(tones_dir: Path, name: str) -> np.ndarray:
    audio, sr = sf.read(tones_dir / f"{name}.wav", dtype="float64")
    assert sr == SR, f"{name}: {sr} != {SR}"
    return audio


def tile_crossfade(tone: np.ndarray, n_total: int, fade_s: float = 1.5) -> np.ndarray:
    """Loop a sustained tone to n_total with equal-power crossfades.

    Uses the steady middle of the tone (skips the attack after the first
    pass and the decay tail always) so the splice is bow against bow.
    """
    fade = int(fade_s * SR)
    head = int(0.4 * SR)
    tail = int(1.2 * SR)
    body = tone[: len(tone) - tail]
    loop = body[head:]
    out = np.zeros(n_total + len(body), dtype=np.float64)
    out[: len(body)] = body
    cursor = len(body) - fade
    ramp_up = np.sin(0.5 * math.pi * np.linspace(0, 1, fade)) if fade else None
    while cursor < n_total:
        seg = min(len(loop), len(out) - cursor)
        piece = loop[:seg].copy()
        f = min(fade, seg)
        piece[:f] *= ramp_up[:f]
        out[cursor : cursor + f] *= np.cos(0.5 * math.pi * np.linspace(0, 1, f))
        out[cursor : cursor + seg] += piece
        cursor += seg - fade
    return out[:n_total]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--tones", type=Path, required=True)
    parser.add_argument(
        "--out",
        type=Path,
        default=Path(__file__).resolve().parents[2]
        / "paper" / "ariadne-protected-nonabelian" / "fantasmas-tejido",
    )
    args = parser.parse_args()
    stems_dir = args.out / "stems"
    stems_dir.mkdir(parents=True, exist_ok=True)

    tl, report, total_seconds = build_timeline()
    n_total = int(total_seconds * SR)

    stems: dict[str, np.ndarray] = {}
    for voice, tone_name in VOICE_TONE.items():
        tone = load_tone(args.tones, tone_name)
        bed = tile_crossfade(tone, n_total)
        env = render_envelope(tl.tracks.get(voice, [(0, 0)]), n_total)
        gain = 0.55 if voice == "sombra" else 1.0
        stems[voice] = (bed * env * gain).astype(np.float32)

    # Z3 cells and the reveal: real weave plucks placed at event times.
    pluck_tones = {name: load_tone(args.tones, name) for name in ("zD", "zF", "zA", "rv")}
    hz_to_state = {293.665: 0, 349.228: 1, 220.0: 2}
    for name, events in tl.plucks.items():
        if name.startswith("cell_"):
            buf = np.zeros(n_total, dtype=np.float64)
            for when, hz, amp in events:
                state = hz_to_state[min(hz_to_state, key=lambda k: abs(k - hz))]
                wave = pluck_tones[STATE_PLUCK[state]]
                start = int(when * SR)
                end = min(start + len(wave), n_total)
                buf[start:end] += amp * wave[: end - start]
            stems[name] = buf.astype(np.float32)
    # the reveal: one strum of the woven reveal chord, scaled by how much
    # of the final voicing survives (sum of pluck amplitudes)
    reveal_events = [e for k, v in tl.plucks.items() if k.startswith("telar_") for e in v]
    if reveal_events:
        when = min(e[0] for e in reveal_events)
        weight = sum(e[2] for e in reveal_events)
        wave = pluck_tones["rv"]
        buf = np.zeros(n_total, dtype=np.float64)
        start = int(when * SR)
        end = min(start + len(wave), n_total)
        buf[start:end] += min(weight, 1.2) * wave[: end - start]
        stems["revelacion"] = buf.astype(np.float32)

    for name, audio in stems.items():
        lift = 3.5 if name.startswith("telar_") or name == "sombra" else 1.3
        audio = audio * lift
        peak = float(np.abs(audio).max())
        if peak > 0.9:
            audio = audio * (0.9 / peak)
        sf.write(stems_dir / f"{name}.wav", audio.astype(np.float32), SR, subtype="FLOAT")

    objects = []
    for v in range(8):
        azimuth = math.radians(v * 45.0)
        objects.append({
            "objectId": f"telar_{v}", "label": f"voz {v}", "audio": f"telar_{v}.wav",
            "position": {"x": 2.6 * math.sin(azimuth), "y": 0.3 if v % 2 == 0 else 1.5,
                         "z": -2.6 * math.cos(azimuth)},
            "gainDb": 0, "reverbSend": 0.3,
        })
    objects.append({"objectId": "sombra", "label": "la sombra", "audio": "sombra.wav",
                    "position": {"x": 0.0, "y": 3.2, "z": -0.5}, "gainDb": -2, "reverbSend": 0.5})
    for cell in range(7):
        if f"cell_{cell}" in stems:
            azimuth = math.radians(-75 + cell * 25)
            objects.append({
                "objectId": f"cell_{cell}", "label": f"celda {cell}", "audio": f"cell_{cell}.wav",
                "position": {"x": 3.2 * math.sin(azimuth), "y": 0.0, "z": -3.2 * math.cos(azimuth)},
                "gainDb": -1, "reverbSend": 0.22,
            })
    if "revelacion" in stems:
        objects.append({"objectId": "revelacion", "label": "la revelacion", "audio": "revelacion.wav",
                        "position": {"x": 0.0, "y": 0.8, "z": -1.8}, "gainDb": 1, "reverbSend": 0.4})

    scene = {
        "sceneId": "fantasmas-del-telar-tejido",
        "title": "Los Fantasmas del Telar — version tejida",
        "sampleRate": SR,
        "durationSeconds": float(total_seconds + 4.0),
        "coordinateSystem": "+X right, +Y up, -Z front",
        "listener": {},
        "room": {"enabled": True, "decaySeconds": 2.6, "wet": 0.35, "damping": 0.45, "seed": 1949},
        "metadata": {"form": "passacaglia on a braid word — woven voices",
                     "voices": "bowed cello/violin bodies, breath halos, weave plucks",
                     "report": report},
        "objects": objects,
    }
    (args.out / "fantasmas_tejido_scene.json").write_text(json.dumps(scene, indent=2))
    print(json.dumps(report, indent=2, default=str))
    print(f"stems: {stems_dir}  scene: {args.out / 'fantasmas_tejido_scene.json'}  duration: {total_seconds:.1f}s")


if __name__ == "__main__":
    main()
