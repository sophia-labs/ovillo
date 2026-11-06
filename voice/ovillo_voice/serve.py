"""The studio page: http://127.0.0.1:8023, local only.

GET  /                      the page (studio.html)
GET  /api/index             the studio as JSON (the page polls it every 1.5 s)
GET  /files/<path>          a file under the studio (takes, renders, PNGs)
POST /api/record/start      {"take", "notes"?, "device"?}
POST /api/record/stop
POST /api/analyse           {"take"}
POST /api/sections          {"take", "sections": {name: [t0, t1] | null}}
POST /api/prompt/rec        {"retake"?}   record the current prompt as <session>-<id>[-n]
POST /api/prompt/goto       {"i"}
POST /api/roll/start | /api/roll/mark {"kind": next|again|back} | /api/roll/stop   one rolling take, cut per line
"""
import json, pathlib, subprocess, sys, threading, traceback
from http.server import ThreadingHTTPServer, BaseHTTPRequestHandler
from urllib.parse import unquote, urlparse
from . import studio, record, ops, roll

PAGE = pathlib.Path(__file__).with_name("studio.html")
BUSY = {}
TYPES = {".wav": "audio/wav", ".png": "image/png", ".json": "application/json", ".html": "text/html; charset=utf-8", ".log": "text/plain"}


def background(take, label, fn):
    def run():
        BUSY[take] = label
        try:
            fn(take)
        except Exception as e:
            studio.log("error", take=take, error=f"{label}: {e}")
        finally:
            BUSY.pop(take, None)
    threading.Thread(target=run, daemon=True).start()


CUTS = {}  # roll take -> the process cutting it


def stop_roll():
    """Halt the roll now and cut it in its own process (cuts queue on a lock): the page stays live, and the next set can
    roll while this one is transcribed and cut."""
    h = roll.halt()
    exe = pathlib.Path(sys.executable).with_name("ovillo-voice")
    log = open(studio.STUDIO / "takes" / h["roll"] / "cut.log", "w")
    CUTS[h["roll"]] = subprocess.Popen([str(exe), "cutroll", h["roll"]], stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
    return dict(h, cutting=True)


def cutting_now():
    for k, p in list(CUTS.items()):
        if p.poll() is not None:
            CUTS.pop(k)
    return sorted(CUTS)


class H(BaseHTTPRequestHandler):
    def log_message(self, *a):
        pass

    def send(self, code, body, ctype="application/json"):
        b = body if isinstance(body, bytes) else json.dumps(body).encode()
        self.send_response(code); self.send_header("Content-Type", ctype); self.send_header("Content-Length", str(len(b)))
        self.send_header("Cache-Control", "no-store"); self.end_headers(); self.wfile.write(b)

    def do_GET(self):
        path = unquote(urlparse(self.path).path)
        if path in ("/", "/index.html"):
            return self.send(200, PAGE.read_bytes(), TYPES[".html"])
        if path == "/api/devices":
            return self.send(200, dict(devices=record.devices(), chosen=record.settings().get("device") or record.DEFAULT_DEVICE))
        if path == "/api/index":
            ix = studio.index(); ix["busy"] = dict(BUSY, **({"_cut": cutting_now()} if cutting_now() else {})); return self.send(200, ix)
        if path.startswith("/files/"):
            p = (studio.STUDIO / path[len("/files/"):]).resolve()
            if studio.STUDIO.resolve() not in p.parents or not p.is_file():
                return self.send(404, {"error": "not found"})
            return self.send(200, p.read_bytes(), TYPES.get(p.suffix, "application/octet-stream"))
        self.send(404, {"error": "not found"})

    def do_POST(self):
        path = urlparse(self.path).path
        n = int(self.headers.get("Content-Length") or 0); body = json.loads(self.rfile.read(n) or b"{}")
        try:
            if path == "/api/record/start":
                return self.send(200, record.start(body["take"], body.get("device"), notes=body.get("notes", "")))
            if path == "/api/record/stop" and roll.state():
                return self.send(200, stop_roll())  # a bare stop mid-roll must still cut the roll
            if path == "/api/record/stop":
                r = record.stop()
                if (ops.take_meta(r["take"]).get("prompt")):
                    background(r["take"], "measuring", ops.quick)
                return self.send(200, r)
            if path == "/api/roll/start":
                return self.send(200, roll.start(body.get("device")))
            if path == "/api/roll/mark":
                return self.send(200, roll.mark(body.get("kind", "next")))
            if path == "/api/roll/stop":
                return self.send(200, stop_roll())
            if path == "/api/prompt/rec":
                P = ops.prompts(); i = P["current"]
                if not P["prompts"]:
                    return self.send(400, {"error": "no prompts yet"})
                take = ops.prompt_take(P, i, body.get("retake", False))
                pr = dict(P["prompts"][i], session=P["session"], index=i)
                return self.send(200, record.start(take, body.get("device"), chime=False, prompt=pr))
            if path == "/api/device":
                return self.send(200, record.set_device(body["device"]))
            if path == "/api/prompt/select":
                return self.send(200, ops.select_take(body.get("take")))
            if path == "/api/prompt/goto":
                return self.send(200, ops.goto_prompt(body["i"]))
            if path == "/api/sections":
                return self.send(200, ops.set_sections(body["take"], body["sections"]))
            if path == "/api/analyse":
                background(body["take"], "analysing", ops.analyse)
                return self.send(202, {"started": body["take"]})
        except Exception as e:
            traceback.print_exc(); return self.send(400, {"error": str(e)})
        self.send(404, {"error": "not found"})


def main(port=8023):
    srv = ThreadingHTTPServer(("127.0.0.1", port), H)
    print(f"ovillo-voice studio: http://127.0.0.1:{port}  (studio {studio.STUDIO})", flush=True)
    srv.serve_forever()
