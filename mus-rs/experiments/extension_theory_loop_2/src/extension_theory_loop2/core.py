"""Compatibility facade for the EXTENSION loop-2 reference objects."""

from .process import ObservableHMM, ProcessSpec, all_words, teacher_hankel
from .predictive_spectrum import (
    filter_jacobian,
    filter_lyapunov_spectrum,
    participation_rank,
    spectrum,
    tangent_basis,
)
from .models_clifford import (
    PredictiveModel,
    StableCliffordBank,
    clifford_generators,
)
from .models_state import (
    ExactPositiveRealization,
    LinearStateModel,
    PositiveProjectiveModel,
    SmallGRU,
)
from .training import TrainConfig, build_model, evaluate_model, train_predictive_model

__all__ = [
    "ExactPositiveRealization",
    "LinearStateModel",
    "ObservableHMM",
    "PositiveProjectiveModel",
    "PredictiveModel",
    "ProcessSpec",
    "SmallGRU",
    "StableCliffordBank",
    "TrainConfig",
    "all_words",
    "build_model",
    "clifford_generators",
    "evaluate_model",
    "filter_jacobian",
    "filter_lyapunov_spectrum",
    "participation_rank",
    "spectrum",
    "tangent_basis",
    "teacher_hankel",
    "train_predictive_model",
]
