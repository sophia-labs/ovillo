from __future__ import annotations

import hashlib
import json
from pathlib import Path
import unittest

from run_unified_experiments import source_tree_sha256


ROOT = Path(__file__).resolve().parents[2]
ARTIFACTS = ROOT / "unified_artifacts"


def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


class FixtureArtifactTests(unittest.TestCase):
    def test_manifest_hashes_every_committed_artifact(self) -> None:
        manifest = json.loads((ARTIFACTS / "unified_manifest.json").read_text())
        self.assertEqual(manifest["schema"], "ariadne.artifact-manifest.v1")
        seen: set[str] = set()
        for entry in manifest["files"]:
            path = ARTIFACTS / entry["path"]
            self.assertTrue(path.is_file(), entry["path"])
            self.assertEqual(path.stat().st_size, entry["bytes"])
            self.assertEqual(file_sha256(path), entry["sha256"])
            seen.add(entry["path"])
        expected = {
            path.name
            for path in ARTIFACTS.iterdir()
            if path.is_file() and path.name != "unified_manifest.json"
        }
        self.assertEqual(seen, expected)

    def test_fixture_v2_has_binding_claim_and_oracle_identity(self) -> None:
        fixture = json.loads((ARTIFACTS / "unified_fixture_v2.json").read_text())
        required = {
            "schema",
            "source_commit",
            "generation_mode",
            "oracle",
            "claim",
            "family",
            "topology",
            "controls",
            "lowering",
            "physical_observable",
            "finite_time",
            "state_connection",
            "open_system",
            "audibility",
            "artifacts",
            "acceptance",
            "negative_results",
        }
        self.assertTrue(required.issubset(fixture))
        self.assertEqual(fixture["schema"], "ariadne.unified-instrument.fixture.v2")
        self.assertEqual(fixture["generation_mode"], "full")
        self.assertEqual(fixture["claim"]["reference_level"], "protected-acoustic")
        self.assertIsNone(fixture["claim"]["production_level"])
        self.assertEqual(
            fixture["oracle"]["source_tree_sha256"], source_tree_sha256(ROOT)
        )
        self.assertEqual(fixture["topology"]["bundle_rank_complex"], 2)
        self.assertEqual(fixture["topology"]["second_chern"], 1)
        self.assertGreater(
            fixture["topology"]["exact_gap"]["protected_cluster_gap"], 0.0
        )
        self.assertFalse(fixture["acceptance"]["production_rust_replay"])
        self.assertFalse(fixture["acceptance"]["lean_kernel_check"])
        self.assertFalse(fixture["acceptance"]["listener_study"])

    def test_schema_is_well_formed_json_and_matches_fixture_version(self) -> None:
        schema = json.loads(
            (ROOT / "schemas" / "unified_fixture_v2.schema.json").read_text()
        )
        self.assertEqual(schema["type"], "object")
        self.assertIn("claim", schema["required"])
        self.assertEqual(
            schema["properties"]["schema"]["const"],
            "ariadne.unified-instrument.fixture.v2",
        )


if __name__ == "__main__":
    unittest.main()
