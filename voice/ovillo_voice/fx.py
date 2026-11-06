"""Character chains: make one throat into a cast. Ariadne first, then whatever it takes.

Each chain is a list of steps run in order on a take's dry.wav:
    ("body",   preset, {knob: value})        an Ovillo Body (ovillo-body render): physics, receipted
    ("shift",  formant_ratio, pitch_median_hz | None, range_factor)   Praat "Change gender" (PSOLA): NOT physics
    ("ff",     "ffmpeg -af filtergraph")     filters: band, EQ, drive, exciter ... NOT physics
    ("codec",  "opus8" | "mulaw" | "adpcm16")   a round trip through a real lossy codec: the crunch
    ("air",    {hiss, hum, crackle (dB)})    the transmission's own noise, mixed under
    ("key",    "noise" | "blip" | "click")   keying the radio: a burst before and a tail after
Every render writes renders/<take>/fx-<chain>/wet.wav and body.json, with the chain recorded step by step and
non_physics=true. The Body's receipt is kept as receipt.json. scene() lays a session's circled takes end to end.
"""
import json, pathlib, subprocess, tempfile, time, zlib
import numpy as np
from scipy import signal
from scipy.io import wavfile
from . import studio, ops, analysis

SR = 48000

# --- the cast ---------------------------------------------------------------------------------------------------------
CHAINS = {
    # HQ, far away: warm, older, certain. A long speaking tube in Ariadne, then a smaller, older woman's voice made of
    # Vera's, then an AM-ish link: mid-forward, compressed, a little saturated, Opus at 8 kbps, hiss and hum.
    "mom": [
        ("body", "radio", {"powered.level": 0.35, "flap.on": 0.0}),
        ("shift", 1.10, 168, 0.8),
        ("ff", "agate=threshold=0.015:ratio=3:range=0.1:attack=5:release=200,highpass=f=380,highpass=f=380,lowpass=f=3400,equalizer=f=600:t=q:w=1:g=-4,equalizer=f=1600:t=q:w=1.2:g=7,"
               "acompressor=threshold=-24dB:ratio=6:attack=5:release=120:makeup=6,volume=4dB,asoftclip=type=tanh"),
        ("air", {"hiss": -44, "hum": -52, "crackle": -42}),   # into the line, so the link shapes its own noise
        ("codec", "opus8"),
        ("key", "noise"),
    ],
    # The support line: young, bright, close on a digital headset. Ariadne's v body for sparkle, then up in pitch and
    # size, lively range, a presence peak and an exciter, a hair of chorus, then the PS1 sound chip's 4-bit ADPCM.
    "hot": [
        ("body", "v", {}),
        ("shift", 1.17, 208, 1.35),
        ("ff", "highpass=f=420,lowpass=f=6000,equalizer=f=3200:t=q:w=1.5:g=6,aexciter=amount=2:drive=6:freq=2500,"
               "chorus=0.6:0.9:12:0.25:0.3:2,acompressor=threshold=-22dB:ratio=4:attack=3:release=80:makeup=4"),
        ("codec", "adpcm16"),
        ("air", {"hiss": -52}),
        ("key", "blip"),
    ],
    # Veronica herself, on location: the close one. The gentle v body, a little more size and lift, a throat-mic band,
    # gentle compression. No codec: she's the clean voice the others come through.
    "vera": [
        ("body", "v", {"branch1.length_m": 0.0607, "branch2.length_m": 0.0366, "branch3.length_m": 0.0317,
                       "branch1.area_cm2": 1.5, "branch1.neck_cm2": 1.5, "branch2.area_cm2": 1.2, "branch2.neck_cm2": 1.2}),
        ("shift", 1.08, 152, 1.0),
        ("ff", "highpass=f=140,lowpass=f=7500,equalizer=f=250:t=q:w=1:g=-3,equalizer=f=2800:t=q:w=1.5:g=2,"
               "acompressor=threshold=-20dB:ratio=3:attack=8:release=150:makeup=3"),
    ],
    # The Gatekeeper on a field telephone: Ariadne's gatekeeper (a long throat, a huge open nose, a fur coat that eats
    # the lows, a rattling flap), then bigger and lower and flatter, then a filthy line: narrow, driven, bit-crushed,
    # mu-law at 8 kHz, crackle and mains hum.
    "gatekeeper": [
        ("body", "gatekeeper", {}),
        ("shift", 0.84, 78, 0.75),
        ("ff", "agate=threshold=0.02:ratio=6:range=0.03:attack=4:release=180,"   # shut the line between words before the drive lifts the room
               "highpass=f=330,lowpass=f=2700,equalizer=f=900:t=q:w=1:g=6,volume=9dB,asoftclip=type=hard,"
               "acrusher=bits=7:mix=0.35:mode=log,acompressor=threshold=-20dB:ratio=8:attack=2:release=60:makeup=3"),
        ("air", {"hiss": -40, "hum": -42, "crackle": -34}),   # into the line, before the codec
        ("codec", "mulaw"),
        ("key", "click"),
    ],
}
WHO = {"MOM VERONICA": "mom", "HOT VERONICA": "hot", "VERA": "vera", "VERONICA": "vera", "THE GATEKEEPER": "gatekeeper"}


# --- steps --------------------------------------------------------------------------------------------------------------
def _ff(src, dst, *args):
    subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-i", str(src), *args], check=True)
    return dst


def _read(p):
    sr, a = wavfile.read(str(p))
    a = a.astype(np.float32) / (2 ** 31 if a.dtype == np.int32 else 2 ** 15 if a.dtype == np.int16 else 1.0)
    if a.ndim > 1:
        a = a.mean(axis=1)
    if sr != SR:
        a = signal.resample_poly(a, SR, sr).astype(np.float32)
    return a


def _write(p, a):
    wavfile.write(str(p), SR, np.clip(a, -1, 1).astype(np.float32))


def step_body(src, dst, preset, sets):
    b = studio.body_bin(); cmd = [str(b), "render", "--body", preset, "--in", str(src), "--out", str(dst), "--receipt", str(dst) + ".receipt.json"]
    for k, v in sets.items():
        cmd += ["--set", f"{k}={v}"]
    subprocess.run(cmd, check=True, capture_output=True)
    return dst


def step_shift(src, dst, formant, pitch, rng):
    from parselmouth.praat import call
    s = analysis.load(src)
    y = call(s, "Change gender", 60, 600, float(formant), float(pitch or 0), float(rng), 1.0)
    y.save(str(dst), "WAV"); return dst


def step_codec(src, dst, kind, tmp):
    mid = pathlib.Path(tmp) / f"codec_{kind}_{time.time_ns()}"
    if kind == "opus8":
        _ff(src, None, "-ac", "1", "-ar", "8000", "-c:a", "libopus", "-b:a", "8k", "-application", "voip", str(mid) + ".ogg"); mid = str(mid) + ".ogg"
    elif kind == "mulaw":
        _ff(src, None, "-ac", "1", "-ar", "8000", "-c:a", "pcm_mulaw", str(mid) + ".wav"); mid = str(mid) + ".wav"
    elif kind == "adpcm16":
        _ff(src, None, "-ac", "1", "-ar", "16000", "-c:a", "adpcm_ima_wav", str(mid) + ".wav"); mid = str(mid) + ".wav"
    _ff(mid, dst, "-ac", "1", "-ar", str(SR), "-c:a", "pcm_f32le", str(dst)); return dst


def step_air(src, dst, spec, seed):
    a = _read(src); n = len(a); rng = np.random.default_rng(seed); out = a.copy()
    rms = active_rms(a)  # noise sits relative to the speech, whatever the silences around it
    if "hiss" in spec:
        h = rng.standard_normal(n).astype(np.float32); b, c = signal.butter(2, [300 / (SR / 2), 3400 / (SR / 2)], "band"); h = signal.lfilter(b, c, h)
        out += h / (np.sqrt(np.mean(h ** 2)) + 1e-9) * rms * 10 ** (spec["hiss"] / 20) * 4
    if "hum" in spec:
        t = np.arange(n) / SR; m = sum(np.sin(2 * np.pi * 60 * k * t) / k for k in (1, 2, 3, 5))
        out += m / np.sqrt(np.mean(m ** 2)) * rms * 10 ** (spec["hum"] / 20) * 4
    if "crackle" in spec:
        imp = np.zeros(n, np.float32); idx = rng.choice(n, size=max(1, int(n / SR * 9)), replace=False)
        imp[idx] = rng.choice([-1, 1], size=len(idx)) * rng.uniform(0.3, 1.0, len(idx))
        b, c = signal.butter(2, 1800 / (SR / 2), "high"); imp = signal.lfilter(b, c, imp)
        out += imp * rms * 10 ** (spec["crackle"] / 20) * 40
    _write(dst, out); return dst


def _key_sound(kind, rng):
    if kind == "blip":
        t = np.arange(int(0.05 * SR)) / SR; s = np.sin(2 * np.pi * 1760 * t) * np.hanning(len(t)) * 0.25
        return s.astype(np.float32), (s * 0.8).astype(np.float32)
    if kind == "click":
        c = np.zeros(int(0.03 * SR), np.float32); c[:24] = rng.uniform(-0.6, 0.6, 24)
        n = rng.standard_normal(int(0.09 * SR)).astype(np.float32) * np.linspace(0.15, 0, int(0.09 * SR)); return c, np.concatenate([c, n])
    burst = rng.standard_normal(int(0.16 * SR)).astype(np.float32); b, a = signal.butter(2, [900 / (SR / 2), 3500 / (SR / 2)], "band")
    burst = signal.lfilter(b, a, burst) * np.linspace(0.35, 0, len(burst)) ** 1.5
    return burst.astype(np.float32), burst[::-1].copy() * 0.7


def step_key(src, dst, kind, seed):
    a = _read(src); pre, post = _key_sound(kind, np.random.default_rng(seed))
    if kind != "blip":  # the key's click and hiss come down the same narrow line as the voice
        b, c = signal.butter(3, [300 / (SR / 2), 3400 / (SR / 2)], "band"); pre, post = signal.lfilter(b, c, pre).astype(np.float32), signal.lfilter(b, c, post).astype(np.float32)
    gap = np.zeros(int(0.06 * SR), np.float32)
    _write(dst, np.concatenate([pre, gap, a, gap, post])); return dst


def active_rms(a, floor_db=-35.0):
    """RMS over the 20 ms frames within floor_db of the loudest: the level of the speech, not of the silences."""
    n = int(0.02 * SR); f = a[: len(a) // n * n].reshape(-1, n); r = np.sqrt(np.mean(f ** 2, axis=1)) + 1e-12
    keep = r > r.max() * 10 ** (floor_db / 20)
    return float(np.sqrt(np.mean(r[keep] ** 2))) if keep.any() else float(np.sqrt(np.mean(a ** 2)) + 1e-9)


def normalise(a, target_dbfs=-20.0, peak=-1.0):
    rms = active_rms(a); a = a * 10 ** ((target_dbfs - 20 * np.log10(rms)) / 20)
    pk = float(np.max(np.abs(a)) + 1e-9); lim = 10 ** (peak / 20)
    return a * (lim / pk) if pk > lim else a


# --- run ----------------------------------------------------------------------------------------------------------------
def render(take, chain, compare=False):
    steps = CHAINS[chain]; out = studio.render_dir(take, f"fx-{chain}"); src = ops.dry(take); t0 = time.time()
    receipt = None
    with tempfile.TemporaryDirectory() as tmp:
        s = analysis.load(src); seg = analysis.segments(s)["segments"]   # the voice only, with a little air either side
        voice = [g for g in seg if g["voiced_pct"] >= 20 and g.get("f0_median", 999) < 450]   # not clicks, chair squeaks, UI blips
        seg = voice or seg
        a0, a1 = (max(0.0, seg[0]["start"] - 0.2), min(s.duration, seg[-1]["end"] + 0.25)) if seg else (0.0, s.duration)
        cur = pathlib.Path(tmp) / "trim.wav"; _ff(src, cur, "-ss", f"{a0:.3f}", "-to", f"{a1:.3f}", "-ac", "1", "-c:a", "pcm_f32le", str(cur))
        for k, st in enumerate(steps):
            nxt = pathlib.Path(tmp) / f"{k:02d}_{st[0]}.wav"
            if st[0] == "body":
                step_body(cur, nxt, st[1], st[2]); receipt = studio.read_json(str(nxt) + ".receipt.json")
            elif st[0] == "shift":
                step_shift(cur, nxt, *st[1:])
            elif st[0] == "ff":
                _ff(cur, nxt, "-af", st[1], "-ac", "1", "-ar", str(SR), "-c:a", "pcm_f32le", str(nxt))
            elif st[0] == "codec":
                step_codec(cur, nxt, st[1], tmp)
            elif st[0] == "air":
                step_air(cur, nxt, st[1], seed=zlib.crc32(f"{take}/{chain}".encode()))
            elif st[0] == "key":
                step_key(cur, nxt, st[1], seed=zlib.crc32(f"{take}/{chain}/key".encode()))
            cur = nxt
        a = normalise(_read(cur)); _write(pathlib.Path(tmp) / "final.wav", a)
        _ff(pathlib.Path(tmp) / "final.wav", None, "-c:a", "pcm_s24le", str(out / "wet.wav"))
    if receipt:
        studio.write_json(out / "receipt.json", receipt)
    studio.write_json(out / "body.json", dict(body=f"fx-{chain}", non_physics=True, chain=[list(s) for s in steps],
                                              note="Ariadne Body first (physics, receipt.json), then non-physics shaping",
                                              rendered=time.strftime("%Y-%m-%d %H:%M:%S"), wall_s=round(time.time() - t0, 2)))
    if compare:
        ops.compare(take, f"fx-{chain}")
    return out / "wet.wav"


def scene(session, out_take=None, gaps=None, chains=True):
    """Lay a session's circled takes end to end in scene order, each through its character's chain (or dry)."""
    S = studio.read_json(studio.STUDIO / "sessions" / f"{session}.json") or ops.prompts()
    order = S["prompts"] if S.get("session") == session else []  # the session file is already in scene order
    parts, log = [], []
    for p in order:
        take = (S.get("selects") or {}).get(p["id"])
        if not take:
            continue
        chain = WHO.get(p.get("who", ""), "vera")
        wav = render(take, chain) if chains else ops.dry(take)
        a = _read(wav) if chains else normalise(_read(wav))
        gap = (gaps or {}).get(p["id"], 0.45)
        parts += [np.zeros(int(gap * SR), np.float32), a]; log.append(dict(line=p["id"], who=p.get("who"), take=take, chain=chain if chains else "dry"))
    a = np.concatenate(parts + [np.zeros(int(0.8 * SR), np.float32)])
    name = out_take or f"{session}-scene-{'fx' if chains else 'dry'}"
    d = studio.take_dir(name)
    with tempfile.TemporaryDirectory() as tmp:
        _write(pathlib.Path(tmp) / "s.wav", a); _ff(pathlib.Path(tmp) / "s.wav", None, "-c:a", "pcm_s24le", str(d / "dry.wav"))
    studio.write_json(d / "take.json", dict(take=name, device="assembled", recorded=time.strftime("%Y-%m-%d %H:%M:%S"),
                                             notes=f"{session} scene, {'character chains' if chains else 'dry takes'}: " + ", ".join(f"{x['line']} {x['who']}" for x in log),
                                             sections={}, scene=log))
    studio.log("scene", session=session, take=name, lines=len(log))
    return dict(take=name, seconds=round(len(a) / SR, 1), lines=log)


# --- babel: the law's voices piling up; unison: one voice becoming many -------------------------------------------------
BABEL_BODIES = ["coke", "bottle", "beehive", "cathedral", "radio", "neutral", "sitar", "guard_box", "gatekeeper", "v"]


def _voice_chain(rng):
    """A random speaker: an Ariadne body (with its receipts), then a different size and pitch, then a little band."""
    body = BABEL_BODIES[rng.integers(len(BABEL_BODIES))]
    formant = float(rng.uniform(0.84, 1.2)); st = float(rng.uniform(-6, 5)); rng_f = float(rng.uniform(0.7, 1.3))
    lo, hi = int(rng.uniform(80, 400)), int(rng.uniform(2800, 9000))
    return [("body", body, {}), ("shift_rel", formant, st, rng_f), ("ff", f"highpass=f={lo},lowpass=f={hi}")], dict(body=body, formant=round(formant, 2), semitones=round(st, 1), lo=lo, hi=hi)


def _run_chain(src, steps, tmp, tag):
    cur = pathlib.Path(src)
    for k, st in enumerate(steps):
        nxt = pathlib.Path(tmp) / f"{tag}_{k:02d}.wav"
        if st[0] == "body":
            step_body(cur, nxt, st[1], st[2])
        elif st[0] == "shift_rel":   # pitch relative to the take's own median
            from parselmouth.praat import call
            s = analysis.load(cur); _, f = analysis.pitch(s); v = f[f > 0]
            med = float(np.median(v)) if len(v) > 5 else 0.0
            y = call(s, "Change gender", 60, 600, float(st[1]), med * 2 ** (st[2] / 12) if med else 0.0, float(st[3]), 1.0); y.save(str(nxt), "WAV")
        elif st[0] == "ff":
            _ff(cur, nxt, "-af", st[1], "-ac", "1", "-ar", str(SR), "-c:a", "pcm_f32le", str(nxt))
        cur = nxt
    return cur


def _trimmed(take, tmp):
    src = ops.dry(take); s = analysis.load(src); seg = [g for g in analysis.segments(s)["segments"] if g["voiced_pct"] >= 20] or analysis.segments(s)["segments"]
    a0, a1 = (max(0.0, seg[0]["start"] - 0.05), min(s.duration, seg[-1]["end"] + 0.1)) if seg else (0.0, s.duration)
    out = pathlib.Path(tmp) / f"trim_{take}.wav"; _ff(src, out, "-ss", f"{a0:.3f}", "-to", f"{a1:.3f}", "-ac", "1", "-c:a", "pcm_f32le", str(out)); return out


def babel(takes, name="babel", seconds=60.0, first_gap=3.0, last_gap=0.18, seed=1637):
    """Lay many takes over each other, each as a different speaker, entering faster and faster (gaps shrink
    geometrically from first_gap to last_gap), panned across the field, louder and brighter as it goes; ends on a
    hard cut. Writes takes/<name>/dry.wav (stereo) and a placement log."""
    rng = np.random.default_rng(seed); n = int(seconds * SR); mix = np.zeros((n, 2), np.float32); log = []
    with tempfile.TemporaryDirectory() as tmp:
        t, k = 0.0, 0
        while t < seconds - 0.5:
            take = takes[k % len(takes)]; k += 1
            steps, desc = _voice_chain(rng)
            a = normalise(_read(_run_chain(_trimmed(take, tmp), steps, tmp, f"v{k}")), target_dbfs=-24)
            x = t / seconds; gain = 10 ** ((-10 + 9 * x ** 1.5 + rng.uniform(-2, 2)) / 20); pan = float(rng.uniform(-0.9, 0.9))
            i0 = int(t * SR); seg = a[: max(0, min(len(a), n - i0))] * gain
            mix[i0:i0 + len(seg), 0] += seg * np.sqrt((1 - pan) / 2); mix[i0:i0 + len(seg), 1] += seg * np.sqrt((1 + pan) / 2)
            log.append(dict(at=round(t, 2), take=take, pan=round(pan, 2), **desc))
            t += first_gap * (last_gap / first_gap) ** x * float(rng.uniform(0.7, 1.3))
        # the pressure: a slow brightening tilt, then a limiter
        b, c = signal.butter(1, 2500 / (SR / 2), "high"); hp = signal.lfilter(b, c, mix, axis=0)
        mix = mix + hp * np.linspace(0, 1.2, n)[:, None] ** 2
        mix = np.tanh(mix * 1.4) / 1.4
        mix = mix * (10 ** (-1 / 20) / (np.max(np.abs(mix)) + 1e-9))
        d = studio.take_dir(name); p = pathlib.Path(tmp) / "m.wav"; wavfile.write(str(p), SR, mix.astype(np.float32))
        _ff(p, None, "-c:a", "pcm_s24le", str(d / "dry.wav"))
    studio.write_json(d / "take.json", dict(take=name, device="assembled (babel)", recorded=time.strftime("%Y-%m-%d %H:%M:%S"),
                                             notes=f"babel of {len(log)} entries from {len(set(takes))} takes", sections={}, babel=log))
    studio.log("babel", take=name, entries=len(log), seconds=seconds)
    return dict(take=name, entries=len(log), voices=len(set(takes)))


def unison(take, name=None, copies=9, seed=7):
    """One line becoming a body: the take plus copies of itself, each a hair different in pitch, size and time,
    the whole choir in Ariadne's cathedral. Writes takes/<name>/dry.wav (stereo)."""
    rng = np.random.default_rng(seed); name = name or f"{take}-unison"
    with tempfile.TemporaryDirectory() as tmp:
        base = _trimmed(take, tmp); voices = []
        for k in range(copies):
            if k == 0:
                v = _read(base)
            else:
                v = _read(_run_chain(base, [("shift_rel", float(rng.uniform(0.97, 1.04)), float(rng.uniform(-0.25, 0.25)), 1.0)], tmp, f"u{k}"))
            dly = np.zeros(int(rng.uniform(0, 0.035) * SR), np.float32); voices.append(np.concatenate([dly, v]))
        L = max(len(v) for v in voices); st = np.zeros((L, 2), np.float32)
        for k, v in enumerate(voices):
            pan = 0.0 if k == 0 else float(rng.uniform(-1, 1)); g = 1.0 if k == 0 else 0.55
            st[: len(v), 0] += v * g * np.sqrt((1 - pan) / 2); st[: len(v), 1] += v * g * np.sqrt((1 + pan) / 2)
        mono = pathlib.Path(tmp) / "uni.wav"; _write(mono, st.mean(axis=1))
        hall = pathlib.Path(tmp) / "hall.wav"; step_body(mono, hall, "cathedral", {})
        h = _read(hall); n = max(len(h), L); out = np.zeros((n, 2), np.float32)
        out[: L] += st * 0.7; out[: len(h)] += h[:, None] * 0.6
        out = out * (10 ** (-1 / 20) / (np.max(np.abs(out)) + 1e-9))
        d = studio.take_dir(name); p = pathlib.Path(tmp) / "o.wav"; wavfile.write(str(p), SR, out.astype(np.float32))
        _ff(p, None, "-c:a", "pcm_s24le", str(d / "dry.wav"))
    studio.write_json(d / "take.json", dict(take=name, device="assembled (unison)", recorded=time.strftime("%Y-%m-%d %H:%M:%S"),
                                             notes=f"{take} as a choir of {copies}, in the cathedral", sections={}))
    studio.log("unison", take=name, source=take, copies=copies)
    return dict(take=name, copies=copies)
