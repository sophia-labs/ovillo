from pathlib import Path

from token_radiation_unified_world_v1.experiment import verify_result

ROOT = Path(__file__).resolve().parents[1]
verify_result(ROOT / "results" / "construction-20260815")
print("committed unified-world result verified")
