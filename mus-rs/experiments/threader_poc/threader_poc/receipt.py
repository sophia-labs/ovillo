from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import numpy as np
from torch import Tensor


@dataclass(frozen=True)
class ArtifactDigest:
    name: str
    sha256: str
    shape: list[int]
    dtype: str


def tensor_digest(name: str, tensor: Tensor) -> ArtifactDigest:
    array = tensor.detach().cpu().contiguous().numpy()
    digest = hashlib.sha256(array.tobytes(order="C")).hexdigest()
    return ArtifactDigest(name=name, sha256=digest, shape=list(array.shape), dtype=str(array.dtype))


def canonical_digest(payload: dict[str, Any]) -> str:
    encoded = json.dumps(payload, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def write_receipt(path: Path, payload: dict[str, Any]) -> dict[str, Any]:
    body = dict(payload)
    body["receipt_sha256"] = canonical_digest(body)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(body, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return body


def save_wav(path: Path, waveform: Tensor, sample_rate: int) -> None:
    import wave

    audio = waveform.detach().cpu().numpy().astype(np.float32)
    audio = np.clip(audio, -0.999, 0.999)
    pcm = (audio * 32767.0).astype("<i2")
    path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(path), "wb") as handle:
        handle.setnchannels(1)
        handle.setsampwidth(2)
        handle.setframerate(sample_rate)
        handle.writeframes(pcm.tobytes())
