"""Small deterministic metrics used by the CPU construction."""

from __future__ import annotations

import math
from typing import Iterable

import numpy as np
import torch
import torch.nn.functional as F


def binary_auc(scores: Iterable[float], labels: Iterable[bool]) -> float:
    score = np.asarray(tuple(scores), dtype=np.float64)
    label = np.asarray(tuple(labels), dtype=bool)
    positive = score[label]
    negative = score[~label]
    if len(positive) == 0 or len(negative) == 0:
        raise ValueError("AUC requires positive and negative examples")
    greater = (positive[:, None] > negative[None, :]).mean()
    equal = (positive[:, None] == negative[None, :]).mean()
    return float(greater + 0.5 * equal)


def binary_f1(scores: np.ndarray, labels: np.ndarray, threshold: float) -> float:
    prediction = scores > threshold
    truth = labels.astype(bool)
    true_positive = np.logical_and(prediction, truth).sum()
    false_positive = np.logical_and(prediction, ~truth).sum()
    false_negative = np.logical_and(~prediction, truth).sum()
    return float(
        2 * true_positive
        / max(2 * true_positive + false_positive + false_negative, 1)
    )


def calibrate_threshold(scores: np.ndarray, labels: np.ndarray) -> tuple[float, float]:
    candidates = np.linspace(0.2, 0.8, 61)
    values = [binary_f1(scores, labels, threshold) for threshold in candidates]
    index = int(np.argmax(values))
    return float(candidates[index]), float(values[index])


def bits_per_byte(logits: torch.Tensor, targets: torch.Tensor) -> float:
    loss = F.cross_entropy(
        logits.reshape(-1, logits.shape[-1]),
        targets.reshape(-1),
        ignore_index=-100,
    )
    return float(loss.detach() / math.log(2.0))


def accuracy(logits: torch.Tensor, targets: torch.Tensor) -> float:
    mask = targets >= 0
    if not bool(mask.any()):
        raise ValueError("accuracy requires at least one valid target")
    return float((logits.argmax(dim=-1)[mask] == targets[mask]).float().mean())
