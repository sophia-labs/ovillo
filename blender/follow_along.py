"""Follow along: run this once inside Blender (Scripting workspace > Open > Run Script).

It builds the Ariadne steel-string scene now, then watches the builder and its data (the guitar's
visual anatomy, the render's energy trace, the audio) and rebuilds whenever any of them changes,
so the scene in front of you tracks the work as it happens. Run it again to stop watching.
"""
import os
import runpy
import time
import traceback

import bpy

def repo_root():
    here = globals().get("__file__", "")
    if not os.path.isfile(here):
        for text in bpy.data.texts:
            if text.filepath.endswith("follow_along.py"):
                here = bpy.path.abspath(text.filepath)
    return os.path.dirname(os.path.dirname(os.path.abspath(here)))


ROOT = repo_root()
BUILDER = os.path.join(ROOT, "blender/ariadne_scene.py")
# What to show: by default the README's render of the Doc Watson Wayfaring Stranger. Point these
# at any `mus render ... --energy` output to watch another performance.
OPTIONS = dict(
    visual=os.path.join(ROOT, "profiles/ariadne-steel-string-v0.visual.json"),
    trace=os.path.join(ROOT, "examples/tab/out/wayfaring-stranger-doc.energy.json"),
    audio=os.path.join(ROOT, "examples/tab/out/wayfaring-stranger-doc.wav"),
)
WATCH = [BUILDER] + list(OPTIONS.values())

state = bpy.app.driver_namespace.setdefault("ariadne_follow_along", {})


def log(line):
    text = bpy.data.texts.get("Ariadne log") or bpy.data.texts.new("Ariadne log")
    text.write(time.strftime("%H:%M:%S ") + line + "\n")
    print("[ariadne] " + line)


def tick():
    if not state.get("running"):
        return None
    stamps = {p: os.path.getmtime(p) for p in WATCH if os.path.exists(p)}
    if stamps != state.get("stamps"):
        state["stamps"] = stamps
        started = time.time()
        try:
            namespace = runpy.run_path(BUILDER, run_name="ariadne_follow_along",
                                       init_globals={"ARIADNE_OPTIONS": dict(OPTIONS)})
            namespace["build"]()
            log(f"rebuilt in {time.time() - started:.1f} s; press space to play")
        except Exception:
            log("rebuild failed:\n" + traceback.format_exc())
    return 2.0


if state.get("running"):
    state["running"] = False
    log("stopped following")
else:
    state["running"] = True
    state["stamps"] = None
    bpy.app.timers.register(tick, first_interval=0.1, persistent=True)
    log("following " + ", ".join(os.path.basename(p) for p in WATCH))
