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
from .trajectory_export import (
    TrajectoryExportError,
    closed_token_to_trajectory,
    program_digest,
    write_closed_token_trajectory,
)

__all__ = [
    "CliffordConfig",
    "ClosedCliffordGestureModel",
    "ClosedGestureConfig",
    "DevelopmentConfig",
    "GeometryConfig",
    "MorphologyConfig",
    "TrajectoryExportError",
    "closed_token_to_trajectory",
    "commuting_control",
    "development_receipt",
    "exact_receipt",
    "geometry_receipt",
    "optimize_morphology",
    "program_digest",
    "train_closed_gestures",
    "write_closed_token_trajectory",
]
