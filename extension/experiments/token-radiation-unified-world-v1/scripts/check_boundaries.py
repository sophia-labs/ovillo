from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
for path in ROOT.rglob("*"):
    if not path.is_file():
        continue
    lowered = path.name.lower()
    if lowered.endswith((".epub", ".pdf", ".ckpt", ".pt", ".pth")):
        raise SystemExit(f"forbidden source or heavyweight model artifact: {path}")
print("artifact boundary verified")
