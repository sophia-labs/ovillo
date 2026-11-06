"""What a take is, and what a Body did to it. Every number here is measured from the audio.

The methods:
- **Pitch:** Praat autocorrelation. Two passes for speech, after Hirst: the second pass's range is fitted to the first pass's quartiles.
- **Formants:** Burg. On a sustained vowel the ceiling is the one giving the steadiest track (after Escudero et al. 2009). In connected speech it's the ceiling where the tract-length estimate plateaus.
- **Apparent vocal-tract length:** from formant spacing, assuming a uniform tube closed at the glottis. Fi = (i - 1/2) dF, so L = c / (2 dF), with c = 350 m/s. This is the main *size* cue a listener hears, so it's the number to watch when a Body is meant to feminize.
- **Voice quality:** HNR, local jitter and shimmer, and uncorrected H1-H2, on a held /a/. CPPS on speech.
- **Spectral balance:**
  - alpha ratio: 1-5 kHz against 50 Hz-1 kHz, voiced frames only
  - LTAS
  - centroid above 1 kHz (sibilance)
- **compare():** treats the Body as a system from the dry take to the wet one.
  - A Welch cross-spectral transfer estimate |H(f)| (the resonances and notches the Body imposes) and its coherence. Coherence falls where the Body isn't linear: rattles, contacts, active material.
  - Every metric above, dry against wet, after a cross-correlation delay alignment.
"""
import math, pathlib
import numpy as np
import parselmouth
from parselmouth.praat import call
from scipy import signal

C = 35000.0  # cm/s
HILLENBRAND = {  # Hillenbrand et al. (1995) adult means, F1-F3 Hz: context, not targets
    "men": {"ah": (768, 1333, 2522), "ee": (342, 2322, 3000), "oo": (378, 997, 2343)},
    "women": {"ah": (936, 1551, 2815), "ee": (437, 2761, 3372), "oo": (459, 1105, 2735)},
}
SECTION_KEYS = ["room", "ah", "ee", "oo", "uh", "ah_low", "ah_mid", "ah_high", "sweep", "glide", "ss", "sh", "mm",
                "hvd", "mama", "passage", "passage_v", "v_lines", "chat", "man_low", "man_weird"]


# ---------- loading

def load(path):
    s = parselmouth.Sound(str(path))
    return s.convert_to_mono() if s.n_channels > 1 else s


def part(s, a, b):
    return s.extract_part(from_time=max(0, a), to_time=min(s.duration, b), preserve_times=False)


def rms_dbfs(x):
    v = x.values[0]
    return round(float(20 * np.log10(np.sqrt(np.mean(v ** 2)) + 1e-12)), 1)


# ---------- pitch

def pitch(s, floor=50, ceil=800, two_pass=True):
    p = s.to_pitch_ac(time_step=0.01, pitch_floor=floor, pitch_ceiling=ceil)
    f = p.selected_array["frequency"]; v = f[f > 0]
    if two_pass and len(v) > 20:
        q1, q3 = np.percentile(v, [25, 75])
        p = s.to_pitch_ac(time_step=0.01, pitch_floor=max(40, 0.75 * q1), pitch_ceiling=min(1100, 1.5 * q3))
    return p.xs(), p.selected_array["frequency"]


def f0_stats(f):
    f = f[f > 0]
    if len(f) < 5:
        return None
    med = float(np.median(f)); st = 12 * np.log2(f / med)
    p5, p25, p75, p95 = np.percentile(f, [5, 25, 75, 95])
    return dict(median_hz=round(med, 1), p5_hz=round(float(p5), 1), p25_hz=round(float(p25), 1), p75_hz=round(float(p75), 1),
                p95_hz=round(float(p95), 1), sd_st=round(float(st.std()), 2), range_p5_p95_st=round(float(12 * np.log2(p95 / p5)), 1),
                voiced_s=round(len(f) * 0.01, 2))


def voiced_at(t, f):
    """A function time -> voiced? from a pitch track."""
    def g(tt):
        i = np.clip(np.round((np.asarray(tt) - t[0]) / 0.01).astype(int), 0, len(f) - 1)
        return f[i] > 0
    return g


# ---------- segmentation

def segments(s, min_gap=0.45, min_len=0.25):
    it = s.to_intensity(minimum_pitch=75, time_step=0.01)
    db = it.values[0]; t = it.xs()
    floor = float(np.percentile(db, 10)); thr = max(floor + 12, float(db.max()) - 35)
    on = db > thr; segs = []; i = 0
    while i < len(on):
        if on[i]:
            j = i
            while j < len(on) and on[j]:
                j += 1
            segs.append([float(t[i]), float(t[j - 1])]); i = j
        else:
            i += 1
    merged = []
    for a, b in segs:
        if merged and a - merged[-1][1] < min_gap:
            merged[-1][1] = b
        else:
            merged.append([a, b])
    out = []
    for a, b in merged:
        if b - a < min_len:
            continue
        x = part(s, a - 0.05, b + 0.05); _, f = pitch(x, two_pass=False); v = f[f > 0]
        e = dict(start=round(a - 0.05, 2), end=round(b + 0.05, 2), seconds=round(b - a, 2), voiced_pct=round(100 * len(v) / max(1, len(f))))
        if len(v) > 5:
            n = max(1, len(v) // 3)
            e.update(f0_median=round(float(np.median(v))), f0_start=round(float(np.median(v[:n]))), f0_end=round(float(np.median(v[-n:]))),
                     f0_min=round(float(v.min())), f0_max=round(float(v.max())))
        out.append(e)
    return dict(noise_floor_db=round(floor, 1), gate_db=round(thr, 1), segments=out)


# ---------- formants and tract length

def formant_matrix(x, ceil, n=4):
    fm = x.to_formant_burg(time_step=0.01, max_number_of_formants=5, maximum_formant=ceil, window_length=0.025)
    t = np.array(fm.xs())
    F = np.stack([np.array(call(fm, "To Matrix", i).values[0]) for i in range(1, n + 1)])
    F[F <= 0] = np.nan
    return t, F


def delta_f(F):
    F = np.asarray([f for f in F if np.isfinite(f)]); k = np.arange(1, len(F) + 1) - 0.5
    return float(np.sum(F * k) / np.sum(k * k))


def formants_steady(x):
    d = x.duration; core = part(x, 0.2 * d, 0.8 * d); best = None
    for ceil in range(4500, 6751, 250):
        _, F = formant_matrix(core, ceil)
        tr = [v[np.isfinite(v)] for v in F]
        if any(len(v) < 5 for v in tr[:3]):
            continue
        cost = sum(float(np.std(np.log(v))) for v in tr[:3])
        if best is None or cost < best[0]:
            best = (cost, ceil, [float(np.median(v)) if len(v) else float("nan") for v in tr])
    if best is None:
        return None
    return dict(ceiling_hz=best[1], F=[round(m) for m in best[2]], steadiness=round(best[0], 3))


def connected(x, label=None):
    """Running-speech measurements: pitch, formant medians on voiced frames, tract length, CPPS, balance."""
    t, f = pitch(x); vo = voiced_at(t, f)
    sweep = {}
    for ceil in range(4500, 6501, 250):
        tf, F = formant_matrix(x, ceil); m = vo(tf)
        meds = [float(np.nanmedian(row[m])) if m.any() else float("nan") for row in F]
        sweep[ceil] = (meds, C / (2 * delta_f(meds)) if all(np.isfinite(meds)) else float("nan"))
    cs = sorted(sweep); Ls = [sweep[c][1] for c in cs]
    var = np.array([abs(Ls[i] - Ls[i - 1]) + abs(Ls[i + 1] - Ls[i]) for i in range(1, len(cs) - 1)])
    if np.all(np.isnan(var)) or f0_stats(f) is None:  # too little voicing to measure resonance on
        return dict(f0=f0_stats(f), formant_medians_hz=None, formant_ceiling_hz=None, apparent_vtl_cm=None, vtl_by_ceiling={},
                    cpps_db=None, alpha_ratio_db=None, centroid_hz=None, level_dbfs=rms_dbfs(x))
    best = cs[1 + int(np.nanargmin(var))]
    fr, P = ltas(x, vo)
    return dict(f0=f0_stats(f), formant_medians_hz=[round(m) if np.isfinite(m) else None for m in sweep[best][0]], formant_ceiling_hz=best,
                apparent_vtl_cm=round(sweep[best][1], 2), vtl_by_ceiling={c: round(sweep[c][1], 2) for c in cs},
                cpps_db=cpps(x), alpha_ratio_db=alpha_ratio(fr, P), centroid_hz=centroid(fr, P), level_dbfs=rms_dbfs(x))


# ---------- voice quality and spectra

def h1_h2(x, f0):
    a = x.values[0]; sr = x.sampling_frequency; n = int(0.04 * sr); out = []
    for i in range(0, len(a) - n, n // 2):
        spec = 20 * np.log10(np.abs(np.fft.rfft(a[i:i + n] * np.hanning(n), 8 * n)) + 1e-12); fr = np.fft.rfftfreq(8 * n, 1 / sr)
        pk = lambda fc: spec[(fr > fc * 0.85) & (fr < fc * 1.15)].max()
        out.append(pk(f0) - pk(2 * f0))
    return round(float(np.median(out)), 1) if out else None


def voice_quality(x, f0):
    floor, ceil = 0.6 * f0, 1.6 * f0
    hv = x.to_harmonicity_cc(time_step=0.01, minimum_pitch=floor, silence_threshold=0.1, periods_per_window=1.0).values[0]
    hv = hv[hv > -100]
    pp = call(x, "To PointProcess (periodic, cc)", floor, ceil)
    return dict(hnr_db=round(float(np.mean(hv)), 1), jitter_local_pct=round(100 * call(pp, "Get jitter (local)", 0, 0, 0.0001, 0.02, 1.3), 2),
                shimmer_local_pct=round(100 * call([x, pp], "Get shimmer (local)", 0, 0, 0.0001, 0.02, 1.3, 1.6), 2), h1_h2_db=h1_h2(x, f0))


def cpps(x):
    try:
        # A straight least-squares trend line (Hillenbrand's CPPS): Praat's robust fit is ~50x slower for the same reading.
        pc = call(x, "To PowerCepstrogram", 60, 0.005, 5000, 50)
        return round(call(pc, "Get CPPS", "no", 0.02, 0.0005, 60, 330, 0.05, "Parabolic", 0.001, 0.0, "Straight", "Least squares"), 2)
    except Exception:
        return None


def ltas(x, voiced=None, n=4096):
    a = x.values[0]; sr = x.sampling_frequency; hop = n // 2
    frames = [(i, a[i:i + n]) for i in range(0, len(a) - n, hop)]
    if voiced is not None:
        keep = voiced(np.array([(i + n / 2) / sr for i, _ in frames])) if frames else []
        frames = [fr for fr, k in zip(frames, keep) if k]
    if not frames:
        return None, None
    w = np.hanning(n)
    P = np.mean([np.abs(np.fft.rfft(fr * w)) ** 2 for _, fr in frames], axis=0)
    return np.fft.rfftfreq(n, 1 / sr), P


def alpha_ratio(fr, P):
    if fr is None:
        return None
    lo = P[(fr >= 50) & (fr < 1000)].sum(); hi = P[(fr >= 1000) & (fr < 5000)].sum()
    return round(float(10 * np.log10(hi / lo)), 1)


def centroid(fr, P, lo=1000, hi=16000):
    if fr is None:
        return None
    m = (fr >= lo) & (fr <= hi)
    return round(float(np.sum(fr[m] * P[m]) / np.sum(P[m])))


def smooth_octave(fr, db, frac=6):
    out = np.empty_like(db)
    for i, f in enumerate(fr):
        if f <= 0:
            out[i] = db[i]; continue
        m = (fr >= f * 2 ** (-0.5 / frac)) & (fr <= f * 2 ** (0.5 / frac))
        out[i] = 10 * np.log10(np.mean(10 ** (db[m] / 10)))
    return out


# ---------- a take

def analyse_take(wav, sections=None):
    s = load(wav); R = dict(file=str(wav), seconds=round(s.duration, 2))
    sec = {k: part(s, *v) for k, v in (sections or {}).items()}
    if "room" in sec:
        R["room_dbfs"] = rms_dbfs(sec["room"])
    # speech: every named speech section, or the whole take if none are named
    speech = [k for k in ("passage", "passage_v", "v_lines", "chat", "man_low", "man_weird") if k in sec]
    R["speech"] = {k: connected(sec[k]) for k in speech} if speech else {"whole": connected(s)}
    vow = {k: formants_steady(sec[k]) for k in ("ah", "ee", "oo", "uh") if k in sec}
    vow = {k: v for k, v in vow.items() if v}
    for k in vow:
        st = f0_stats(pitch(sec[k])[1]); vow[k]["f0_hz"] = st["median_hz"] if st else None
    if vow:
        R["vowels"] = vow
        R["vowel_vtl_cm"] = round(C / (2 * float(np.mean([delta_f(v["F"]) for v in vow.values()]))), 2)
    for k in ("ah", "ah_low", "ah_mid", "ah_high"):
        if k in sec:
            st = f0_stats(pitch(sec[k])[1])
            if st:
                R.setdefault("voice_quality", {})[k] = dict(f0_hz=st["median_hz"], **voice_quality(sec[k], st["median_hz"]))
    if "glide" in sec:
        _, f = pitch(sec["glide"], floor=45, ceil=1100, two_pass=False); v = f[f > 0]
        if len(v) > 10:
            lo, hi = np.percentile(v, [1, 99])
            R["range"] = dict(low_hz=round(float(lo), 1), high_hz=round(float(hi), 1), semitones=round(float(12 * np.log2(hi / lo)), 1))
    for k in ("ss", "sh", "mm"):
        if k in sec:
            fr, P = ltas(sec[k])
            R.setdefault("held", {})[k] = dict(centroid_hz=centroid(fr, P, 500 if k == "mm" else 1000),
                                               peak_hz=round(float(fr[np.argmax(P * (fr > 100))])) if fr is not None else None)
    main = R["speech"].get("passage") or next(iter(R["speech"].values()))
    R["summary"] = dict(f0_median_hz=(main["f0"] or {}).get("median_hz"), apparent_vtl_cm=main["apparent_vtl_cm"],
                        alpha_ratio_db=main["alpha_ratio_db"], cpps_db=main["cpps_db"], level_dbfs=main["level_dbfs"])
    return R


def plot_take(wav, R, sections, out):
    import matplotlib; matplotlib.use("Agg"); import matplotlib.pyplot as plt
    s = load(wav); a = s.values[0]; sr = s.sampling_frequency
    fig = plt.figure(figsize=(14, 9), dpi=90); gs = fig.add_gridspec(3, 2, height_ratios=[1.3, 1, 1.2])
    ax = fig.add_subplot(gs[0, :])
    f, t, S = signal.spectrogram(a, sr, nperseg=1024, noverlap=768); m = f <= 8000
    ax.pcolormesh(t, f[m], 10 * np.log10(S[m] + 1e-14), shading="auto", cmap="magma", vmin=np.percentile(10 * np.log10(S[m] + 1e-14), 40))
    tp, fp = pitch(s, floor=45, ceil=1100, two_pass=False); ax.plot(tp, np.where(fp > 0, fp * 10, np.nan), ".", ms=1.5, color="#7fffd4", label="F0 x10")
    for k, (x0, x1) in (sections or {}).items():
        ax.axvspan(x0, x1, color="w", alpha=0.06); ax.text(x0, 7600, k, color="w", fontsize=7)
    ax.set_ylabel("Hz"); ax.set_title(f"{pathlib.Path(wav).parent.name}: spectrogram, F0 x10 in aqua"); ax.legend(loc="upper right", fontsize=7)
    ax2 = fig.add_subplot(gs[1, :]); ax2.semilogy(tp, np.where(fp > 0, fp, np.nan), ".", ms=2, color="#c03070")
    ax2.axhspan(165, 255, alpha=0.07, color="#c03070"); ax2.axhspan(85, 155, alpha=0.07, color="#3060c0"); ax2.set_ylim(50, 1000)
    ax2.set_ylabel("F0 Hz"); ax2.set_xlim(0, s.duration); ax2.set_title("F0 (bands: typical speaking F0 ranges, women pink, men blue; wide overlap)", fontsize=9)
    ax3 = fig.add_subplot(gs[2, 0])
    for who, col in (("men", "#3060c0"), ("women", "#c03070")):
        p = HILLENBRAND[who]; seq = ("ee", "ah", "oo", "ee")
        ax3.plot([p[v][1] for v in seq], [p[v][0] for v in seq], "--", color=col, lw=1, label=f"{who}, Hillenbrand 1995")
    if R.get("vowels"):
        V = R["vowels"]; ks = [k for k in ("ee", "ah", "oo") if k in V]
        ax3.plot([V[k]["F"][1] for k in ks + ks[:1]], [V[k]["F"][0] for k in ks + ks[:1]], "o-", color="k", label="this take")
        for k in V:
            ax3.annotate(k, (V[k]["F"][1], V[k]["F"][0]), xytext=(4, 4), textcoords="offset points")
    ax3.invert_xaxis(); ax3.invert_yaxis(); ax3.set_xlabel("F2 Hz"); ax3.set_ylabel("F1 Hz"); ax3.legend(fontsize=7); ax3.set_title("vowels", fontsize=9)
    ax4 = fig.add_subplot(gs[2, 1])
    for k, (x0, x1) in ((sections or {}) or {"whole": (0, s.duration)}).items():
        if k in R["speech"] or k == "whole":
            x = part(s, x0, x1); tt, ff = pitch(x); fr, P = ltas(x, voiced_at(tt, ff))
            if fr is not None:
                db = 10 * np.log10(P + 1e-20); db = smooth_octave(fr, db - db.max())
                ax4.semilogx(fr[1:], db[1:], lw=1, label=k)
    ax4.set_xlim(60, 16000); ax4.set_ylim(-70, 3); ax4.set_xlabel("Hz"); ax4.set_ylabel("dB"); ax4.legend(fontsize=7); ax4.set_title("LTAS, voiced frames, 1/6 octave", fontsize=9)
    fig.tight_layout(); fig.savefig(out); plt.close(fig)


# ---------- dry against wet

def align(dry, wet, sr, max_ms=80):
    n = int(sr * max_ms / 1000); seg = slice(0, min(len(dry), len(wet), int(sr * 20)))
    c = signal.correlate(wet[seg], dry[seg], mode="full", method="fft"); mid = len(dry[seg]) - 1
    lag = int(np.argmax(np.abs(c[mid:mid + n + 1])))
    return lag


def compare(dry_wav, wet_wav, out_png=None, title=""):
    D = load(dry_wav); W = load(wet_wav); sr = D.sampling_frequency
    d = D.values[0]; w = W.values[0]
    lag = align(d, w, sr); w = w[lag:]; n = min(len(d), len(w)); d, w = d[:n], w[:n]
    nper = 4096
    fr, Pxy = signal.csd(d, w, sr, nperseg=nper); _, Pxx = signal.welch(d, sr, nperseg=nper)
    H = 20 * np.log10(np.abs(Pxy) / (Pxx + 1e-20) + 1e-12)
    _, coh = signal.coherence(d, w, sr, nperseg=nper)
    Dm = parselmouth.Sound(d, sr); Wm = parselmouth.Sound(w, sr)
    cd, cw = connected(Dm), connected(Wm)
    keys = ("apparent_vtl_cm", "alpha_ratio_db", "centroid_hz", "cpps_db", "level_dbfs")
    delta = {k: (round(cw[k] - cd[k], 2) if cd.get(k) is not None and cw.get(k) is not None else None) for k in keys}
    f0d, f0w = (cd["f0"] or {}).get("median_hz"), (cw["f0"] or {}).get("median_hz")
    m = (fr > 80) & (fr < 8000)
    # the Body's strongest peaks and notches between 80 Hz and 8 kHz, from the smoothed transfer estimate
    Hs = smooth_octave(fr, H, 12); pk = signal.find_peaks(Hs[m], prominence=3)[0]; nt = signal.find_peaks(-Hs[m], prominence=3)[0]
    fm = fr[m]
    res = dict(delay_ms=round(1000 * lag / sr, 2), dry=cd, wet=cw, delta=delta,
               f0_ratio=round(f0w / f0d, 3) if f0d and f0w else None,
               transfer=dict(peaks_hz=[round(float(fm[i])) for i in pk][:12], notches_hz=[round(float(fm[i])) for i in nt][:12],
                             gain_db_at={str(f): round(float(np.interp(f, fr, Hs)), 1) for f in (125, 250, 500, 1000, 2000, 3000, 4000, 6000, 8000)},
                             mean_coherence_80_8k=round(float(np.mean(coh[m])), 3)))
    res["summary"] = dict(vtl_cm=[cd["apparent_vtl_cm"], cw["apparent_vtl_cm"]], alpha_db=[cd["alpha_ratio_db"], cw["alpha_ratio_db"]],
                          f0_hz=[f0d, f0w], coherence=res["transfer"]["mean_coherence_80_8k"], delay_ms=res["delay_ms"])
    if out_png:
        plot_compare(d, w, sr, fr, H, Hs, coh, cd, cw, res, out_png, title)
    return res


def plot_compare(d, w, sr, fr, H, Hs, coh, cd, cw, res, out, title):
    import matplotlib; matplotlib.use("Agg"); import matplotlib.pyplot as plt
    fig = plt.figure(figsize=(14, 10), dpi=90); gs = fig.add_gridspec(3, 2, height_ratios=[1, 1, 1.1])
    for row, (x, name) in enumerate(((d, "dry"), (w, "wet"))):
        ax = fig.add_subplot(gs[row, 0]); f, t, S = signal.spectrogram(x, sr, nperseg=1024, noverlap=768); m = f <= 8000
        L = 10 * np.log10(S[m] + 1e-14); ax.pcolormesh(t, f[m], L, shading="auto", cmap="magma", vmin=np.percentile(L, 40))
        ax.set_title(f"{name}  VTL {(cd if row == 0 else cw)['apparent_vtl_cm']} cm", fontsize=9); ax.set_ylabel("Hz")
    ax = fig.add_subplot(gs[0, 1]); m = (fr > 50) & (fr < 16000)
    ax.semilogx(fr[m], H[m], color="#bbb", lw=0.6); ax.semilogx(fr[m], Hs[m], color="k", lw=1.4)
    for p in res["transfer"]["peaks_hz"]:
        ax.axvline(p, color="#c03070", lw=0.6, alpha=0.6)
    for p in res["transfer"]["notches_hz"]:
        ax.axvline(p, color="#3060c0", lw=0.6, alpha=0.6)
    ax.set_title("the Body as a filter: |H| dry→wet (1/12 oct), peaks pink, notches blue", fontsize=9); ax.set_ylabel("dB")
    ax = fig.add_subplot(gs[1, 1]); ax.semilogx(fr[m], coh[m], color="#30a060"); ax.set_ylim(0, 1.02)
    ax.set_title(f"coherence (1 = linear; dips = nonlinear or noisy)   mean 80 Hz-8 kHz {res['transfer']['mean_coherence_80_8k']}", fontsize=9)
    ax = fig.add_subplot(gs[2, 0])
    for x, name, col in ((d, "dry", "k"), (w, "wet", "#c03070")):
        f, P = signal.welch(x, sr, nperseg=4096); db = 10 * np.log10(P + 1e-20); db = smooth_octave(f, db - db.max())
        ax.semilogx(f[1:], db[1:], color=col, label=name)
    ax.set_xlim(60, 16000); ax.set_ylim(-80, 3); ax.legend(); ax.set_title("LTAS, each normalised to its peak, 1/6 octave", fontsize=9)
    ax = fig.add_subplot(gs[2, 1]); ax.axis("off")
    lines = [f"{title}", f"delay {res['delay_ms']} ms   F0 {cd['f0'] and cd['f0']['median_hz']} -> {cw['f0'] and cw['f0']['median_hz']} Hz"]
    lines += [f"formants dry {cd['formant_medians_hz']}", f"formants wet {cw['formant_medians_hz']}"]
    lines += [f"{k}: {cd.get(k)} -> {cw.get(k)}  ({'+' if (v or 0) >= 0 else ''}{v})" for k, v in res["delta"].items()]
    ax.text(0, 1, "\n".join(lines), va="top", family="monospace", fontsize=9)
    fig.tight_layout(); fig.savefig(out); plt.close(fig)
