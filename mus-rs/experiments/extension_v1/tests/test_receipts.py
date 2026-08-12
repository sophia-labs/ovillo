from pathlib import Path

from extension_v1.receipts import digest_value, manifest, write_manifest


def test_canonical_digest_ignores_mapping_insertion_order() -> None:
    assert digest_value({"a": 1, "b": [2, 3]}) == digest_value(
        {"b": [2, 3], "a": 1}
    )


def test_manifest_omits_transient_and_self_referential_files(tmp_path: Path) -> None:
    (tmp_path / "kept.txt").write_text("evidence\n", encoding="utf-8")
    (tmp_path / "__pycache__").mkdir()
    (tmp_path / "__pycache__" / "ignored.pyc").write_bytes(b"cache")
    (tmp_path / ".pytest_cache").mkdir()
    (tmp_path / ".pytest_cache" / "ignored").write_text("cache", encoding="utf-8")
    (tmp_path / "ARTIFACT-MANIFEST.json").write_text("{}\n", encoding="utf-8")
    (tmp_path / "MANIFEST.sha256").write_text("stale\n", encoding="utf-8")

    values = write_manifest(tmp_path)

    assert list(values) == ["kept.txt"]
    assert manifest(tmp_path) == values
