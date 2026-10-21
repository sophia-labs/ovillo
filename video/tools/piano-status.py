"""Say whether the Ariadne piano World's demo render exists yet, and what the manifest uses.

    python3 tools/piano-status.py

The piano segment is a silent PLACEHOLDER until the other agent's branch
(ariadne/piano-world-v1-20261021, worktree ~/dev/ovillo-piano-20261021) leaves
ovillo/renders/piano-demo.wav, -receipt.json, -notes.tsv and -energy.json.
"""

import json
import os
import subprocess
from pathlib import Path

here = Path(__file__).resolve().parent.parent
manifest = json.loads((here / "manifest.json").read_text())
seg = next(s for s in manifest["segments"] if s["id"] == "piano")
real = seg.get("real", {})
d = Path(os.path.expanduser(real.get("dir", "")))
names = [real.get(k) for k in ("audio", "receipt", "events", "energy")]
present = {n: (d / n).exists() for n in names if n}
print(f"piano: manifest uses {'PLACEHOLDER (silent)' if seg.get('placeholder') else seg.get('audio')}")
print("piano: real render files: " + ", ".join(f"{n} {'present' if ok else 'missing'}" for n, ok in present.items()))
if d.exists():
    try:
        log = subprocess.run(["git", "-C", str(d.parent.parent), "log", "--oneline", "-3"], capture_output=True, text=True).stdout.strip()
        print("piano: branch head:\n  " + log.replace("\n", "\n  "))
    except OSError:
        pass
