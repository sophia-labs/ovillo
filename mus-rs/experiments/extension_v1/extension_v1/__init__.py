"""Extension-v1: gestural sequence computation in Clifford string media."""

from .clifford import CliffordConfig, exact_receipt
from .closed_gestures import (
    ClosedGestureConfig,
    ClosedCliffordGestureModel,
    commuting_control,
    train_closed_gestures,
)
from .development import DevelopmentConfig, development_receipt
from .geometry import GeometryConfig, geometry_receipt
from .morphology import MorphologyConfig, optimize_morphology

__all__ = [
    "CliffordConfig",
    "ClosedCliffordGestureModel",
    "ClosedGestureConfig",
    "DevelopmentConfig",
    "GeometryConfig",
    "MorphologyConfig",
    "commuting_control",
    "development_receipt",
    "exact_receipt",
    "geometry_receipt",
    "optimize_morphology",
    "train_closed_gestures",
]
