"""Recording Vera: the microphone straight to takes/<take>/dry.wav (48 kHz, 24-bit, mono), via ffmpeg.

start() returns at once and leaves ffmpeg running; stop() ends it cleanly (SIGINT, so the WAV header is
finished). A chime plays just before capture starts. Only one recording at a time; its state is
<studio>/.recording.json.
"""
import os, pathlib, re, signal, subprocess, sys, time
from . import studio

DEFAULT_DEVICE = "MacBook Pro Microphone"


def devices():
    out = subprocess.run(["ffmpeg", "-hide_banner", "-f", "avfoundation", "-list_devices", "true", "-i", ""],
                         capture_output=True, text=True).stderr
    audio = out.split("audio devices:", 1)[-1] if "audio devices:" in out else ""
    return [dict(index=int(i), name=n.strip()) for i, n in re.findall(r"\[(\d+)\] (.+)", audio)]


def settings():
    return studio.read_json(studio.STUDIO / "settings.json", {}) or {}


def set_device(name):
    s = settings(); s["device"] = name; studio.write_json(studio.STUDIO / "settings.json", s)
    studio.log("device", device=name); return s


def resolve(device):
    devs = devices()
    if device is None:
        device = settings().get("device") or DEFAULT_DEVICE
        if not any(str(device).lower() in d["name"].lower() for d in devs):
            device = DEFAULT_DEVICE  # the chosen mic is unplugged: fall back rather than fail
    if isinstance(device, int) or str(device).isdigit():
        return next(d for d in devs if d["index"] == int(device))
    m = [d for d in devs if str(device).lower() in d["name"].lower()]
    if not m:
        raise ValueError(f"no input device matching {device!r}; have {[d['name'] for d in devs]}")
    return m[0]


def _alive(pid):
    try:  # our own child (the page server started it): reap it, or it lingers as a zombie that looks alive
        done, _ = os.waitpid(pid, os.WNOHANG)
        return done != pid
    except ChildProcessError:
        pass
    try:
        os.kill(pid, 0); return True
    except OSError:
        return False


def status():
    r = studio.read_json(studio.STUDIO / ".recording.json")
    if r and not _alive(r["pid"]):
        (studio.STUDIO / ".recording.json").unlink(missing_ok=True); return None
    if r:
        r["elapsed_s"] = round(time.time() - r["started"], 1)
    return r


def start(take, device=None, max_seconds=900, notes="", chime=True, prompt=None):
    if status():
        raise RuntimeError(f"already recording {status()['take']}; stop it first")
    dev = resolve(device)
    out = studio.take_dir(take) / "dry.wav"
    if out.exists():
        raise FileExistsError(f"take {take!r} already has a dry.wav; pick a new name")
    # capture.py (CoreAudio via PortAudio) keeps every buffer; ffmpeg's avfoundation input dropped 7-27% of them
    p = subprocess.Popen([sys.executable, "-m", "ovillo_voice.capture", dev["name"], str(out), str(max_seconds)],
                         stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=open(out.with_suffix(".log"), "w"),
                         start_new_session=True, cwd=str(pathlib.Path(__file__).resolve().parent.parent))
    # The device takes 1-7 s to open, so chime only once audio is really arriving (0.25 s of samples on disk).
    # The chime lands in the take as a marker; analysis sections start after it.
    t0 = time.time()
    while time.time() - t0 < 15 and p.poll() is None:
        if out.exists() and out.stat().st_size > 44 + 48000 * 3 // 4:
            break
        time.sleep(0.03)
    if p.poll() is not None:
        raise RuntimeError(f"ffmpeg exited ({p.returncode}): {out.with_suffix('.log').read_text()[-500:]}")
    if chime:
        subprocess.Popen(["afplay", "/System/Library/Sounds/Tink.aiff"])
    rec = dict(take=take, pid=p.pid, device=dev["name"], started=time.time(), open_s=round(time.time() - t0, 2), path=str(out))
    studio.write_json(studio.STUDIO / ".recording.json", rec)
    studio.write_json(studio.take_dir(take) / "take.json", dict(take=take, device=dev["name"], recorded=time.strftime("%Y-%m-%d %H:%M:%S"),
                                                                notes=notes, sections={}, prompt=prompt))
    studio.log("record_start", take=take, device=dev["name"])
    return rec


def stop():
    r = status()
    if not r:
        raise RuntimeError("not recording")
    (studio.STUDIO / ".recording.json").unlink(missing_ok=True)  # first, so a second STOP finds nothing to stop
    dur = round(time.time() - r["started"], 1)
    os.kill(r["pid"], signal.SIGINT)
    for _ in range(100):
        if not _alive(r["pid"]):
            break
        time.sleep(0.05)
    studio.log("record_stop", take=r["take"], seconds=dur)
    subprocess.Popen(["afplay", "/System/Library/Sounds/Pop.aiff"])
    return dict(take=r["take"], path=r["path"], seconds=dur)


def adopt(take, wav, notes=""):
    """Bring an existing WAV in as a take (converted to 48 kHz mono 24-bit)."""
    out = studio.take_dir(take) / "dry.wav"
    subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", "-i", str(wav), "-ac", "1", "-ar", "48000", "-c:a", "pcm_s24le", str(out)], check=True)
    meta = studio.read_json(studio.take_dir(take) / "take.json", {}) or {}
    meta.update(take=take, device=meta.get("device", f"file:{wav}"), notes=notes or meta.get("notes", ""), sections=meta.get("sections", {}))
    studio.write_json(studio.take_dir(take) / "take.json", meta)
    studio.log("adopt", take=take, source=str(wav))
    return dict(take=take, path=str(out))
