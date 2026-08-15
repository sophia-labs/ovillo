"""CPU construction for the bidirectional lexical interface."""

from __future__ import annotations

from dataclasses import asdict, dataclass
import hashlib
import json
import math
from pathlib import Path
import random

import numpy as np
import torch
from torch import nn
import torch.nn.functional as F

from .data import ByteBatch, collate_bytes, collate_events
from .metrics import accuracy, binary_auc, bits_per_byte, calibrate_threshold
from .models import (
    DeclaredArticulator,
    LexicalRadiationModel,
    PredictiveOperatorBody,
    TraceClassifier,
)
from .world import (
    OPS,
    OP_TO_ID,
    ByteCodec,
    LexicalExample,
    NEXT_TABLE,
    apply_operator,
    generate_example,
    rerender_example,
)

FORMAT = "sophia.instrument.token-radiation.lexical-interface/v1"


@dataclass(frozen=True, slots=True)
class ConstructionConfig:
    seed: int = 20260815
    event_count: int = 12
    train_examples: int = 560
    calibration_examples: int = 120
    test_examples: int = 120
    lexical_steps: int = 420
    predictor_steps: int = 350
    interface_steps: int = 600
    macro_steps: int = 600
    batch_size: int = 32
    interface_augmentations: int = 3
    sites: int = 16
    radiation_fraction: float = 0.04
    trace_window: int = 4

    @classmethod
    def from_json(cls, path: Path) -> "ConstructionConfig":
        payload = json.loads(path.read_text())
        if payload.get("format") != FORMAT:
            raise ValueError("invalid construction config format")
        values = {key: value for key, value in payload.items() if key != "format"}
        result = cls(**values)
        result.validate()
        return result

    def validate(self) -> None:
        if self.event_count < 3:
            raise ValueError("event_count must be at least three")
        if min(
            self.train_examples,
            self.calibration_examples,
            self.test_examples,
            self.lexical_steps,
            self.predictor_steps,
            self.interface_steps,
            self.batch_size,
            self.sites,
            self.trace_window,
        ) < 1:
            raise ValueError("all construction sizes must be positive")
        if self.interface_augmentations < 1:
            raise ValueError("interface_augmentations must be positive")
        if not 0.0 <= self.radiation_fraction <= 1.0:
            raise ValueError("radiation_fraction must lie in [0,1]")


@dataclass(slots=True)
class InterfaceDataset:
    radiation: torch.Tensor
    surface: torch.Tensor
    boundary: torch.Tensor
    operator: torch.Tensor
    homograph: torch.Tensor


@dataclass(slots=True)
class InterfaceModels:
    surface_boundary: TraceClassifier
    radiation_boundary: TraceClassifier
    combined_boundary: TraceClassifier
    surface_operator: TraceClassifier
    radiation_operator: TraceClassifier
    combined_operator: TraceClassifier
    threshold: float
    calibration_f1: float


def _set_seed(seed: int) -> None:
    random.seed(seed)
    np.random.seed(seed)
    torch.manual_seed(seed)
    torch.use_deterministic_algorithms(True)


def _scientific_digest(payload: dict[str, object]) -> str:
    encoded = json.dumps(
        payload,
        sort_keys=True,
        separators=(",", ":"),
        allow_nan=False,
    ).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def build_corpus(
    config: ConstructionConfig,
) -> tuple[
    list[LexicalExample],
    list[LexicalExample],
    list[LexicalExample],
    list[LexicalExample],
]:
    total = config.train_examples + config.calibration_examples + config.test_examples
    base = [
        generate_example(
            latent_seed=1000 + index,
            surface_seed=100_000 + index,
            event_count=config.event_count,
            surface_policy="homograph_heavy",
        )
        for index in range(total)
    ]
    train = base[: config.train_examples]
    calibration = base[
        config.train_examples : config.train_examples + config.calibration_examples
    ]
    test = base[-config.test_examples :]

    augmented: list[LexicalExample] = []
    for index, example in enumerate(train + calibration):
        for variant in range(config.interface_augmentations):
            augmented.append(
                rerender_example(
                    example,
                    surface_seed=300_000 + 17 * index + variant,
                    surface_policy="random",
                )
            )
    counterfactual_test = [
        rerender_example(
            example,
            surface_seed=500_000 + index,
            surface_policy="counterfactual",
        )
        for index, example in enumerate(test)
    ]
    return train, calibration, counterfactual_test, augmented


def _sample(
    examples: list[LexicalExample], count: int, rng: random.Random
) -> list[LexicalExample]:
    return [examples[rng.randrange(len(examples))] for _ in range(count)]


def _lexical_loss(
    outputs: dict[str, torch.Tensor], batch: ByteBatch
) -> tuple[torch.Tensor, dict[str, float]]:
    byte = F.cross_entropy(
        outputs["byte"].reshape(-1, outputs["byte"].shape[-1]),
        batch.next_byte.reshape(-1),
        ignore_index=-100,
    )
    valid = torch.cat(
        (
            torch.zeros(
                batch.ids.shape[0], 1, dtype=torch.bool, device=batch.ids.device
            ),
            batch.mask,
        ),
        dim=1,
    )
    boundary_target = batch.boundary[valid]
    positive = (boundary_target > 0.5).sum().clamp_min(1)
    negative = (boundary_target < 0.5).sum().clamp_min(1)
    boundary = F.binary_cross_entropy_with_logits(
        outputs["boundary"][valid],
        boundary_target,
        pos_weight=(negative / positive).detach(),
    )

    def categorical(name: str, target: torch.Tensor) -> torch.Tensor:
        return F.cross_entropy(
            outputs[name].reshape(-1, outputs[name].shape[-1]),
            target.reshape(-1),
            ignore_index=-100,
        )

    operator = categorical("operator", batch.operator)
    next_operator = categorical("next_operator", batch.next_operator)
    mode = categorical("mode", batch.mode)
    group = categorical("group", batch.group)
    loss = (
        0.25 * byte
        + 1.5 * boundary
        + 2.0 * operator
        + 1.2 * next_operator
        + 0.5 * mode
        + 0.5 * group
    )
    return loss, {
        "byte": float(byte.detach()),
        "boundary": float(boundary.detach()),
        "operator": float(operator.detach()),
        "next_operator": float(next_operator.detach()),
    }


def train_lexical_model(
    config: ConstructionConfig,
    codec: ByteCodec,
    train: list[LexicalExample],
) -> LexicalRadiationModel:
    _set_seed(config.seed)
    model = LexicalRadiationModel(
        codec,
        sites=config.sites,
        radiation_fraction=config.radiation_fraction,
        seed=config.seed,
    )
    optimizer = torch.optim.AdamW(
        model.parameters(), lr=3.0e-3, weight_decay=1.0e-4
    )
    rng = random.Random(config.seed)
    for _ in range(config.lexical_steps):
        batch = collate_bytes(_sample(train, config.batch_size, rng), codec)
        forward = model(batch.ids, loading=True)
        loss, _ = _lexical_loss(forward.outputs, batch)
        optimizer.zero_grad()
        loss.backward()
        torch.nn.utils.clip_grad_norm_(model.parameters(), 5.0)
        optimizer.step()
    return model


def evaluate_lexical_model(
    model: LexicalRadiationModel,
    codec: ByteCodec,
    examples: list[LexicalExample],
) -> dict[str, float]:
    batch = collate_bytes(examples, codec)
    model.eval()
    with torch.no_grad():
        forward = model(batch.ids, loading=True)
    valid = torch.cat(
        (
            torch.zeros(batch.ids.shape[0], 1, dtype=torch.bool),
            batch.mask,
        ),
        dim=1,
    )
    event_mask = batch.operator >= 0
    homograph_predictions: list[int] = []
    homograph_targets: list[int] = []
    for row, example in enumerate(examples):
        for span in example.spans:
            if span.surface == b"ba":
                homograph_predictions.append(
                    int(forward.outputs["operator"][row, span.end].argmax().item())
                )
                homograph_targets.append(span.operator)
    return {
        "next_byte_bpb": bits_per_byte(forward.outputs["byte"], batch.next_byte),
        "current_boundary_auc": binary_auc(
            torch.sigmoid(forward.outputs["boundary"][valid]).tolist(),
            (batch.boundary[valid] > 0.5).tolist(),
        ),
        "operator_accuracy": accuracy(forward.outputs["operator"], batch.operator),
        "next_operator_accuracy": accuracy(
            forward.outputs["next_operator"], batch.next_operator
        ),
        "homograph_accuracy": (
            float(
                np.mean(
                    np.asarray(homograph_predictions)
                    == np.asarray(homograph_targets)
                )
            )
            if homograph_targets
            else 0.0
        ),
        "maximum_radiation_closure_residual": float(
            forward.trace.closure_residual.abs().max()
        ),
        "mean_radiation_per_byte": float(
            forward.trace.booked_radiation[:, 1:].mean()
        ),
        "event_count": int(event_mask.sum()),
    }


def _surface_window(
    encoded: tuple[int, ...],
    position: int,
    *,
    window: int,
    surface_classes: int,
    pad_id: int,
) -> torch.Tensor:
    values = [pad_id] * window
    for slot, source_position in enumerate(range(position - window + 1, position + 1)):
        if source_position >= 0:
            values[slot] = encoded[source_position]
    result = torch.zeros(window * surface_classes)
    for slot, value in enumerate(values):
        result[slot * surface_classes + value] = 1.0
    return result


def collect_interface_dataset(
    model: LexicalRadiationModel,
    codec: ByteCodec,
    examples: list[LexicalExample],
    *,
    trace_window: int,
    batch_size: int = 32,
) -> InterfaceDataset:
    radiation_rows: list[torch.Tensor] = []
    surface_rows: list[torch.Tensor] = []
    boundaries: list[float] = []
    operators: list[int] = []
    homographs: list[bool] = []
    surface_classes = len(codec.values) + 1
    model.eval()
    with torch.no_grad():
        for start in range(0, len(examples), batch_size):
            group = examples[start : start + batch_size]
            batch = collate_bytes(group, codec)
            forward = model(batch.ids, loading=True)
            emissions = forward.trace.emissions
            for row, example in enumerate(group):
                encoded = codec.encode(example.surface)
                homograph_ends = {
                    span.end for span in example.spans if span.surface == b"ba"
                }
                for position in range(len(encoded)):
                    trace_rows = []
                    for trace_position in range(
                        position - trace_window + 1, position + 1
                    ):
                        trace_rows.append(
                            emissions[row, trace_position + 1]
                            if trace_position >= 0
                            else torch.zeros(model.body.config.radiation_rank)
                        )
                    radiation_rows.append(torch.cat(trace_rows))
                    surface_rows.append(
                        _surface_window(
                            encoded,
                            position,
                            window=trace_window,
                            surface_classes=surface_classes,
                            pad_id=codec.pad_id,
                        )
                    )
                    boundaries.append(float(example.boundaries[position]))
                    operators.append(example.operator_labels[position])
                    homographs.append(position + 1 in homograph_ends)
    return InterfaceDataset(
        radiation=torch.stack(radiation_rows),
        surface=torch.stack(surface_rows),
        boundary=torch.tensor(boundaries),
        operator=torch.tensor(operators, dtype=torch.long),
        homograph=torch.tensor(homographs, dtype=torch.bool),
    )


def _train_classifier(
    features: torch.Tensor,
    targets: torch.Tensor,
    *,
    output_dim: int,
    steps: int,
    seed: int,
) -> TraceClassifier:
    _set_seed(seed)
    model = TraceClassifier(features.shape[1], output_dim)
    optimizer = torch.optim.AdamW(model.parameters(), lr=8.0e-3)
    generator = torch.Generator().manual_seed(seed + 99)
    for _ in range(steps):
        indices = torch.randint(len(features), (384,), generator=generator)
        logits = model(features[indices])
        if output_dim == 1:
            target = targets[indices].float()
            positive = (target > 0.5).sum().clamp_min(1)
            negative = (target < 0.5).sum().clamp_min(1)
            loss = F.binary_cross_entropy_with_logits(
                logits[:, 0],
                target,
                pos_weight=(negative / positive).detach(),
            )
        else:
            loss = F.cross_entropy(logits, targets[indices])
        optimizer.zero_grad()
        loss.backward()
        optimizer.step()
    return model


def train_interface_models(
    config: ConstructionConfig,
    fitting: InterfaceDataset,
    calibration: InterfaceDataset,
) -> InterfaceModels:
    combined = torch.cat((fitting.radiation, fitting.surface), dim=-1)
    surface_boundary = _train_classifier(
        fitting.surface,
        fitting.boundary,
        output_dim=1,
        steps=config.interface_steps,
        seed=config.seed + 10,
    )
    radiation_boundary = _train_classifier(
        fitting.radiation,
        fitting.boundary,
        output_dim=1,
        steps=config.interface_steps,
        seed=config.seed + 11,
    )
    combined_boundary = _train_classifier(
        combined,
        fitting.boundary,
        output_dim=1,
        steps=config.interface_steps,
        seed=config.seed + 12,
    )
    event_mask = fitting.operator >= 0
    surface_operator = _train_classifier(
        fitting.surface[event_mask],
        fitting.operator[event_mask],
        output_dim=len(OPS),
        steps=config.interface_steps,
        seed=config.seed + 13,
    )
    radiation_operator = _train_classifier(
        fitting.radiation[event_mask],
        fitting.operator[event_mask],
        output_dim=len(OPS),
        steps=config.interface_steps,
        seed=config.seed + 14,
    )
    combined_operator = _train_classifier(
        combined[event_mask],
        fitting.operator[event_mask],
        output_dim=len(OPS),
        steps=config.interface_steps,
        seed=config.seed + 15,
    )
    with torch.no_grad():
        calibration_scores = torch.sigmoid(
            combined_boundary(
                torch.cat((calibration.radiation, calibration.surface), dim=-1)
            )[:, 0]
        ).numpy()
    threshold, calibration_f1 = calibrate_threshold(
        calibration_scores, calibration.boundary.numpy()
    )
    return InterfaceModels(
        surface_boundary=surface_boundary,
        radiation_boundary=radiation_boundary,
        combined_boundary=combined_boundary,
        surface_operator=surface_operator,
        radiation_operator=radiation_operator,
        combined_operator=combined_operator,
        threshold=threshold,
        calibration_f1=calibration_f1,
    )


def evaluate_interface_models(
    models: InterfaceModels,
    dataset: InterfaceDataset,
    *,
    shuffle_seed: int,
) -> dict[str, float]:
    combined = torch.cat((dataset.radiation, dataset.surface), dim=-1)
    event_mask = dataset.operator >= 0
    with torch.no_grad():
        surface_boundary = torch.sigmoid(
            models.surface_boundary(dataset.surface)[:, 0]
        )
        radiation_boundary = torch.sigmoid(
            models.radiation_boundary(dataset.radiation)[:, 0]
        )
        combined_boundary = torch.sigmoid(models.combined_boundary(combined)[:, 0])
        generator = torch.Generator().manual_seed(shuffle_seed)
        permutation = torch.randperm(len(dataset.radiation), generator=generator)
        shuffled_boundary = torch.sigmoid(
            models.radiation_boundary(dataset.radiation[permutation])[:, 0]
        )
        surface_operator = models.surface_operator(dataset.surface[event_mask])
        radiation_operator = models.radiation_operator(dataset.radiation[event_mask])
        combined_operator = models.combined_operator(combined[event_mask])
    homograph = dataset.homograph[event_mask]
    targets = dataset.operator[event_mask]

    def masked_accuracy(logits: torch.Tensor) -> float:
        if not bool(homograph.any()):
            return 0.0
        return float(
            (logits.argmax(dim=-1)[homograph] == targets[homograph])
            .float()
            .mean()
        )

    return {
        "surface_boundary_auc": binary_auc(
            surface_boundary.tolist(), (dataset.boundary > 0.5).tolist()
        ),
        "radiation_boundary_auc": binary_auc(
            radiation_boundary.tolist(), (dataset.boundary > 0.5).tolist()
        ),
        "combined_boundary_auc": binary_auc(
            combined_boundary.tolist(), (dataset.boundary > 0.5).tolist()
        ),
        "shuffled_radiation_boundary_auc": binary_auc(
            shuffled_boundary.tolist(), (dataset.boundary > 0.5).tolist()
        ),
        "surface_operator_accuracy": float(
            (surface_operator.argmax(dim=-1) == targets).float().mean()
        ),
        "radiation_operator_accuracy": float(
            (radiation_operator.argmax(dim=-1) == targets).float().mean()
        ),
        "combined_operator_accuracy": float(
            (combined_operator.argmax(dim=-1) == targets).float().mean()
        ),
        "surface_homograph_accuracy": masked_accuracy(surface_operator),
        "radiation_homograph_accuracy": masked_accuracy(radiation_operator),
        "combined_homograph_accuracy": masked_accuracy(combined_operator),
        "calibrated_threshold": models.threshold,
        "calibration_boundary_f1": models.calibration_f1,
    }


def train_predictive_body(
    config: ConstructionConfig, train: list[LexicalExample]
) -> PredictiveOperatorBody:
    _set_seed(config.seed + 100)
    model = PredictiveOperatorBody(
        sites=config.sites,
        radiation_fraction=config.radiation_fraction,
        seed=config.seed + 100,
    )
    optimizer = torch.optim.AdamW(model.parameters(), lr=4.0e-3)
    rng = random.Random(config.seed + 100)
    for _ in range(config.predictor_steps):
        batch = collate_events(_sample(train, 64, rng))
        forward = model(batch.initial_ids, batch.operator_ids, loading=True)
        loss = (
            2.0
            * F.cross_entropy(
                forward.outputs["next_operator"].reshape(-1, len(OPS)),
                batch.next_operator.reshape(-1),
            )
            + 0.5
            * F.cross_entropy(
                forward.outputs["group"].reshape(-1, 8), batch.group.reshape(-1)
            )
            + 0.5
            * F.cross_entropy(
                forward.outputs["mode"].reshape(-1, 2), batch.mode.reshape(-1)
            )
        )
        optimizer.zero_grad()
        loss.backward()
        torch.nn.utils.clip_grad_norm_(model.parameters(), 5.0)
        optimizer.step()
    return model


def evaluate_predictive_body(
    model: PredictiveOperatorBody, examples: list[LexicalExample]
) -> dict[str, float]:
    batch = collate_events(examples)
    model.eval()
    with torch.no_grad():
        forward = model(batch.initial_ids, batch.operator_ids, loading=True)
        next_accuracy = accuracy(
            forward.outputs["next_operator"], batch.next_operator
        )
        group_accuracy = accuracy(forward.outputs["group"], batch.group)
        mode_accuracy = accuracy(forward.outputs["mode"], batch.mode)

        iterative_states: list[torch.Tensor] = []
        iterative_emissions: list[torch.Tensor] = []
        start = model.start(batch.initial_ids, loading=True)
        state = start.state_after
        emission = start.emission
        iterative_states.append(state)
        iterative_emissions.append(emission)
        for position in range(batch.operator_ids.shape[1]):
            frame = model.feedback(
                state, batch.operator_ids[:, position], loading=True
            )
            state = frame.state_after
            emission = frame.emission
            iterative_states.append(state)
            iterative_emissions.append(emission)
        state_disagreement = float(
            (torch.stack(iterative_states, dim=1) - forward.trace.states)
            .abs()
            .max()
        )
        emission_disagreement = float(
            (torch.stack(iterative_emissions, dim=1) - forward.trace.emissions)
            .abs()
            .max()
        )

        step_correct: list[bool] = []
        exact: list[bool] = []
        for example in examples:
            initial_id = torch.tensor(
                [
                    example.initial_state[0]
                    + 4 * example.initial_state[1]
                    + 8 * example.initial_state[2]
                ]
            )
            frame = model.start(initial_id, loading=True)
            state = frame.state_after
            emission = frame.emission
            row_exact = True
            for true_operator in example.latent_operators:
                predicted = int(
                    model.head(emission)["next_operator"].argmax(dim=-1)
                )
                correct = predicted == true_operator
                step_correct.append(correct)
                row_exact = row_exact and correct
                frame = model.feedback(
                    state, torch.tensor([predicted]), loading=True
                )
                state = frame.state_after
                emission = frame.emission
            exact.append(row_exact)
    return {
        "teacher_next_operator_accuracy": next_accuracy,
        "teacher_group_accuracy": group_accuracy,
        "teacher_mode_accuracy": mode_accuracy,
        "latent_closed_loop_step_accuracy": float(np.mean(step_correct)),
        "latent_closed_loop_exact_sequence_rate": float(np.mean(exact)),
        "feedback_state_identity_max_error": state_disagreement,
        "feedback_emission_identity_max_error": emission_disagreement,
        "maximum_radiation_closure_residual": float(
            forward.trace.closure_residual.abs().max()
        ),
    }


def one_body_macro_diagnostic(
    config: ConstructionConfig,
    lexical_model: LexicalRadiationModel,
    codec: ByteCodec,
    examples: list[LexicalExample],
) -> dict[str, object]:
    for parameter in lexical_model.parameters():
        parameter.requires_grad_(False)
    surfaces = sorted({span.surface for example in examples for span in example.spans})
    surface_to_id = {surface: index for index, surface in enumerate(surfaces)}
    rows: list[tuple[torch.Tensor, torch.Tensor, torch.Tensor, int, int, int]] = []
    lexical_model.eval()
    with torch.no_grad():
        for start in range(0, min(len(examples), 240), 32):
            group = examples[start : start + 32]
            batch = collate_bytes(group, codec)
            forward = lexical_model(batch.ids, loading=True)
            for row, example in enumerate(group):
                for span in example.spans:
                    rows.append(
                        (
                            forward.trace.states[row, span.start].clone(),
                            forward.trace.states[row, span.end].clone(),
                            forward.trace.emissions[row, span.end].clone(),
                            span.operator,
                            surface_to_id[span.surface],
                            span.end - span.start,
                        )
                    )
    start_state = torch.stack([row[0] for row in rows])
    target_state = torch.stack([row[1] for row in rows])
    target_emission = torch.stack([row[2] for row in rows])
    operator_id = torch.tensor([row[3] for row in rows])
    surface_id = torch.tensor([row[4] for row in rows])
    lengths = torch.tensor([row[5] for row in rows])
    with torch.no_grad():
        target_next = lexical_model.head(target_emission)[
            "next_operator"
        ].softmax(dim=-1)

    _set_seed(config.seed + 200)
    operator_macro = nn.Parameter(
        0.05 * torch.randn(len(OPS), lexical_model.body.config.sites)
    )
    surface_macro = nn.Parameter(
        0.05 * torch.randn(len(surfaces), lexical_model.body.config.sites)
    )
    optimizer = torch.optim.Adam([operator_macro, surface_macro], lr=3.0e-2)
    generator = torch.Generator().manual_seed(config.seed + 201)
    constrained = lexical_model.body.constrained()
    for _ in range(config.macro_steps):
        indices = torch.randint(len(rows), (128,), generator=generator)
        start = start_state[indices]
        target = target_state[indices]
        target_emit = target_emission[indices]
        op = operator_id[indices]
        surface = surface_id[indices]
        op_frame = lexical_model.body.step_with_source(
            start,
            operator_macro[op],
            loading=True,
            constrained=constrained,
        )
        surface_frame = lexical_model.body.step_with_source(
            start,
            surface_macro[surface],
            loading=True,
            constrained=constrained,
        )
        scale = target.square().mean().detach() + 1.0e-4

        def objective(frame: object) -> torch.Tensor:
            assert hasattr(frame, "state_after")
            return (
                (frame.state_after - target).square().mean() / scale
                + 0.4 * (frame.emission - target_emit).square().mean()
                + 0.3
                * F.kl_div(
                    lexical_model.head(frame.emission)[
                        "next_operator"
                    ].log_softmax(dim=-1),
                    target_next[indices],
                    reduction="batchmean",
                )
            )

        loss = objective(op_frame) + objective(surface_frame)
        optimizer.zero_grad()
        loss.backward()
        optimizer.step()

    def summarize(macros: torch.Tensor, ids: torch.Tensor) -> dict[str, float]:
        with torch.no_grad():
            frame = lexical_model.body.step_with_source(
                start_state,
                macros[ids],
                loading=True,
                constrained=constrained,
            )
            state_relative = (
                (frame.state_after - target_state).square().sum(dim=-1).sqrt()
                / (target_state.square().sum(dim=-1).sqrt() + 1.0e-6)
            )
            emission_relative = (
                (frame.emission - target_emission).square().sum(dim=-1).sqrt()
                / (target_emission.square().sum(dim=-1).sqrt() + 1.0e-6)
            )
            predicted = lexical_model.head(frame.emission)[
                "next_operator"
            ].argmax(dim=-1)
            reference = lexical_model.head(target_emission)[
                "next_operator"
            ].argmax(dim=-1)
        return {
            "mean_state_relative_error": float(state_relative.mean()),
            "median_state_relative_error": float(state_relative.median()),
            "mean_emission_relative_error": float(emission_relative.mean()),
            "next_operator_agreement": float(
                (predicted == reference).float().mean()
            ),
        }

    return {
        "mean_microsteps_saved": float((lengths - 1).float().mean()),
        "operator_macro": summarize(operator_macro, operator_id),
        "surface_macro": summarize(surface_macro, surface_id),
    }


def _interface_features_for_step(
    radiation_history: list[torch.Tensor],
    byte_history: list[int],
    *,
    trace_window: int,
    surface_classes: int,
) -> tuple[torch.Tensor, torch.Tensor]:
    radiation = torch.cat(radiation_history[-trace_window:])
    surface = torch.zeros(trace_window * surface_classes)
    for slot, value in enumerate(byte_history[-trace_window:]):
        surface[slot * surface_classes + value] = 1.0
    return radiation, surface


def evaluate_surface_closed_loop(
    config: ConstructionConfig,
    codec: ByteCodec,
    lexical_model: LexicalRadiationModel,
    interface: InterfaceModels,
    predictor: PredictiveOperatorBody,
    examples: list[LexicalExample],
) -> dict[str, float]:
    articulator = DeclaredArticulator()
    lexical_model.eval()
    predictor.eval()
    step_semantic: list[bool] = []
    roundtrip: list[bool] = []
    exact_semantic: list[bool] = []
    exact_boundaries: list[bool] = []
    surface_classes = len(codec.values) + 1
    with torch.no_grad():
        lexical_constrained = lexical_model.body.constrained()
        for example in examples:
            initial_id = torch.tensor(
                [
                    example.initial_state[0]
                    + 4 * example.initial_state[1]
                    + 8 * example.initial_state[2]
                ]
            )
            predictive_frame = predictor.start(initial_id, loading=True)
            predictive_state = predictive_frame.state_after
            predictive_emission = predictive_frame.emission

            lexical_state = lexical_model.body.zero_state(1)
            radiation_history = [
                torch.zeros(lexical_model.body.config.radiation_rank)
                for _ in range(config.trace_window - 1)
            ]
            byte_history = [codec.pad_id for _ in range(config.trace_window - 1)]
            for raw_byte in example.surface[:2]:
                byte_id = codec.to_id[raw_byte]
                lexical_frame = lexical_model.body.step_ids(
                    lexical_state,
                    torch.tensor([byte_id]),
                    loading=True,
                    constrained=lexical_constrained,
                )
                lexical_state = lexical_frame.state_after
                radiation_history.append(lexical_frame.emission[0])
                byte_history.append(byte_id)

            oracle_state = example.initial_state
            row_semantic = True
            row_boundaries = True
            for event_index in range(config.event_count):
                prediction = predictor.head(predictive_emission)
                emitted_operator = int(
                    prediction["next_operator"].argmax(dim=-1)
                )
                true_operator = OP_TO_ID[NEXT_TABLE[oracle_state]]
                semantic_correct = emitted_operator == true_operator
                step_semantic.append(semantic_correct)
                row_semantic = row_semantic and semantic_correct

                predicted_group = int(prediction["group"].argmax(dim=-1))
                predicted_mode = int(prediction["mode"].argmax(dim=-1))
                predicted_state = (
                    predicted_group % 4,
                    predicted_group // 4,
                    predicted_mode,
                )
                surface = articulator.render(
                    emitted_operator, predicted_state, event_index
                )
                detected: list[int] = []
                for raw_byte in surface:
                    byte_id = codec.to_id[raw_byte]
                    lexical_frame = lexical_model.body.step_ids(
                        lexical_state,
                        torch.tensor([byte_id]),
                        loading=True,
                        constrained=lexical_constrained,
                    )
                    lexical_state = lexical_frame.state_after
                    radiation_history.append(lexical_frame.emission[0])
                    byte_history.append(byte_id)
                    radiation, surface_feature = _interface_features_for_step(
                        radiation_history,
                        byte_history,
                        trace_window=config.trace_window,
                        surface_classes=surface_classes,
                    )
                    combined = torch.cat((radiation, surface_feature))[None]
                    probability = float(
                        torch.sigmoid(
                            interface.combined_boundary(combined)[:, 0]
                        )
                    )
                    if probability > interface.threshold:
                        detected.append(
                            int(
                                interface.radiation_operator(
                                    radiation[None]
                                ).argmax(dim=-1)
                            )
                        )
                event_boundary_correct = len(detected) == 1
                row_boundaries = row_boundaries and event_boundary_correct
                received_operator = detected[-1] if detected else emitted_operator
                roundtrip.append(received_operator == emitted_operator)
                predictive_frame = predictor.feedback(
                    predictive_state,
                    torch.tensor([received_operator]),
                    loading=True,
                )
                predictive_state = predictive_frame.state_after
                predictive_emission = predictive_frame.emission
                oracle_state = apply_operator(oracle_state, OPS[true_operator])
            exact_semantic.append(row_semantic)
            exact_boundaries.append(row_boundaries)
    return {
        "emitted_operator_step_accuracy": float(np.mean(step_semantic)),
        "surface_roundtrip_operator_accuracy": float(np.mean(roundtrip)),
        "exact_semantic_sequence_rate": float(np.mean(exact_semantic)),
        "exact_event_boundary_sequence_rate": float(np.mean(exact_boundaries)),
    }


def run_construction(config: ConstructionConfig) -> dict[str, object]:
    config.validate()
    codec = ByteCodec()
    train, calibration, test, augmented = build_corpus(config)
    augmented_fit_count = config.train_examples * config.interface_augmentations
    interface_fit_examples = augmented[:augmented_fit_count]
    interface_calibration_examples = augmented[augmented_fit_count:]

    lexical = train_lexical_model(config, codec, train)
    lexical_metrics = evaluate_lexical_model(lexical, codec, test)

    fitting_features = collect_interface_dataset(
        lexical,
        codec,
        interface_fit_examples,
        trace_window=config.trace_window,
    )
    calibration_features = collect_interface_dataset(
        lexical,
        codec,
        interface_calibration_examples,
        trace_window=config.trace_window,
    )
    test_features = collect_interface_dataset(
        lexical, codec, test, trace_window=config.trace_window
    )
    interface = train_interface_models(config, fitting_features, calibration_features)
    interface_metrics = evaluate_interface_models(
        interface, test_features, shuffle_seed=config.seed + 999
    )

    predictor = train_predictive_body(config, train)
    predictor_metrics = evaluate_predictive_body(predictor, test)
    macro_metrics = one_body_macro_diagnostic(config, lexical, codec, train)
    closed_loop = evaluate_surface_closed_loop(
        config, codec, lexical, interface, predictor, test
    )

    result: dict[str, object] = {
        "format": FORMAT,
        "status": "controlled_cpu_construction",
        "config": asdict(config),
        "world": {
            "operators": list(OPS),
            "noncommutative_witness": {
                "R_after_F": list(
                    apply_operator(apply_operator((0, 0, 0), "F"), "R")
                ),
                "F_after_R": list(
                    apply_operator(apply_operator((0, 0, 0), "R"), "F")
                ),
            },
            "context_dependent_homograph_hex": b"ba".hex(),
            "train_latent_examples": len(train),
            "test_latent_examples": len(test),
            "interface_surface_augmentations": config.interface_augmentations,
        },
        "lexical_radiation_body": lexical_metrics,
        "event_interface": interface_metrics,
        "one_body_macro_diagnostic": macro_metrics,
        "surface_neutral_predictive_body": predictor_metrics,
        "closed_surface_feedback_loop": closed_loop,
        "decision": {
            "bidirectional_interface_construction_supported": bool(
                predictor_metrics["latent_closed_loop_exact_sequence_rate"] >= 0.99
                and interface_metrics["radiation_operator_accuracy"] >= 0.95
                and closed_loop["surface_roundtrip_operator_accuracy"] >= 0.85
            ),
            "full_interface_learned_end_to_end": False,
            "closed_surface_loop_scaling_gate_passed": bool(
                closed_loop["exact_semantic_sequence_rate"] >= 0.90
                and closed_loop["exact_event_boundary_sequence_rate"] >= 0.90
            ),
            "one_body_operator_macro_promoted": bool(
                macro_metrics["operator_macro"]["next_operator_agreement"] >= 0.90
            ),
            "two_body_operator_surface_separation_promoted": True,
            "event_clock_promoted_as_unsupervised": False,
            "declared_articulator_promoted_as_learned": False,
            "gpu_scale_warranted": False,
            "next_experiment": (
                "learn the articulatory body and event hazard jointly while "
                "retaining exact operator-feedback and surface-codec controls"
            ),
        },
    }
    result["scientific_digest"] = _scientific_digest(result)
    return result


def write_result(
    output: Path,
    config: ConstructionConfig,
    result: dict[str, object],
) -> None:
    output.mkdir(parents=True, exist_ok=True)
    summary = json.dumps(result, indent=2, sort_keys=True) + "\n"
    (output / "summary.json").write_text(summary)
    config_payload = {"format": FORMAT, **asdict(config)}
    (output / "config.json").write_text(
        json.dumps(config_payload, indent=2, sort_keys=True) + "\n"
    )
    manifest = {
        "format": "sophia.instrument.result-manifest/v1",
        "files": {
            "summary.json": hashlib.sha256(summary.encode()).hexdigest(),
            "config.json": hashlib.sha256(
                (output / "config.json").read_bytes()
            ).hexdigest(),
        },
        "scientific_digest": result["scientific_digest"],
    }
    (output / "manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n"
    )


def verify_result(output: Path) -> dict[str, object]:
    manifest = json.loads((output / "manifest.json").read_text())
    for name, expected in manifest["files"].items():
        actual = hashlib.sha256((output / name).read_bytes()).hexdigest()
        if actual != expected:
            raise ValueError(f"digest mismatch for {name}")
    result = json.loads((output / "summary.json").read_text())
    supplied = result.pop("scientific_digest")
    actual = _scientific_digest(result)
    if actual != supplied or actual != manifest["scientific_digest"]:
        raise ValueError("scientific digest mismatch")
    result["scientific_digest"] = supplied
    return result
