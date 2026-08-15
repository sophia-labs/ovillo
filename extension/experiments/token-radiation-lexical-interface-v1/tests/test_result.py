import json
from pathlib import Path

from token_radiation_lexical_interface_v1.experiment import verify_result


def test_committed_result_verifies() -> None:
    root = Path(__file__).resolve().parents[1] / "results" / "construction-20260815"
    if not (root / "manifest.json").exists():
        return
    result = verify_result(root)
    assert result["format"].endswith("lexical-interface/v1")
    assert isinstance(result["scientific_digest"], str)
    json.dumps(result, allow_nan=False)
