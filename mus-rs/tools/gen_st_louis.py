#!/usr/bin/env python3
"""St. Louis Blues (W.C. Handy, 1914, public domain) — one engine, five selves.

An ensemble where every part is the same string-network engine shaped
into a different instrument by parameters alone:
  contrabajo  — pluck, body_size 2.3, pos deep, felt pick (upright bass)
  guitarra    — pluck, three-note comp voicings, palm-lean, short ring
  mandolina   — pluck, body_size 0.6, detune 9 (double course), tremolo
  armonio     — weave pad, harmonic lattice (dimension 1.0), slow couple
  corneta     — bright pluck lead, blue notes scooped by real bends

Form: 12-bar blues chorus (melody) -> 8-bar habanera strain in G minor
(the tango section: mandolin tremolo over the dotted bass) -> final
blues chorus (lead + full band) with tag. Swing 2:1. ~95s, stereo.

Run:  .venv/bin/python mus-rs/tools/gen_st_louis.py <out.wav>
"""
import json
import math
import subprocess
import sys
from pathlib import Path

import numpy as np
import soundfile as sf
from scipy.signal import fftconvolve, lfilter

ROOT = Path(__file__).resolve().parents[2]
MUS = ROOT / "mus-rs/target/release/mus"
OUT = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / "st_louis.wav"
SR = 48_000
BPM = 88
BEAT = 60.0 / BPM
SW = 0.66  # swing: first eighth of the pair

service = subprocess.Popen(
    [str(MUS), "service"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True,
)
rpc = 0


def probe(params, midis, slot, gliss=1.0):
    global rpc
    rpc += 1
    service.stdin.write(json.dumps({
        "id": rpc, "method": "renderWeaveProbe",
        "params": {"params": params, "midis": midis, "slotSeconds": round(slot, 3),
                   "glissRatio": round(gliss, 4)},
    }) + "\n")
    service.stdin.flush()
    return np.fromfile(
        json.loads(service.stdout.readline())["result"]["pcm"]["path"], dtype=np.float32
    ).astype(np.float64)


BASS = {"synth": "pluck", "sus": 1.6, "damp": 0.55, "pos": 0.45, "pick": 0.12,
        "body": 0.7, "body_size": 2.3, "symp": 0, "stiff": 0.03, "courses": 7}
COMP = {"synth": "pluck", "sus": 0.9, "damp": 0.5, "pos": 0.22, "pick": 0.45,
        "body": 0.5, "symp": 0.1, "strum": 9}
MANDO = {"synth": "pluck", "sus": 1.1, "damp": 0.3, "pos": 0.14, "pick": 0.85,
         "body": 0.45, "body_size": 0.6, "detune": 9, "stiff": 0.1, "symp": 0}
PAD = {"synth": "weave", "sus": 4.0, "damp": 0.35, "couple": 0.12, "dimension": 1.0,
       "courses": 9, "chirality": 0.3, "orbit": 0.2, "orbit_depth": 0.3,
       "curvature": 0.1, "body": 0.45, "pos": 0.3, "pick": 0.2}
LEAD = {"synth": "pluck", "sus": 2.8, "damp": 0.28, "pos": 0.19, "pick": 0.7,
        "body": 0.45, "symp": 0.15, "stiff": 0.12, "tension": 8}
CHUCK = {"synth": "pluck", "sus": 0.25, "damp": 0.85, "pos": 0.1, "pick": 1.0,
         "pm": 1, "buzz": 0.35, "body": 0.3, "strum": 4}

L = []
place = lambda t, audio, gain=1.0, pan=0.0: L.append((t, audio, gain, pan))


def swing8(beat_frac):
    whole = int(beat_frac)
    frac = beat_frac - whole
    return whole + (0.0 if frac < 0.25 else SW if frac < 0.75 else 1.0) if False else beat_frac


def at(bar, beat):  # absolute seconds for (bar, beat) with swung eighths
    whole = math.floor(beat)
    frac = beat - whole
    if abs(frac - 0.5) < 0.01:
        frac = SW
    return (bar * 4 + whole + frac) * BEAT


# ---------------- 12-bar blues in G: chord roots per bar
BLUES = [43, 48, 43, 43, 48, 48, 43, 43, 50, 48, 43, 50]  # G C G G C C G G D C G D
SEVENTH = {43: [43, 53, 59], 48: [48, 58, 64], 50: [50, 60, 66]}  # 3-note shells


def walking(chorus_bar0, choruses=1):
    for c in range(choruses):
        for bar in range(12):
            root = BLUES[bar] - 12  # bass octave
            nxt = BLUES[(bar + 1) % 12] - 12
            line = [root, root + 4, root + 7, nxt - 1 if nxt > root else nxt + 1]
            for beat, midi in enumerate(line):
                place(at(chorus_bar0 + c * 12 + bar, beat), probe(BASS, [midi], 0.62),
                      0.85, -0.15)


def comping(chorus_bar0, choruses=1, energy=0.5):
    for c in range(choruses):
        for bar in range(12):
            shell = SEVENTH[BLUES[bar]]
            place(at(chorus_bar0 + c * 12 + bar, 1.5), probe(COMP, shell, 0.5),
                  energy, 0.35)
            place(at(chorus_bar0 + c * 12 + bar, 3), probe(COMP, shell, 0.6),
                  energy * 1.1, 0.35)


# Handy's first-strain melody, in contour (G blues; blue thirds scooped)
def blues_melody(bar0, gain=0.75, voice=LEAD, pan=-0.35, embellish=False):
    P = [
        (0, 0.0, 67, 1.0, 1.0), (0, 1.0, 64, 0.5, 1.0), (0, 1.5, 67, 1.5, 1.0),
        (0, 3.0, 64, 1.0, 1.0),
        (1, 0.0, 62, 0.5, 1.0), (1, 0.5, 59, 0.5, 1.059), (1, 1.0, 57, 1.0, 1.0),
        (1, 2.0, 55, 2.0, 1.0),
        (3, 2.0, 55, 0.5, 1.0), (3, 2.5, 58, 1.5, 1.059),
        (4, 0.0, 60, 1.0, 1.0), (4, 1.0, 64, 0.5, 1.0), (4, 1.5, 60, 1.5, 1.0),
        (4, 3.0, 58, 1.0, 1.122),
        (5, 0.0, 57, 1.0, 1.0), (5, 1.0, 55, 3.0, 1.0),
        (7, 2.0, 55, 0.5, 1.0), (7, 2.5, 59, 0.5, 1.0), (7, 3.0, 62, 1.0, 1.0),
        (8, 0.0, 66, 2.0, 1.0), (8, 2.0, 62, 1.0, 1.0), (8, 3.0, 60, 1.0, 1.0),
        (9, 0.0, 58, 1.5, 1.059), (9, 1.5, 57, 0.5, 1.0), (9, 2.0, 55, 2.0, 1.0),
        (10, 0.0, 55, 1.0, 1.0), (10, 1.0, 59, 0.5, 1.0), (10, 1.5, 62, 0.5, 1.0),
        (10, 2.0, 67, 2.0, 1.0),
        (11, 0.0, 62, 1.0, 1.0), (11, 1.0, 59, 0.5, 1.0), (11, 1.5, 57, 0.5, 1.0),
        (11, 2.0, 55, 2.0, 1.0),
    ]
    for bar, beat, midi, dur, gliss in P:
        start_ratio = 1 / gliss if gliss > 1.0 else 1.0
        base = midi - 1 if gliss > 1.0 else midi
        place(at(bar0 + bar, beat), probe(voice, [base], dur * BEAT * 1.05, gliss),
              gain, pan)
        if embellish and dur >= 2.0:
            for k in range(3):
                place(at(bar0 + bar, beat + dur - 0.75 + k * 0.25),
                      probe({**voice, "sus": 1.0}, [midi + 12], 0.2), gain * 0.3, 0.3)


# ---------------- habanera strain (G minor, the tango section)
def habanera(bar0, bars=8):
    GM = [[43, 55, 58, 62], [43, 55, 58, 62], [50, 54, 57, 60], [43, 55, 58, 62]]
    for bar in range(bars):
        # the dotted habanera bass: G . . D | G D
        root = 31 if bar % 4 != 2 else 38
        place(at(bar0 + bar, 0.0), probe(BASS, [root], 0.9), 0.9, -0.15)
        place(at(bar0 + bar, 0.75), probe(BASS, [root + 12], 0.4), 0.6, -0.15)
        place(at(bar0 + bar, 2.0), probe(BASS, [root + 7], 0.62), 0.8, -0.15)
        place(at(bar0 + bar, 3.0), probe(BASS, [root + 12], 0.62), 0.7, -0.15)
        pad_chord = GM[bar % 4]
        if bar % 2 == 0:
            place(at(bar0 + bar, 0), probe(PAD, pad_chord, 8 * BEAT * 0.95), 0.4, 0.0)
    # mandolin melody, tremolo on the long tones (Gm strain contour)
    MEL = [
        (0, 0.0, 74, 3.0), (0, 3.0, 72, 1.0), (1, 0.0, 70, 2.0), (1, 2.0, 67, 2.0),
        (2, 0.0, 69, 3.0), (2, 3.0, 70, 1.0), (3, 0.0, 67, 4.0),
        (4, 0.0, 74, 3.0), (4, 3.0, 72, 1.0), (5, 0.0, 70, 2.0), (5, 2.0, 67, 2.0),
        (6, 0.0, 66, 3.0), (6, 3.0, 69, 1.0), (7, 0.0, 67, 4.0),
    ]
    for bar, beat, midi, dur in MEL:
        if dur >= 2.0:  # tremolo
            strikes = int(dur * BEAT / 0.115)
            for k in range(strikes):
                place(at(bar0 + bar, beat) + k * 0.115,
                      probe(MANDO, [midi], 0.35), 0.4 * (1 - 0.25 * (k % 2)), 0.45)
        else:
            place(at(bar0 + bar, beat), probe(MANDO, [midi], dur * BEAT), 0.5, 0.45)


# ---------------- the arrangement
walking(0, 1)
comping(0, 1, 0.45)
blues_melody(0, 0.72, LEAD, -0.35)

habanera(12, 8)

walking(20, 1)
comping(20, 1, 0.6)
blues_melody(20, 0.8, LEAD, -0.35, embellish=True)
for bar in range(20, 32):  # muted backbeat chucks
    for beat in (1, 3):
        place(at(bar, beat), probe(CHUCK, [43, 50, 55], 0.3), 0.3, 0.1)

# tag: last four bars again, ritard by stretching placement
TAG = [(0, 55), (1, 59), (2, 62), (3, 67)]
t_tag = at(32, 0)
for k, (i, midi) in enumerate(TAG):
    stretch = 1.0 + k * 0.12
    place(t_tag + i * BEAT * stretch, probe(LEAD, [midi], 1.4), 0.7, -0.2)
    place(t_tag + i * BEAT * stretch, probe(BASS, [midi - 24], 1.4), 0.8, -0.1)
place(t_tag + 4.6 * BEAT, probe({**COMP, "strum": 40, "sus": 6}, [43, 50, 55, 59, 62], 8),
      0.9, 0.0)
place(t_tag + 4.6 * BEAT, probe(BASS, [31], 8), 0.9, -0.1)

# ---------------- stereo assembly + room
total = int((t_tag + 12) * SR)
mix = np.zeros((total, 2))
for t0, audio, gain, pan in L:
    start = int(round(t0 * SR))
    if start >= total:
        continue
    seg = (audio * gain)[: total - start]
    theta = (pan + 1) * math.pi / 4
    mix[start:start + len(seg), 0] += seg * math.cos(theta)
    mix[start:start + len(seg), 1] += seg * math.sin(theta)

rng = np.random.default_rng(1914)
impulse_t = np.arange(int(0.9 * SR)) / SR
for channel in range(2):
    impulse = rng.standard_normal(len(impulse_t)) * np.exp(-impulse_t * 5.5)
    impulse = lfilter([1 - 0.68], [1, -0.68], impulse)
    impulse[0] = 0.0
    wet = fftconvolve(mix[:, channel], impulse * 0.05)[:total]
    mix[:, channel] = mix[:, channel] * 0.9 + wet

mix *= 0.93 / np.abs(mix).max()
sf.write(OUT, mix.astype(np.float32), SR, subtype="PCM_16")
print(f"{OUT}  {total/SR:.1f}s  {len(L)} events")
service.stdin.close()
service.terminate()
