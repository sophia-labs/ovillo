"""Canonical JSON receipts and artifact digests for Extension experiments."""
from __future__ import annotations

import fnmatch
import hashlib
import json
from pathlib import Path
from typing import Any, Iterable, Mapping


_DEFAULT_MANIFEST_EXCLUDES = (
    ".git/**",
    ".pytest_cache/**",
    "**/__pycache__/**",
    "**/*.pyc",
    "**/*.pyo",
    "**/*.egg-info/**",
    "ARTIFACT-MANIFEST.json",
    "MANIFEST.sha256",
)


def canonical_json_bytes(value: Any) -> bytes:
    """Serialize a JSON value canonically for content addressing."""
    return json.dumps(
        value,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
        allow_nan=False,
    ).encode("utf-8")


def digest_value(value: Any) -> str:
    """Return a SHA-256 digest of a canonical JSON value."""
    return hashlib.sha256(canonical_json_bytes(value)).hexdigest()


def write_receipt(path: Path, value: Mapping[str, Any]) -> str:
    """Write an indented receipt carrying the digest of its pre-envelope value."""
    path.parent.mkdir(parents=True, exist_ok=True)
    digest = digest_value(value)
    envelope = dict(value)
    envelope.setdefault("receipt", {})
    if isinstance(envelope["receipt"], dict):
        envelope["receipt"]["sha256"] = digest
    path.write_text(
        json.dumps(envelope, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    return digest


def file_sha256(path: Path) -> str:
    """Hash one file without loading it fully into memory."""
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        while chunk := stream.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def _is_excluded(relative: str, patterns: Iterable[str]) -> bool:
    # pathlib emits POSIX-style paths on the Linux research runner. Normalizing
    # here keeps manifests stable if the package is regenerated elsewhere.
    relative = relative.replace("\\", "/")
    return any(fnmatch.fnmatch(relative, pattern) for pattern in patterns)


def manifest(
    root: Path,
    *,
    excludes: Iterable[str] = _DEFAULT_MANIFEST_EXCLUDES,
) -> dict[str, str]:
    """Return a deterministic file manifest, omitting transient build state."""
    root = root.resolve()
    result: dict[str, str] = {}
    for path in sorted(root.rglob("*")):
        if not path.is_file():
            continue
        relative = path.relative_to(root).as_posix()
        if _is_excluded(relative, excludes):
            continue
        result[relative] = file_sha256(path)
    return result


def write_manifest(root: Path) -> dict[str, str]:
    """Write both JSON and sha256sum-compatible manifests for ``root``."""
    values = manifest(root)
    (root / "ARTIFACT-MANIFEST.json").write_text(
        json.dumps(values, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    lines = [f"{digest}  {relative}" for relative, digest in values.items()]
    (root / "MANIFEST.sha256").write_text(
        "\n".join(lines) + "\n",
        encoding="utf-8",
    )
    return values
