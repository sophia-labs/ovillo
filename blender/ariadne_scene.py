"""Build a Blender scene of an Ariadne steel-string world playing a score.

Inputs (all written by the repo's own tools):
  --visual  profiles/ariadne-steel-string-v0.visual.json   geometry + the top's 72 coupled mode shapes
  --trace   <render>.energy.json                           `mus render --energy` (v1: modes, flows, touches)
  --audio   <render>.wav                                   the same render, for sound in the timeline

What you see is the world's own state:
  * the top deforms as the sum of its real coupled mode shapes, each scaled by that mode's real
    energy (sqrt E / omega, the physical displacement law; high modes lifted by --flatten so eyes
    can find them) and oscillating at a slowed, log-compressed rate (the only invented thing);
  * each string glows and swings with its stored energy over its current speaking length; a finger
    sits on the fret the tab names; the pluck point flashes at every touch (thumb amber, fingers blue);
  * bridge columns light with the energy the bridge moves from each string into the top;
  * the air in the box glows with the air-dominated modes; shells leave the soundhole with radiated power;
  * the room's light follows the room field's stored energy.

Run inside Blender (Scripting > Open > Run Script, or via follow_along.py), or headless:
  blender --background --python blender/ariadne_scene.py -- --trace X.energy.json --audio X.wav --out X.blend
"""
import json
import math
import os
import sys

import bmesh
import bpy
import numpy as np

def repo_root():
    """The repo this file lives in, whether run from disk, from follow_along.py, or from Blender's text editor."""
    here = globals().get("__file__", "")
    if not os.path.isfile(here):
        for text in bpy.data.texts:
            if text.filepath.endswith("ariadne_scene.py"):
                here = bpy.path.abspath(text.filepath)
    return os.path.dirname(os.path.dirname(os.path.abspath(here)))


ROOT = repo_root()
OUT = os.path.join(ROOT, "examples/tab/out")
DEFAULTS = dict(
    visual=os.path.join(ROOT, "profiles/ariadne-steel-string-v0.visual.json"),
    trace=os.path.join(OUT, "wayfaring-stranger-doc.energy.json"),
    audio=os.path.join(OUT, "wayfaring-stranger-doc.wav"),
    out=None, fps=30, modes=24, flatten=0.6, seconds=None,
)
COLLECTION = "Ariadne"
THUMB, FINGERS = (1.0, 0.55, 0.15), (0.25, 0.65, 1.0)


def options():
    opts = dict(DEFAULTS)
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    for key, value in zip(argv[0::2], argv[1::2]):
        key = key.lstrip("-")
        opts[key] = type(DEFAULTS[key])(value) if isinstance(DEFAULTS.get(key), (int, float)) else value
    opts.update(globals().get("ARIADNE_OPTIONS", {}))         # follow_along.py passes overrides here
    return opts


# ---------------------------------------------------------------- scene plumbing


def ariadne_scene():
    """A dedicated scene, so nothing in your other scenes is touched (or in the way)."""
    scene = bpy.data.scenes.get("Ariadne") or bpy.data.scenes.new("Ariadne")
    for window in bpy.context.window_manager.windows:
        window.scene = scene
    return scene


def fresh_collection(scene):
    old = bpy.data.collections.get(COLLECTION)
    if old:
        for obj in list(old.all_objects):
            data = obj.data
            bpy.data.objects.remove(obj, do_unlink=True)
            if data is not None and data.users == 0:
                for pool in (bpy.data.meshes, bpy.data.curves, bpy.data.lights, bpy.data.cameras):
                    if data.name in pool and pool[data.name] == data:
                        pool.remove(data)
        bpy.data.collections.remove(old)
    for mat in [m for m in bpy.data.materials if m.name.startswith("ari.") and m.users == 0]:
        bpy.data.materials.remove(mat)
    col = bpy.data.collections.new(COLLECTION)
    scene.collection.children.link(col)
    return col


def material(name, color, emission=(0, 0, 0), strength=0.0, alpha=1.0, roughness=0.5, metallic=0.0):
    mat = bpy.data.materials.get("ari." + name) or bpy.data.materials.new("ari." + name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    bsdf.inputs["Base Color"].default_value = (*color, 1.0)
    bsdf.inputs["Roughness"].default_value = roughness
    bsdf.inputs["Metallic"].default_value = metallic
    bsdf.inputs["Emission Color"].default_value = (*emission, 1.0)
    bsdf.inputs["Emission Strength"].default_value = strength
    bsdf.inputs["Alpha"].default_value = alpha
    if alpha < 1.0:
        for attr, value in (("surface_render_method", "BLENDED"), ("blend_method", "BLEND")):
            try:
                setattr(mat, attr, value)
            except (AttributeError, TypeError):
                pass
    return mat


def mesh_object(col, name, verts, faces, mat=None, parent=None):
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata([tuple(v) for v in verts], [], [tuple(f) for f in faces])
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    col.objects.link(obj)
    if mat:
        obj.data.materials.append(mat)
    if parent:
        obj.parent = parent
    return obj


def box(col, name, center, size, mat, parent):
    cx, cy, cz = center
    sx, sy, sz = (s / 2 for s in size)
    v = [(cx + x * sx, cy + y * sy, cz + z * sz) for x in (-1, 1) for y in (-1, 1) for z in (-1, 1)]
    f = [(0, 1, 3, 2), (4, 6, 7, 5), (0, 4, 5, 1), (2, 3, 7, 6), (0, 2, 6, 4), (1, 5, 7, 3)]
    return mesh_object(col, name, v, f, mat, parent)


def sphere(col, name, radius, location, mat, parent, ico=False):
    mesh = bpy.data.meshes.new(name)
    bm = bmesh.new()
    if ico:
        bmesh.ops.create_icosphere(bm, subdivisions=3, radius=radius)
    else:
        bmesh.ops.create_uvsphere(bm, u_segments=32, v_segments=16, radius=radius)
    bm.to_mesh(mesh)
    bm.free()
    obj = bpy.data.objects.new(name, mesh)
    col.objects.link(obj)
    obj.location = location
    obj.data.materials.append(mat)
    obj.parent = parent
    return obj


# ---------------------------------------------------------------- keyframes, fast


def _fcurve(owner_id, data_path, index):
    ad = owner_id.animation_data or owner_id.animation_data_create()
    if ad.action is None:
        ad.action = bpy.data.actions.new(owner_id.name + ".ariadne")
    act = ad.action
    try:                                                     # the F-Curve API Blender 4.x still offers
        fc = act.fcurves.find(data_path, index=index) or act.fcurves.new(data_path, index=index)
        # 4.4+ files those curves under a slot; an action only animates through a slot that is bound
        if getattr(ad, "action_slot", 0) is None and len(getattr(act, "slots", [])):
            ad.action_slot = act.slots[0]
        return fc
    except AttributeError:
        pass
    slot = ad.action_slot or act.slots.new(owner_id.id_type, owner_id.name)
    ad.action_slot = slot
    layer = act.layers[0] if act.layers else act.layers.new("Layer")
    strip = layer.strips[0] if layer.strips else layer.strips.new(type="KEYFRAME")
    bag = strip.channelbag(slot, ensure=True)
    return bag.fcurves.find(data_path, index=index) or bag.fcurves.new(data_path, index=index)


def animate(owner_id, data_path, frames, values, index=0, step=False):
    fc = _fcurve(owner_id, data_path, index)
    fc.keyframe_points.clear()
    n = len(frames)
    fc.keyframe_points.add(n)
    co = np.empty(2 * n, dtype=np.float32)
    co[0::2], co[1::2] = frames, values
    fc.keyframe_points.foreach_set("co", co)
    fc.keyframe_points.foreach_set("interpolation", np.full(n, 0 if step else 1, dtype=np.int32))
    fc.update()


# ---------------------------------------------------------------- the guitar


def build_guitar(col, vis):
    root = bpy.data.objects.new("Guitar", None)
    col.objects.link(root)
    root.rotation_euler = (math.pi / 2, 0, math.radians(-30))   # body axis up, top turned 30 degrees off the camera so it can be seen to flex
    root.location = (0, 0, 0.12)
    depth = vis["depth"]
    wood = material("wood", (0.33, 0.17, 0.08), roughness=0.45)
    spruce = material("top", (0.62, 0.46, 0.26), emission=(1.0, 0.72, 0.38), strength=0.0, roughness=0.4)
    dark = material("ebony", (0.03, 0.025, 0.02), roughness=0.3)
    bone = material("bone", (0.92, 0.9, 0.82), roughness=0.3)
    metal = material("fret", (0.8, 0.8, 0.82), metallic=1.0, roughness=0.2)

    top = mesh_object(col, "Top", [(x, y, 0.0) for x, y in vis["top"]["vertices"]], vis["top"]["faces"], spruce, root)
    top.data.polygons.foreach_set("use_smooth", [True] * len(top.data.polygons))
    outline = vis["outline"]
    n = len(outline)
    side_v = [(x, y, 0.0) for x, y in outline] + [(x, y, -depth) for x, y in outline]
    side_f = [(i, (i + 1) % n, n + (i + 1) % n, n + i) for i in range(n)]
    mesh_object(col, "Sides", side_v, side_f, wood, root)
    mesh_object(col, "Back", [(x, y, -depth) for x, y in outline], [tuple(range(n - 1, -1, -1))], wood, root)
    air = material("air", (0.2, 0.6, 1.0), emission=(0.25, 0.7, 1.0), strength=0.0, alpha=0.12, roughness=1.0)
    inset = [(0.93 * x, y, z) for x, y, z in side_v]          # shrink across only: stays inside the waist
    air_obj = mesh_object(col, "Air", [(x, y, -0.006 if z == 0 else -depth + 0.006) for x, y, z in inset],
                          side_f + [tuple(range(n - 1, -1, -1)), tuple(range(n, 2 * n))], air, root)

    b = vis["bridge"]
    box(col, "Bridge", (0, b["y"] - 0.004, 0.0045), (2 * b["halfwidth"], b["depth"], 0.009), dark, root)
    box(col, "Saddle", (0, vis["saddle_y"], 0.0105), (0.075, 0.003, 0.004), bone, root)
    nk = vis["neck"]
    j, nut = nk["joint_y"], vis["nut_y"]
    wj, wn = nk["width_at_joint"] / 2, nk["width_at_nut"] / 2
    neck_v = [(-wj, j - 0.03, -0.02), (wj, j - 0.03, -0.02), (wn, nut, -0.02), (-wn, nut, -0.02),
              (-wj, j - 0.03, 0.0065), (wj, j - 0.03, 0.0065), (wn, nut, 0.0065), (-wn, nut, 0.0065)]
    neck_f = [(0, 3, 2, 1), (4, 5, 6, 7), (0, 1, 5, 4), (1, 2, 6, 5), (2, 3, 7, 6), (3, 0, 4, 7)]
    mesh_object(col, "Neck", neck_v, neck_f, wood, root)
    board_end = nk["frets_y"][19] - 0.006
    fb_v = [(-wj, board_end, 0.0065), (wj, board_end, 0.0065), (wn, nut, 0.0065), (-wn, nut, 0.0065),
            (-wj, board_end, 0.0095), (wj, board_end, 0.0095), (wn, nut, 0.0095), (-wn, nut, 0.0095)]
    mesh_object(col, "Fingerboard", fb_v, neck_f, dark, root)
    for k, y in enumerate(nk["frets_y"], start=1):
        w = wn + (wj - wn) * (nut - y) / (nut - j)
        box(col, f"Fret {k}", (0, y, 0.0102), (2 * w, 0.0022, 0.0016), metal, root)
    box(col, "Nut", (0, nut + 0.002, 0.0105), (2 * wn, 0.005, 0.005), bone, root)
    box(col, "Headstock", (0, nut + 0.095, -0.002), (0.085, 0.18, 0.016), wood, root)
    return root, top, air_obj, spruce, air


def string_point(s, y, saddle_y, nut_y):
    u = (y - saddle_y) / (nut_y - saddle_y)
    return s["saddle_x"] + u * (s["nut_x"] - s["saddle_x"]), 0.0125 - 0.001 * u


def build_strings(col, vis, root, frets_used):
    """Each string is a curve with one shape key per speaking length it uses: the fundamental's arc."""
    saddle_y, nut_y = vis["saddle_y"], vis["nut_y"]
    frets_y = [nut_y] + vis["neck"]["frets_y"]
    out = {}
    for s in vis["strings"]:
        curve = bpy.data.curves.new(f"String {s['number']}", "CURVE")
        curve.dimensions = "3D"
        curve.bevel_depth = max(0.0006, s["gauge_m"] * 0.9)
        curve.bevel_resolution = 2
        spline = curve.splines.new("POLY")
        ys = np.linspace(saddle_y, nut_y + 0.004, 120)
        spline.points.add(len(ys) - 1)
        for p, y in zip(spline.points, ys):
            x, z = string_point(s, y, saddle_y, nut_y)
            p.co = (x, y, z, 1.0)
        obj = bpy.data.objects.new(curve.name, curve)
        col.objects.link(obj)
        obj.parent = root
        glow = material(f"string {s['number']}", (0.75, 0.72, 0.68), emission=(1.0, 0.9, 0.7), strength=0.0,
                        metallic=1.0, roughness=0.25)
        curve.materials.append(glow)
        obj.shape_key_add(name="Basis")
        keys = {}
        for fret in sorted(frets_used.get(s["number"], {0})):
            stop = frets_y[min(fret, len(frets_y) - 1)] if fret > 0 else nut_y
            key = obj.shape_key_add(name=f"fret {fret}", from_mix=False)
            for p, y in zip(key.data, ys):
                if y < stop:
                    p.co.z += 0.004 * math.sin(math.pi * (y - saddle_y) / (stop - saddle_y))
            keys[fret] = key
        out[s["number"]] = dict(obj=obj, keys=keys, material=glow, spec=s)
    return out


# ---------------------------------------------------------------- the world's energies


def build_world_views(col, vis, root, top, strings):
    flows = {}
    for n, st in strings.items():
        s = st["spec"]
        mat = material(f"bridge flow {n}", (1.0, 0.45, 0.1), emission=(1.0, 0.45, 0.1), strength=0.0)
        flows[n] = (box(col, f"Bridge flow {n}", (s["saddle_x"], vis["saddle_y"], 0.006), (0.0035, 0.0035, 0.012), mat, root), mat)
    pool_mat = material("bridge pool", (1.0, 0.42, 0.1), emission=(1.0, 0.42, 0.1), strength=0.0, alpha=0.0, roughness=1.0)
    ring = [(0.13 * math.cos(a), vis["saddle_y"] + 0.07 * math.sin(a), 0.0016) for a in np.linspace(0, 2 * math.pi, 48, endpoint=False)]
    pool = mesh_object(col, "Bridge pool", [(0.0, vis["saddle_y"], 0.0016)] + ring,
                       [(0, 1 + i, 1 + (i + 1) % 48) for i in range(48)], pool_mat, root)
    flows["pool"] = (pool, pool_mat)
    shells = []
    hole = vis["soundhole"]
    for k in range(10):
        mat = material(f"shell {k}", (0.6, 0.85, 1.0), emission=(0.55, 0.85, 1.0), strength=0.0, alpha=0.0, roughness=1.0)
        shell = sphere(col, f"Radiation shell {k}", 1.0, (hole["center"][0], hole["center"][1], 0.0), mat, root)
        shell.data.polygons.foreach_set("use_smooth", [True] * len(shell.data.polygons))
        shell.scale = (0.001, 0.001, 0.001)
        shells.append((shell, mat))
    return flows, shells


def stage(col, scene):
    floor = material("floor", (0.02, 0.02, 0.025), roughness=0.8)
    mesh_object(col, "Floor", [(-4, -4, 0), (4, -4, 0), (4, 4, 0), (-4, 4, 0)], [(0, 1, 2, 3)], floor)
    cam = bpy.data.objects.get("Ariadne Camera")             # lives outside the rebuilt collection: your framing survives
    if cam is None:
        cam = bpy.data.objects.new("Ariadne Camera", bpy.data.cameras.new("Ariadne Camera"))
        cam.location = (0.62, -2.05, 0.98)
        cam.rotation_euler = mathutils_track(np.array([0.0, 0.0, 0.62]) - np.array(cam.location))
        cam.data.lens = 38
    if not cam.users_collection:
        scene.collection.objects.link(cam)
    scene.camera = cam
    for name, loc, energy, size in (("Key", (-1.3, -1.6, 1.9), 90, 1.6), ("Rim", (1.3, 1.1, 1.5), 70, 1.0)):
        light = bpy.data.lights.new(f"Ariadne {name}", "AREA")
        light.energy, light.size = energy, size
        obj = bpy.data.objects.new(f"Ariadne {name}", light)
        obj.location = loc
        obj.rotation_euler = (0, 0, 0)
        direction = -np.array(loc) + np.array([0, 0, 0.4])
        obj.rotation_euler = mathutils_track(direction)
        col.objects.link(obj)
    world = scene.world or bpy.data.worlds.new("Ariadne World")
    scene.world = world
    world.use_nodes = True
    bg = world.node_tree.nodes.get("Background")
    bg.inputs["Color"].default_value = (0.01, 0.012, 0.02, 1.0)
    bg.inputs["Strength"].default_value = 1.0
    return world


def mathutils_track(direction):
    from mathutils import Vector
    return Vector(direction).to_track_quat("-Z", "Y").to_euler()


def render_settings(scene, fps, frames):
    for engine in ("BLENDER_EEVEE_NEXT", "BLENDER_EEVEE"):
        try:
            scene.render.engine = engine
            break
        except TypeError:
            continue
    scene.render.fps = fps
    scene.render.resolution_x, scene.render.resolution_y = 1920, 1080
    scene.frame_start, scene.frame_end = 1, frames
    scene.view_settings.view_transform = "AgX" if "AgX" in [i.identifier for i in scene.view_settings.bl_rna.properties["view_transform"].enum_items] else "Filmic"
    scene.use_nodes = True
    tree = scene.node_tree
    tree.nodes.clear()
    layers = tree.nodes.new("CompositorNodeRLayers")
    glare = tree.nodes.new("CompositorNodeGlare")
    for attr, value in (("glare_type", "FOG_GLOW"), ("quality", "MEDIUM"), ("threshold", 1.2), ("size", 8)):
        try:
            setattr(glare, attr, value)
        except (AttributeError, TypeError):
            pass
    out = tree.nodes.new("CompositorNodeComposite")
    tree.links.new(layers.outputs["Image"], glare.inputs["Image"])
    tree.links.new(glare.outputs["Image"], out.inputs["Image"])


def attach_audio(scene, path):
    if not path or not os.path.exists(path):
        return
    if scene.sequence_editor is None:
        scene.sequence_editor_create()
    seq = scene.sequence_editor
    strips = getattr(seq, "strips", None) or seq.sequences
    for s in [s for s in strips if s.name == "Ariadne audio"]:
        strips.remove(s)
    strip = strips.new_sound("Ariadne audio", path, channel=1, frame_start=1)
    strip.volume = 1.0
    scene.sync_mode = "AUDIO_SYNC"


# ---------------------------------------------------------------- driving it from the trace


def drive(vis, trace, opts, scene, top, top_mat, air_mat, strings, flows, shells, world):
    fps = opts["fps"]
    w = trace["worlds"][0]
    hop, start = w["hopSeconds"], w["startSeconds"]
    total_s = start + hop * len(w["frames"])
    if opts.get("seconds"):
        total_s = min(total_s, float(opts["seconds"]))
    frames = np.arange(1, int(total_s * fps) + 1, dtype=np.float32)
    t = (frames - 1) / fps
    idx = np.clip(((t - start) / hop).astype(int), 0, len(w["frames"]) - 1)
    before = t < start

    def series(rows):
        a = np.asarray(rows, dtype=np.float64)[idx]
        a[before] = 0.0
        return a

    def smooth(x, seconds=0.06):
        k = max(1, int(seconds * fps))
        return np.convolve(x, np.ones(k) / k, mode="same")

    def level(x, floor_db=60.0):
        """Energy to 0..1 on a log scale spanning `floor_db` below its loudest moment."""
        peak = max(float(np.max(x)), 1e-30)
        return np.clip(1.0 + 10.0 * np.log10(np.maximum(x, 1e-30) / peak) / floor_db, 0.0, 1.0)

    bodies = w["channels"][:-1]
    energy = series(w["frames"])

    # -- the top: real mode shapes, real energies, slowed phase
    freq = np.array(vis["modes"]["frequency_hz"])
    modes = series(w["modeFrames"]) if "modeFrames" in w else None
    if modes is not None:
        amp = np.sqrt(2.0 * np.maximum(modes, 0.0)) / (2 * np.pi * freq)      # physical displacement law
        amp *= (freq / freq[0]) ** opts["flatten"]                            # lift high modes for the eye
        amp = np.column_stack([smooth(amp[:, k], 0.05) for k in range(amp.shape[1])])
        choose = np.argsort(-amp.max(axis=0))[: int(opts["modes"])]
        gain = 0.02 / max(float(amp[:, choose].max()), 1e-30)                 # loudest mode swings 20 mm (exaggerated)
        top.shape_key_add(name="Basis")
        shapes = np.array(vis["modes"]["shapes"])
        for k in choose:
            key = top.shape_key_add(name=f"mode {k + 1} ({freq[k]:.0f} Hz)", from_mix=False)
            shape = shapes[k] / max(np.abs(shapes[k]).max(), 1e-30)
            co = np.empty(3 * len(key.data), dtype=np.float32)
            key.data.foreach_get("co", co)
            co[2::3] += shape.astype(np.float32)
            key.data.foreach_set("co", co)
            key.slider_min, key.slider_max = -10.0, 10.0
            f_vis = 0.45 * math.sqrt(freq[k] / freq[0])                     # slowed, log-compressed
            values = gain * amp[:, k] * np.cos(2 * np.pi * f_vis * t + 0.7 * k)
            animate(top.data.shape_keys, f'key_blocks["{key.name}"].value', frames, values)
    radiated = smooth(series(w["radiationFrames"]).sum(axis=1)) if "radiationFrames" in w else None
    if radiated is not None:
        animate(top_mat.node_tree, 'nodes["Principled BSDF"].inputs["Emission Strength"].default_value', frames,
                1.2 * level(radiated, 40) ** 2)
        air_modes = np.array(vis["modes"]["air_share"])
        air_energy = (modes * air_modes).sum(axis=1) if modes is not None else energy[:, bodies.index("box/0")]
        animate(air_mat.node_tree, 'nodes["Principled BSDF"].inputs["Emission Strength"].default_value', frames,
                6.0 * level(smooth(air_energy), 40) ** 2)
        # shells: one per strong radiation onset, expanding from the soundhole
        onset = np.maximum(np.diff(radiated, prepend=radiated[0]), 0.0)
        strength = level(radiated, 30)
        picks = [i for i in range(1, len(onset) - 1) if onset[i] > 0 and onset[i] >= onset[i - 1] and onset[i] >= onset[i + 1]
                 and strength[i] > 0.45]
        picks = [i for k, i in enumerate(picks) if k == 0 or i - picks[k - 1] > fps * 0.12]
        life = int(1.2 * fps)
        scales = {k: np.full(len(frames), 0.001) for k in range(len(shells))}
        alphas = {k: np.zeros(len(frames)) for k in range(len(shells))}
        for n, i in enumerate(picks):
            k = n % len(shells)
            span = slice(i, min(i + life, len(frames)))
            age = np.arange(span.stop - span.start) / life
            scales[k][span] = 0.03 + 0.7 * age
            alphas[k][span] = 0.1 * strength[i] * (1 - age) ** 2
        for k, (shell, mat) in enumerate(shells):
            for axis in range(3):
                animate(shell, "scale", frames, scales[k], index=axis)
            animate(mat.node_tree, 'nodes["Principled BSDF"].inputs["Alpha"].default_value', frames, alphas[k])
            animate(mat.node_tree, 'nodes["Principled BSDF"].inputs["Emission Strength"].default_value', frames, 6 * alphas[k])
    if "room" in w["channels"]:
        room = smooth(energy[:, -1], 0.2)
        animate(world.node_tree, 'nodes["Background"].inputs["Strength"].default_value', frames, 0.6 + 2.5 * level(room, 30))

    # -- strings: stored energy, current speaking length, the touching hands
    touches = w.get("touches", [])
    stopped = {n: np.zeros(len(frames), dtype=int) for n in strings}
    for n in strings:
        cur, last = 0, 0
        for tc in [x for x in touches if x["string"] == n]:
            f = int(max(0.0, tc["seconds"] - (0.008 if tc["kind"] == "pluck" else 0.0)) * fps)
            stopped[n][last:f] = cur
            cur, last = int(tc["fret"]), f
        stopped[n][last:] = cur
    total_into_top = np.zeros(len(frames))
    for n, st in strings.items():
        e = energy[:, bodies.index(f"string/{n}")]
        lv = level(smooth(e, 0.04), 50)
        animate(st["material"].node_tree, 'nodes["Principled BSDF"].inputs["Emission Strength"].default_value', frames, 4.0 * lv ** 2)
        swing = np.sqrt(lv) * np.cos(2 * np.pi * 7.5 * t)                     # a visible shimmer, not the pitch
        for fret, key in st["keys"].items():
            animate(st["obj"].data.shape_keys, f'key_blocks["{key.name}"].value', frames, np.where(stopped[n] == fret, swing, 0.0))
        if "bridgeFrames" in w:
            edges = [i for i, e_id in enumerate(w["edges"]) if e_id.startswith(f"bridge/{n}/")]
            into_top = -series(w["bridgeFrames"])[:, edges].sum(axis=1) / hop     # energy per second leaving the string
            obj, mat = flows[n]
            animate(mat.node_tree, 'nodes["Principled BSDF"].inputs["Emission Strength"].default_value', frames,
                    12.0 * level(np.maximum(smooth(into_top), 0.0), 40) ** 2)
            total_into_top = total_into_top + into_top
    if "bridgeFrames" in w:
        pool, pool_mat = flows["pool"]
        crossing = level(np.maximum(smooth(total_into_top, 0.08), 0.0), 40)
        animate(pool_mat.node_tree, 'nodes["Principled BSDF"].inputs["Alpha"].default_value', frames, 0.55 * crossing)
        animate(pool_mat.node_tree, 'nodes["Principled BSDF"].inputs["Emission Strength"].default_value', frames, 5.0 * crossing ** 2)
    place_hands(vis, strings, touches, frames, fps)


def place_hands(vis, strings, touches, frames, fps):
    col = bpy.data.collections[COLLECTION]
    root = bpy.data.objects["Guitar"]
    saddle_y, nut_y = vis["saddle_y"], vis["nut_y"]
    frets_y = [nut_y] + vis["neck"]["frets_y"]
    for hand, color in (("thumb", THUMB), ("fingers", FINGERS)):
        for n, st in strings.items():
            mine = [tc for tc in touches if tc["string"] == n and (tc["hand"].endswith(".p") == (hand == "thumb"))]
            if not mine:
                continue
            s = st["spec"]
            pos = mine[0].get("pos", 0.18)
            y = saddle_y + pos * (nut_y - saddle_y) * 0.6
            x, z = string_point(s, y, saddle_y, nut_y)
            mat = material(f"{hand} {n}", color, emission=color, strength=0.0)
            sphere(col, f"{hand.title()} touch {n}", 0.0045, (x, y, z + 0.004), mat, root)
            flash = np.zeros(len(frames))
            for tc in mine:
                i = int(tc["seconds"] * fps)
                k = np.arange(i, min(i + int(0.25 * fps), len(frames)))
                flash[k] = np.maximum(flash[k], 30.0 * np.exp(-(k - i) / (0.06 * fps)))
            animate(mat.node_tree, 'nodes["Principled BSDF"].inputs["Emission Strength"].default_value', frames, flash)
    fretting = material("fretting finger", (0.95, 0.8, 0.75), emission=(1.0, 0.6, 0.5), strength=0.6)
    for n, st in strings.items():
        s = st["spec"]
        finger = sphere(col, f"Finger {n}", 0.0055, (0, 0, 0), fretting, root)
        cur = 0
        xs, ys, scl = np.zeros(len(frames)), np.zeros(len(frames)), np.zeros(len(frames))
        changes = [(0, 0)] + [(int(tc["seconds"] * fps), int(tc["fret"])) for tc in touches if tc["string"] == n]
        for k, (f0, fret) in enumerate(changes):
            f1 = changes[k + 1][0] if k + 1 < len(changes) else len(frames)
            y = (frets_y[fret] + 0.35 * (frets_y[fret - 1] - frets_y[fret])) if fret > 0 else nut_y
            x, z = string_point(s, y, saddle_y, nut_y)
            xs[f0:f1], ys[f0:f1], scl[f0:f1] = x, y, (1.0 if fret > 0 else 0.0)
        for axis, values in ((0, xs), (1, ys)):
            animate(finger, "location", frames, values, index=axis, step=True)
        animate(finger, "location", frames, np.full(len(frames), 0.017), index=2, step=True)
        for axis in range(3):
            animate(finger, "scale", frames, scl, index=axis, step=True)


def build(opts=None):
    opts = opts or options()
    scene = ariadne_scene()
    with open(opts["visual"]) as fh:
        vis = json.load(fh)
    trace = None
    if opts.get("trace") and os.path.exists(opts["trace"]):
        with open(opts["trace"]) as fh:
            trace = json.load(fh)
    col = fresh_collection(scene)
    root, top, air_obj, top_mat, air_mat = build_guitar(col, vis)
    touches = trace["worlds"][0].get("touches", []) if trace else []
    frets_used = {}
    for tc in touches:
        frets_used.setdefault(tc["string"], {0}).add(int(tc["fret"]))
    strings = build_strings(col, vis, root, frets_used)
    flows, shells = build_world_views(col, vis, root, top, strings)
    world = stage(col, scene)
    fps = int(opts["fps"])
    if trace:
        w = trace["worlds"][0]
        seconds = w["startSeconds"] + w["hopSeconds"] * len(w["frames"])
        if opts.get("seconds"):
            seconds = min(seconds, float(opts["seconds"]))
        render_settings(scene, fps, int(seconds * fps))
        drive(vis, trace, opts, scene, top, top_mat, air_mat, strings, flows, shells, world)
    else:
        render_settings(scene, fps, 1)
    attach_audio(scene, opts.get("audio"))
    scene.frame_set(1)
    if opts.get("out"):
        bpy.ops.wm.save_as_mainfile(filepath=os.path.abspath(opts["out"]))
    print(f"ariadne scene: {len(col.all_objects)} objects, {scene.frame_end} frames at {fps} fps"
          + ("" if trace else " (no trace yet: static guitar)"))


if __name__ == "__main__":
    build()
