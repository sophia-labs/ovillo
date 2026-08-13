"""Validated authored control paths for persistent physical bodies.

This module mirrors ``mus-trajectory`` without importing the Rust runtime.  It
owns the direct path layer, not inverse gesture synthesis:

``authored knots -> validated path -> deterministic control frames -> receipt``.

The binary semantic digest is deliberately specified in float bits rather than
JSON spelling, so Python and Rust assign the same identity to the same program.
The GARGANTA adapter at the bottom applies these paths to the existing
speaker-static anatomy map while keeping articulation and source controls
separate.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from hashlib import sha256
import json
import math
import struct
from typing import Any, Iterable, Mapping, Sequence


TRAJECTORY_SCHEMA_VERSION = "trajectory-program-v1"
TRAJECTORY_COMPILER_VERSION = "mus-trajectory/0.1.0"


class TrajectoryError(ValueError):
    """Typed trajectory refusal with a stable machine-readable code."""

    def __init__(self, code: str, message: str, coordinate: str | None = None):
        self.code = code
        self.coordinate = coordinate
        self.message = message
        prefix = f"{code} ({coordinate})" if coordinate is not None else code
        super().__init__(f"{prefix}: {message}")


class Interpolation(str, Enum):
    STEP = "step"
    LINEAR = "linear"
    SMOOTHSTEP = "smoothstep"
    SMOOTHERSTEP = "smootherstep"
    COSINE = "cosine"

    @classmethod
    def from_name(cls, name: str) -> "Interpolation":
        aliases = {
            "step": cls.STEP,
            "linear": cls.LINEAR,
            "lin": cls.LINEAR,
            "smoothstep": cls.SMOOTHSTEP,
            "smooth": cls.SMOOTHSTEP,
            "smootherstep": cls.SMOOTHERSTEP,
            "smoother": cls.SMOOTHERSTEP,
            "cosine": cls.COSINE,
            "cos": cls.COSINE,
        }
        try:
            return aliases[name]
        except KeyError as exc:
            raise TrajectoryError(
                "trajectory-invalid-interpolation",
                f"unknown interpolation {name!r}",
            ) from exc


class DisposalPolicy(str, Enum):
    RADIATE = "radiate"
    DISSIPATE = "dissipate"
    RETIRE = "retire"
    REJECT = "reject"


@dataclass(frozen=True)
class StateSemantics:
    kind: str = "fixed_capacity"
    disposition: DisposalPolicy | None = None

    @classmethod
    def from_dict(cls, data: Mapping[str, Any] | None) -> "StateSemantics":
        if not data:
            return cls()
        kind = str(data.get("kind", "fixed_capacity"))
        disposition = data.get("disposition")
        if kind == "fixed_capacity":
            if disposition is not None:
                raise TrajectoryError(
                    "trajectory-invalid-contract",
                    "fixed_capacity state semantics cannot carry a disposition",
                )
            return cls(kind=kind)
        if kind == "topology_edit":
            if disposition is None:
                raise TrajectoryError(
                    "trajectory-invalid-contract",
                    "topology_edit requires a disposal disposition",
                )
            try:
                policy = DisposalPolicy(str(disposition))
            except ValueError as exc:
                raise TrajectoryError(
                    "trajectory-invalid-contract",
                    f"unknown disposal policy {disposition!r}",
                ) from exc
            return cls(kind=kind, disposition=policy)
        raise TrajectoryError(
            "trajectory-invalid-contract",
            f"unknown state semantics {kind!r}",
        )

    def to_dict(self) -> dict[str, Any]:
        result: dict[str, Any] = {"kind": self.kind}
        if self.disposition is not None:
            result["disposition"] = self.disposition.value
        return result


@dataclass(frozen=True)
class CoordinateContract:
    unit: str
    minimum: float
    maximum: float
    max_rate_per_second: float | None = None
    max_acceleration_per_second2: float | None = None
    metric_weight: float | None = None
    state_semantics: StateSemantics = field(default_factory=StateSemantics)

    @classmethod
    def fixed(cls, unit: str, minimum: float, maximum: float) -> "CoordinateContract":
        return cls(unit=unit, minimum=minimum, maximum=maximum)

    @classmethod
    def from_dict(cls, data: Mapping[str, Any]) -> "CoordinateContract":
        return cls(
            unit=str(data["unit"]),
            minimum=float(data["minimum"]),
            maximum=float(data["maximum"]),
            max_rate_per_second=_optional_float(data.get("max_rate_per_second")),
            max_acceleration_per_second2=_optional_float(
                data.get("max_acceleration_per_second2")
            ),
            metric_weight=_optional_float(data.get("metric_weight")),
            state_semantics=StateSemantics.from_dict(data.get("state_semantics")),
        )

    def to_dict(self) -> dict[str, Any]:
        result: dict[str, Any] = {
            "unit": self.unit,
            "minimum": self.minimum,
            "maximum": self.maximum,
            "state_semantics": self.state_semantics.to_dict(),
        }
        if self.max_rate_per_second is not None:
            result["max_rate_per_second"] = self.max_rate_per_second
        if self.max_acceleration_per_second2 is not None:
            result["max_acceleration_per_second2"] = (
                self.max_acceleration_per_second2
            )
        if self.metric_weight is not None:
            result["metric_weight"] = self.metric_weight
        return result


@dataclass(frozen=True)
class TrajectoryKnot:
    at_sample: int
    value: float

    @classmethod
    def from_dict(cls, data: Mapping[str, Any]) -> "TrajectoryKnot":
        return cls(at_sample=int(data["at_sample"]), value=float(data["value"]))

    def to_dict(self) -> dict[str, Any]:
        return {"at_sample": self.at_sample, "value": self.value}


@dataclass(frozen=True)
class TrajectoryTrack:
    coordinate: str
    interpolation: Interpolation
    contract: CoordinateContract
    knots: tuple[TrajectoryKnot, ...]

    @classmethod
    def from_dict(cls, data: Mapping[str, Any]) -> "TrajectoryTrack":
        try:
            interpolation = Interpolation(str(data["interpolation"]))
        except ValueError as exc:
            raise TrajectoryError(
                "trajectory-invalid-interpolation",
                f"unknown interpolation {data.get('interpolation')!r}",
                str(data.get("coordinate", "")),
            ) from exc
        return cls(
            coordinate=str(data["coordinate"]),
            interpolation=interpolation,
            contract=CoordinateContract.from_dict(data["contract"]),
            knots=tuple(TrajectoryKnot.from_dict(item) for item in data["knots"]),
        )

    def to_dict(self) -> dict[str, Any]:
        return {
            "coordinate": self.coordinate,
            "interpolation": self.interpolation.value,
            "contract": self.contract.to_dict(),
            "knots": [knot.to_dict() for knot in self.knots],
        }

    @property
    def changed(self) -> bool:
        return any(
            _f64_bits(left.value) != _f64_bits(right.value)
            for left, right in zip(self.knots[:-1], self.knots[1:], strict=True)
        )

    def value_at(self, sample: int, duration_samples: int) -> float:
        if sample < 0 or sample > duration_samples:
            raise TrajectoryError(
                "trajectory-sample-out-of-range",
                f"sample {sample} is outside [0, {duration_samples}]",
                self.coordinate,
            )
        for index, knot in enumerate(self.knots):
            if sample == knot.at_sample:
                return knot.value
            if sample < knot.at_sample:
                left = self.knots[index - 1]
                width = knot.at_sample - left.at_sample
                phase = (sample - left.at_sample) / width
                if self.interpolation is Interpolation.STEP:
                    shaped = 0.0
                elif self.interpolation is Interpolation.LINEAR:
                    shaped = phase
                elif self.interpolation is Interpolation.SMOOTHSTEP:
                    shaped = phase * phase * (3.0 - 2.0 * phase)
                elif self.interpolation is Interpolation.SMOOTHERSTEP:
                    shaped = phase**3 * (phase * (phase * 6.0 - 15.0) + 10.0)
                else:
                    shaped = 0.5 - 0.5 * math.cos(math.pi * phase)
                return left.value + (knot.value - left.value) * shaped
        return self.knots[-1].value


@dataclass(frozen=True)
class CoordinateValue:
    coordinate: str
    value: float

    def to_dict(self) -> dict[str, Any]:
        return {"coordinate": self.coordinate, "value": self.value}


@dataclass(frozen=True)
class TrajectoryFrame:
    at_sample: int
    sets: tuple[CoordinateValue, ...]

    def to_dict(self) -> dict[str, Any]:
        return {
            "at_sample": self.at_sample,
            "sets": [item.to_dict() for item in self.sets],
        }


@dataclass(frozen=True)
class TrackReceipt:
    coordinate: str
    interpolation: Interpolation
    start_value: float
    end_value: float
    max_rate_per_second: float | None
    max_acceleration_per_second2: float | None
    declared_metric_effort: float
    zero_order_hold_error_bound: float | None

    def to_dict(self) -> dict[str, Any]:
        return {
            "coordinate": self.coordinate,
            "interpolation": self.interpolation.value,
            "start_value": self.start_value,
            "end_value": self.end_value,
            "max_rate_per_second": self.max_rate_per_second,
            "max_acceleration_per_second2": self.max_acceleration_per_second2,
            "declared_metric_effort": self.declared_metric_effort,
            "zero_order_hold_error_bound": self.zero_order_hold_error_bound,
        }


@dataclass(frozen=True)
class TrajectoryReceipt:
    schema_version: str
    compiler_version: str
    program_digest: str
    frame_digest: str
    sample_rate_hz: int
    duration_samples: int
    control_stride_samples: int
    frame_count: int
    tracks: tuple[TrackReceipt, ...]
    claim_level: str = "authored-control-path"
    evidence_class: str = "validated-numerics"

    def to_dict(self) -> dict[str, Any]:
        return {
            "schema_version": self.schema_version,
            "compiler_version": self.compiler_version,
            "program_digest": self.program_digest,
            "frame_digest": self.frame_digest,
            "sample_rate_hz": self.sample_rate_hz,
            "duration_samples": self.duration_samples,
            "control_stride_samples": self.control_stride_samples,
            "frame_count": self.frame_count,
            "tracks": [track.to_dict() for track in self.tracks],
            "claim_level": self.claim_level,
            "evidence_class": self.evidence_class,
        }


@dataclass(frozen=True)
class CompiledTrajectory:
    frames: tuple[TrajectoryFrame, ...]
    receipt: TrajectoryReceipt

    def to_dict(self) -> dict[str, Any]:
        return {
            "frames": [frame.to_dict() for frame in self.frames],
            "receipt": self.receipt.to_dict(),
        }


@dataclass(frozen=True)
class TrajectoryProgram:
    schema_version: str
    sample_rate_hz: int
    duration_samples: int
    control_stride_samples: int
    tracks: tuple[TrajectoryTrack, ...]
    metadata: Mapping[str, str] = field(default_factory=dict)

    @classmethod
    def from_dict(cls, data: Mapping[str, Any]) -> "TrajectoryProgram":
        program = cls(
            schema_version=str(data["schema_version"]),
            sample_rate_hz=int(data["sample_rate_hz"]),
            duration_samples=int(data["duration_samples"]),
            control_stride_samples=int(data["control_stride_samples"]),
            metadata={str(key): str(value) for key, value in data.get("metadata", {}).items()},
            tracks=tuple(TrajectoryTrack.from_dict(item) for item in data["tracks"]),
        )
        program.validate()
        return program

    @classmethod
    def from_json(cls, source: str) -> "TrajectoryProgram":
        try:
            data = json.loads(source)
        except json.JSONDecodeError as exc:
            raise TrajectoryError(
                "trajectory-serialization",
                f"could not decode trajectory JSON: {exc}",
            ) from exc
        if not isinstance(data, dict):
            raise TrajectoryError(
                "trajectory-serialization", "trajectory JSON must be an object"
            )
        return cls.from_dict(data)

    def to_dict(self) -> dict[str, Any]:
        return {
            "schema_version": self.schema_version,
            "sample_rate_hz": self.sample_rate_hz,
            "duration_samples": self.duration_samples,
            "control_stride_samples": self.control_stride_samples,
            "metadata": dict(sorted(self.metadata.items())),
            "tracks": [track.to_dict() for track in self.tracks],
        }

    def to_json(self) -> str:
        return json.dumps(self.to_dict(), indent=2, sort_keys=True) + "\n"

    def validate(self) -> None:
        if self.schema_version != TRAJECTORY_SCHEMA_VERSION:
            raise TrajectoryError(
                "trajectory-unsupported-schema",
                f"expected {TRAJECTORY_SCHEMA_VERSION}, found {self.schema_version}",
            )
        if self.sample_rate_hz <= 0:
            raise TrajectoryError(
                "trajectory-invalid-sample-rate", "sample_rate_hz must be positive"
            )
        if self.duration_samples <= 0:
            raise TrajectoryError(
                "trajectory-invalid-duration", "duration_samples must be positive"
            )
        if self.control_stride_samples <= 0:
            raise TrajectoryError(
                "trajectory-invalid-control-stride",
                "control_stride_samples must be positive",
            )
        coordinates: set[str] = set()
        for track in self.tracks:
            self._validate_track(track)
            if track.coordinate in coordinates:
                raise TrajectoryError(
                    "trajectory-duplicate-coordinate",
                    "each coordinate may appear only once",
                    track.coordinate,
                )
            coordinates.add(track.coordinate)

    def _validate_track(self, track: TrajectoryTrack) -> None:
        coordinate = track.coordinate
        contract = track.contract
        if not coordinate.strip() or coordinate.strip() != coordinate:
            raise TrajectoryError(
                "trajectory-empty-coordinate",
                "coordinate names must be non-empty and have no surrounding whitespace",
                coordinate,
            )
        if (
            not contract.unit.strip()
            or not math.isfinite(contract.minimum)
            or not math.isfinite(contract.maximum)
            or contract.minimum > contract.maximum
        ):
            raise TrajectoryError(
                "trajectory-invalid-contract",
                "unit must be non-empty and finite minimum must not exceed maximum",
                coordinate,
            )
        for name, value, positive in (
            ("max_rate_per_second", contract.max_rate_per_second, True),
            (
                "max_acceleration_per_second2",
                contract.max_acceleration_per_second2,
                True,
            ),
            ("metric_weight", contract.metric_weight, False),
        ):
            if value is not None:
                bad_sign = value <= 0.0 if positive else value < 0.0
                if not math.isfinite(value) or bad_sign:
                    raise TrajectoryError(
                        "trajectory-invalid-contract",
                        f"{name} is outside its finite admissible range",
                        coordinate,
                    )
        if len(track.knots) < 2:
            raise TrajectoryError(
                "trajectory-too-few-knots",
                "a track needs at least two knots",
                coordinate,
            )
        if (
            track.knots[0].at_sample != 0
            or track.knots[-1].at_sample != self.duration_samples
        ):
            raise TrajectoryError(
                "trajectory-endpoint-mismatch",
                f"first knot must be at 0 and last at {self.duration_samples}",
                coordinate,
            )
        for left, right in zip(track.knots[:-1], track.knots[1:], strict=True):
            if left.at_sample >= right.at_sample:
                raise TrajectoryError(
                    "trajectory-nonincreasing-knots",
                    "knot samples must be strictly increasing",
                    coordinate,
                )
        for knot in track.knots:
            if not math.isfinite(knot.value):
                raise TrajectoryError(
                    "trajectory-nonfinite-value",
                    f"knot at sample {knot.at_sample} is not finite",
                    coordinate,
                )
            if not contract.minimum <= knot.value <= contract.maximum:
                raise TrajectoryError(
                    "trajectory-value-out-of-range",
                    f"{knot.value} is outside [{contract.minimum}, {contract.maximum}] at sample {knot.at_sample}",
                    coordinate,
                )

        if contract.state_semantics.kind == "fixed_capacity":
            if contract.state_semantics.disposition is not None:
                raise TrajectoryError(
                    "trajectory-invalid-contract",
                    "fixed_capacity state semantics cannot carry a disposition",
                    coordinate,
                )
            if track.changed and track.interpolation is Interpolation.STEP:
                raise TrajectoryError(
                    "trajectory-step-requires-topology",
                    "a discontinuous authored change needs explicit topology-edit semantics",
                    coordinate,
                )
        elif contract.state_semantics.kind == "topology_edit":
            if contract.state_semantics.disposition is None:
                raise TrajectoryError(
                    "trajectory-invalid-contract",
                    "topology_edit requires a disposal disposition",
                    coordinate,
                )
            if track.interpolation is not Interpolation.STEP:
                raise TrajectoryError(
                    "trajectory-topology-requires-step",
                    "topology edits must occur at declared knot boundaries",
                    coordinate,
                )
            if (contract.metric_weight or 0.0) > 0.0:
                raise TrajectoryError(
                    "trajectory-certificate-unavailable",
                    "quadratic path effort is not defined across a topology edit",
                    coordinate,
                )
        else:
            raise TrajectoryError(
                "trajectory-invalid-contract",
                f"unknown state semantics {contract.state_semantics.kind!r}",
                coordinate,
            )

        rate, acceleration, _effort = self._track_kinematics(track)
        if contract.max_rate_per_second is not None:
            if rate is None and track.changed:
                raise TrajectoryError(
                    "trajectory-certificate-unavailable",
                    "a finite rate certificate is unavailable for this path",
                    coordinate,
                )
            if rate is not None and rate > contract.max_rate_per_second * (1.0 + 1e-12):
                raise TrajectoryError(
                    "trajectory-rate-limit-exceeded",
                    f"measured rate {rate} exceeds declared limit {contract.max_rate_per_second}",
                    coordinate,
                )
        if contract.max_acceleration_per_second2 is not None:
            if acceleration is None and track.changed:
                raise TrajectoryError(
                    "trajectory-certificate-unavailable",
                    "piecewise-linear or discontinuous corners have no finite acceleration certificate",
                    coordinate,
                )
            if (
                acceleration is not None
                and acceleration
                > contract.max_acceleration_per_second2 * (1.0 + 1e-12)
            ):
                raise TrajectoryError(
                    "trajectory-acceleration-limit-exceeded",
                    f"measured acceleration {acceleration} exceeds declared limit {contract.max_acceleration_per_second2}",
                    coordinate,
                )

    def _track_kinematics(
        self, track: TrajectoryTrack
    ) -> tuple[float | None, float | None, float]:
        max_rate = 0.0
        max_acceleration = 0.0
        acceleration_available = True
        effort = 0.0
        weight = track.contract.metric_weight or 0.0
        for left, right in zip(track.knots[:-1], track.knots[1:], strict=True):
            delta = right.value - left.value
            if delta == 0.0:
                continue
            seconds = (right.at_sample - left.at_sample) / self.sample_rate_hz
            rate_factor, acceleration_factor, effort_factor = _interpolation_factors(
                track.interpolation
            )
            if rate_factor is not None:
                max_rate = max(max_rate, rate_factor * abs(delta) / seconds)
            if acceleration_factor is not None:
                max_acceleration = max(
                    max_acceleration,
                    acceleration_factor * abs(delta) / seconds**2,
                )
            else:
                acceleration_available = False
            if weight > 0.0 and effort_factor is not None:
                effort += weight * effort_factor * delta**2 / seconds
        rate: float | None
        if track.changed and track.interpolation is Interpolation.STEP:
            rate = None
        else:
            rate = max_rate
        acceleration = max_acceleration if acceleration_available else None
        return rate, acceleration, effort

    def semantic_bytes(self) -> bytes:
        self.validate()
        data = bytearray()
        _put_string(data, "mus-trajectory-semantic-v1")
        _put_string(data, self.schema_version)
        data.extend(struct.pack(">I", self.sample_rate_hz))
        data.extend(struct.pack(">Q", self.duration_samples))
        data.extend(struct.pack(">I", self.control_stride_samples))
        _put_length(data, len(self.metadata))
        for key, value in sorted(self.metadata.items()):
            _put_string(data, key)
            _put_string(data, value)
        tracks = sorted(self.tracks, key=lambda track: track.coordinate)
        _put_length(data, len(tracks))
        for track in tracks:
            _put_string(data, track.coordinate)
            _put_string(data, track.interpolation.value)
            _put_string(data, track.contract.unit)
            _put_f64(data, track.contract.minimum)
            _put_f64(data, track.contract.maximum)
            _put_optional_f64(data, track.contract.max_rate_per_second)
            _put_optional_f64(data, track.contract.max_acceleration_per_second2)
            _put_optional_f64(data, track.contract.metric_weight)
            _put_string(data, track.contract.state_semantics.kind)
            if track.contract.state_semantics.disposition is not None:
                _put_string(data, track.contract.state_semantics.disposition.value)
            _put_length(data, len(track.knots))
            for knot in track.knots:
                data.extend(struct.pack(">Q", knot.at_sample))
                _put_f64(data, knot.value)
        return bytes(data)

    def semantic_digest(self) -> str:
        return "sha256:" + sha256(self.semantic_bytes()).hexdigest()

    def compile(self) -> CompiledTrajectory:
        self.validate()
        sample_points = {0, self.duration_samples}
        for track in self.tracks:
            sample_points.update(knot.at_sample for knot in track.knots)
        sample_points.update(
            range(
                self.control_stride_samples,
                self.duration_samples,
                self.control_stride_samples,
            )
        )
        tracks = sorted(self.tracks, key=lambda track: track.coordinate)
        frames: list[TrajectoryFrame] = []
        last_bits: dict[str, int] = {}
        for sample in sorted(sample_points):
            sets: list[CoordinateValue] = []
            for track in tracks:
                value = track.value_at(sample, self.duration_samples)
                bits = _f64_bits(value)
                if last_bits.get(track.coordinate) != bits:
                    sets.append(CoordinateValue(track.coordinate, value))
                    last_bits[track.coordinate] = bits
            if sets:
                frames.append(TrajectoryFrame(sample, tuple(sets)))

        stride_seconds = self.control_stride_samples / self.sample_rate_hz
        receipts: list[TrackReceipt] = []
        for track in tracks:
            rate, acceleration, effort = self._track_kinematics(track)
            hold_error = (
                0.0
                if track.interpolation is Interpolation.STEP
                else None if rate is None else rate * stride_seconds
            )
            receipts.append(
                TrackReceipt(
                    coordinate=track.coordinate,
                    interpolation=track.interpolation,
                    start_value=track.knots[0].value,
                    end_value=track.knots[-1].value,
                    max_rate_per_second=rate,
                    max_acceleration_per_second2=acceleration,
                    declared_metric_effort=effort,
                    zero_order_hold_error_bound=hold_error,
                )
            )
        frame_digest = "sha256:" + sha256(_frame_semantic_bytes(frames)).hexdigest()
        receipt = TrajectoryReceipt(
            schema_version=TRAJECTORY_SCHEMA_VERSION,
            compiler_version=TRAJECTORY_COMPILER_VERSION,
            program_digest=self.semantic_digest(),
            frame_digest=frame_digest,
            sample_rate_hz=self.sample_rate_hz,
            duration_samples=self.duration_samples,
            control_stride_samples=self.control_stride_samples,
            frame_count=len(frames),
            tracks=tuple(receipts),
        )
        return CompiledTrajectory(tuple(frames), receipt)


def parse_arrow_values(source: str) -> tuple[float, ...]:
    parts = [part.strip() for part in source.split("->")]
    if len(parts) < 2 or any(not part for part in parts):
        raise TrajectoryError(
            "trajectory-invalid-arrow-syntax",
            f"expected at least two finite values separated by '->': {source}",
        )
    values: list[float] = []
    for part in parts:
        try:
            value = float(part)
        except ValueError as exc:
            raise TrajectoryError(
                "trajectory-invalid-arrow-syntax",
                f"{part!r} is not a floating-point value",
            ) from exc
        if not math.isfinite(value):
            raise TrajectoryError(
                "trajectory-nonfinite-value", f"{part!r} is not finite"
            )
        values.append(value)
    return tuple(values)


def equally_spaced_track(
    coordinate: str,
    values: Sequence[float],
    duration_samples: int,
    interpolation: Interpolation,
    contract: CoordinateContract,
) -> TrajectoryTrack:
    if len(values) < 2:
        raise TrajectoryError(
            "trajectory-too-few-knots",
            "an arrow trajectory needs at least two values",
            coordinate,
        )
    if duration_samples < len(values) - 1:
        raise TrajectoryError(
            "trajectory-duration-too-short",
            "duration is too short to place strictly increasing knots",
            coordinate,
        )
    denominator = len(values) - 1
    knots = tuple(
        TrajectoryKnot(
            at_sample=(duration_samples * index) // denominator,
            value=float(value),
        )
        for index, value in enumerate(values)
    )
    return TrajectoryTrack(coordinate, interpolation, contract, knots)


STRING_INLINE_COORDINATES = {
    "couple": "string.couple",
    "chirality": "string.chirality",
    "orbit": "string.orbit_hz",
    "orbit_depth": "string.orbit_depth",
    "curvature": "string.curvature",
    "body": "string.body",
    "buzz": "string.buzz",
    "sing": "string.sing",
    "bow": "string.bow",
    "bow_speed": "string.bow_speed",
    "sus": "string.sus",
    "damp": "string.damp",
    "breath": "string.breath",
    "pitch_ratio": "string.pitch_ratio",
}
STRING_ENGINE_NAMES = {
    coordinate: key for key, coordinate in STRING_INLINE_COORDINATES.items()
}
STRING_COORDINATE_OBLIGATIONS = {
    "string.pitch_ratio": (
        "integer-delay-state-transport",
        "pitch-sideband-certificate",
    ),
    "string.bow": ("bow-contact-supply-law",),
    "string.bow_speed": ("bow-contact-supply-law",),
    "string.breath": ("stochastic-source-ledger",),
    "string.sing": ("external-source-work-ledger",),
}


@dataclass(frozen=True)
class StringControlFrame:
    at_sample: int
    sets: tuple[tuple[str, float], ...]

    def to_dict(self) -> dict[str, Any]:
        return {
            "at_sample": self.at_sample,
            "sets": [
                {"target": target, "value": value} for target, value in self.sets
            ],
        }


@dataclass(frozen=True)
class StringTrajectoryLowering:
    frequencies_hz: tuple[float, ...]
    native_gliss_ratio: float
    control_frames: tuple[StringControlFrame, ...]
    trajectory_receipt: TrajectoryReceipt
    native_pitch_transport: str
    pitch_start_ratio: float
    pitch_end_ratio: float
    unresolved_obligations: tuple[str, ...]
    strict_transport: bool

    def to_dict(self) -> dict[str, Any]:
        return {
            "frequencies_hz": list(self.frequencies_hz),
            "native_gliss_ratio": self.native_gliss_ratio,
            "control_frames": [frame.to_dict() for frame in self.control_frames],
            "trajectory_receipt": self.trajectory_receipt.to_dict(),
            "native_pitch_transport": self.native_pitch_transport,
            "pitch_start_ratio": self.pitch_start_ratio,
            "pitch_end_ratio": self.pitch_end_ratio,
            "unresolved_obligations": list(self.unresolved_obligations),
            "strict_transport": self.strict_transport,
        }


def string_coordinate_contract(coordinate: str) -> CoordinateContract:
    specs = {
        "string.couple": ("radian", 0.0, 0.45, 1.0),
        "string.chirality": ("ratio", -1.0, 1.0, 0.5),
        "string.orbit_hz": ("hertz", 0.0, 20.0, 0.05),
        "string.orbit_depth": ("ratio", 0.0, 1.0, 0.5),
        "string.curvature": ("ratio", 0.0, 1.0, 0.5),
        "string.body": ("ratio", 0.0, 1.0, 1.0),
        "string.buzz": ("ratio", 0.0, 1.0, 0.5),
        "string.sing": ("ratio", 0.0, 1.0, 0.0),
        "string.bow": ("ratio", 0.0, 1.0, 0.0),
        "string.bow_speed": ("ratio", 0.0, 1.0, 0.0),
        "string.sus": ("second", 0.05, 20.0, 0.1),
        "string.damp": ("ratio", 0.0, 0.95, 0.5),
        "string.breath": ("ratio", 0.0, 1.0, 0.0),
        "string.pitch_ratio": ("ratio", 0.8, 1.2, 1.0),
    }
    try:
        unit, minimum, maximum, weight = specs[coordinate]
    except KeyError as exc:
        raise TrajectoryError(
            "string-trajectory-unsupported-coordinate",
            "the string adapter has no contract for this coordinate",
            coordinate,
        ) from exc
    return CoordinateContract(
        unit=unit,
        minimum=minimum,
        maximum=maximum,
        metric_weight=weight,
        state_semantics=StateSemantics(),
    )


def program_from_inline_string_params(
    params: Mapping[str, str],
    duration_seconds: float,
    *,
    sample_rate_hz: int = 48_000,
) -> tuple[dict[str, str], TrajectoryProgram | None]:
    """Extract registered physical arrows while leaving MUS-A arrows alone."""

    if not math.isfinite(duration_seconds) or duration_seconds <= 0.0:
        raise TrajectoryError(
            "string-trajectory-invalid-duration",
            f"duration {duration_seconds} must be finite and positive",
        )
    duration_samples = round(duration_seconds * sample_rate_hz)
    if duration_samples <= 0:
        raise TrajectoryError(
            "string-trajectory-invalid-duration", "duration rounds to zero samples"
        )
    candidates = sorted(
        key
        for key, value in params.items()
        if "->" in value and key in STRING_INLINE_COORDINATES
    )
    if not candidates:
        return dict(params), None

    control_rate_hz = _parse_positive_float(
        "traj_hz", params.get("traj_hz", "200")
    )
    if control_rate_hz > sample_rate_hz:
        raise TrajectoryError(
            "string-trajectory-invalid-inline-directive",
            f"traj_hz={control_rate_hz} exceeds the {sample_rate_hz} Hz audio rate",
        )
    stride = max(1, round(sample_rate_hz / control_rate_hz))
    global_curve = Interpolation.from_name(params.get("traj_curve", "smoothstep"))
    strict_transport = _parse_bool(
        "traj_strict", params.get("traj_strict", "0")
    )

    static_params = dict(params)
    tracks: list[TrajectoryTrack] = []
    for key in candidates:
        coordinate = STRING_INLINE_COORDINATES[key]
        curve_key = f"{key}_curve"
        if curve_key in params:
            interpolation = Interpolation.from_name(params[curve_key])
        elif coordinate == "string.pitch_ratio":
            interpolation = Interpolation.LINEAR
        else:
            interpolation = global_curve
        track = _parse_inline_track(
            coordinate=coordinate,
            source_key=key,
            expression=params[key],
            duration_samples=duration_samples,
            interpolation=interpolation,
            contract=string_coordinate_contract(coordinate),
        )
        tracks.append(track)
        static_params[key] = str(track.knots[0].value)
        static_params.pop(curve_key, None)
    static_params.pop("traj_curve", None)
    static_params.pop("traj_hz", None)
    static_params.pop("traj_strict", None)

    program = TrajectoryProgram(
        schema_version=TRAJECTORY_SCHEMA_VERSION,
        sample_rate_hz=sample_rate_hz,
        duration_samples=duration_samples,
        control_stride_samples=stride,
        metadata={
            "surface": "mus-inline-string-v1",
            "strict_transport": str(strict_transport).lower(),
        },
        tracks=tuple(tracks),
    )
    program.validate()
    return static_params, program


def lower_string_trajectory(
    program: TrajectoryProgram,
    frequencies_hz: Sequence[float],
    duration_seconds: float,
    *,
    score_gliss_ratio: float = 1.0,
    engine_sample_rate_hz: int = 48_000,
) -> StringTrajectoryLowering:
    """Reference lowering matching ``mus_dsp::trajectory``."""

    program.validate()
    if program.sample_rate_hz != engine_sample_rate_hz:
        raise TrajectoryError(
            "string-trajectory-sample-rate-mismatch",
            f"program is {program.sample_rate_hz} Hz but engine is {engine_sample_rate_hz} Hz",
        )
    expected_duration = round(duration_seconds * engine_sample_rate_hz)
    if program.duration_samples != expected_duration:
        raise TrajectoryError(
            "string-trajectory-duration-mismatch",
            f"program duration {program.duration_samples} does not match {expected_duration}",
        )
    if not math.isfinite(score_gliss_ratio) or score_gliss_ratio <= 0.0:
        raise TrajectoryError(
            "string-trajectory-pitch-conflict",
            f"score gliss ratio {score_gliss_ratio} is not positive and finite",
        )

    obligations = tuple(
        sorted(
            {
                obligation
                for track in program.tracks
                for obligation in STRING_COORDINATE_OBLIGATIONS.get(
                    track.coordinate, ()
                )
            }
        )
    )
    strict = program.metadata.get("strict_transport") in {"true", "1"}
    if strict and obligations:
        raise TrajectoryError(
            "string-trajectory-transport-uncertified",
            "strict transport requested but obligations remain: "
            + ", ".join(obligations),
        )

    for track in program.tracks:
        actual = string_coordinate_contract(track.coordinate)
        for knot in track.knots:
            if not actual.minimum <= knot.value <= actual.maximum:
                raise TrajectoryError(
                    "string-trajectory-engine-domain-mismatch",
                    f"{knot.value} is outside [{actual.minimum}, {actual.maximum}]",
                    track.coordinate,
                )

    pitch_track = next(
        (
            track
            for track in program.tracks
            if track.coordinate == "string.pitch_ratio"
        ),
        None,
    )
    if pitch_track is not None:
        if (
            pitch_track.interpolation is not Interpolation.LINEAR
            or len(pitch_track.knots) != 2
        ):
            raise TrajectoryError(
                "string-trajectory-pitch-path-unsupported",
                "v1 accepts exactly two linear full-event pitch knots",
                pitch_track.coordinate,
            )
        if abs(score_gliss_ratio - 1.0) > 1e-12:
            raise TrajectoryError(
                "string-trajectory-pitch-conflict",
                "score gliss and pitch trajectory cannot both own retuning",
                pitch_track.coordinate,
            )
        pitch_start = pitch_track.knots[0].value
        pitch_end = pitch_track.knots[1].value
        native_gliss = pitch_end / pitch_start
        pitch_mode = "persistent-linear-f0-to-f1"
    else:
        pitch_start = 1.0
        pitch_end = score_gliss_ratio
        native_gliss = score_gliss_ratio
        pitch_mode = "score-gliss-or-static"

    compiled = program.compile()
    control_frames: list[StringControlFrame] = []
    for frame in compiled.frames:
        sets = tuple(
            (STRING_ENGINE_NAMES[item.coordinate], item.value)
            for item in frame.sets
            if item.coordinate != "string.pitch_ratio"
        )
        if sets:
            control_frames.append(StringControlFrame(frame.at_sample, sets))
    return StringTrajectoryLowering(
        frequencies_hz=tuple(float(value) * pitch_start for value in frequencies_hz),
        native_gliss_ratio=native_gliss,
        control_frames=tuple(control_frames),
        trajectory_receipt=compiled.receipt,
        native_pitch_transport=pitch_mode,
        pitch_start_ratio=pitch_start,
        pitch_end_ratio=pitch_end,
        unresolved_obligations=obligations,
        strict_transport=strict,
    )


def _parse_inline_track(
    *,
    coordinate: str,
    source_key: str,
    expression: str,
    duration_samples: int,
    interpolation: Interpolation,
    contract: CoordinateContract,
) -> TrajectoryTrack:
    terms = [term.strip() for term in expression.split("->")]
    if len(terms) < 2 or any(not term for term in terms):
        raise TrajectoryError(
            "string-trajectory-invalid-inline-directive",
            f"{source_key} needs at least two non-empty arrow terms",
            coordinate,
        )
    parsed: list[tuple[float, float | None]] = []
    for index, term in enumerate(terms):
        value_source, separator, time_source = term.rpartition("@")
        if not separator:
            value_source, time_source = term, ""
        try:
            value = float(value_source.strip())
        except ValueError as exc:
            raise TrajectoryError(
                "string-trajectory-invalid-inline-directive",
                f"{source_key}[{index}] value is not numeric",
                coordinate,
            ) from exc
        if not math.isfinite(value):
            raise TrajectoryError(
                "string-trajectory-invalid-inline-directive",
                f"{source_key}[{index}] value is not finite",
                coordinate,
            )
        if time_source:
            try:
                normalized = float(time_source.strip())
            except ValueError as exc:
                raise TrajectoryError(
                    "string-trajectory-invalid-inline-directive",
                    f"{source_key}[{index}] time is not numeric",
                    coordinate,
                ) from exc
            if not math.isfinite(normalized):
                raise TrajectoryError(
                    "string-trajectory-invalid-inline-directive",
                    f"{source_key}[{index}] time is not finite",
                    coordinate,
                )
            parsed.append((value, normalized))
        else:
            parsed.append((value, None))
    timed = sum(time is not None for _, time in parsed)
    if timed not in {0, len(parsed)}:
        raise TrajectoryError(
            "string-trajectory-invalid-inline-directive",
            f"{source_key}: every term must carry @time or none may",
            coordinate,
        )
    values = [value for value, _time in parsed]
    if timed == 0:
        return equally_spaced_track(
            coordinate,
            values,
            duration_samples,
            interpolation,
            contract,
        )
    knots: list[TrajectoryKnot] = []
    for value, normalized in parsed:
        assert normalized is not None
        if not 0.0 <= normalized <= 1.0:
            raise TrajectoryError(
                "string-trajectory-invalid-inline-directive",
                f"normalized time {normalized} is outside [0, 1]",
                coordinate,
            )
        knots.append(
            TrajectoryKnot(round(normalized * duration_samples), value)
        )
    return TrajectoryTrack(
        coordinate=coordinate,
        interpolation=interpolation,
        contract=contract,
        knots=tuple(knots),
    )


def _parse_positive_float(key: str, source: str) -> float:
    try:
        value = float(source)
    except ValueError as exc:
        raise TrajectoryError(
            "string-trajectory-invalid-inline-directive",
            f"{key}={source!r} is not numeric",
        ) from exc
    if not math.isfinite(value) or value <= 0.0:
        raise TrajectoryError(
            "string-trajectory-invalid-inline-directive",
            f"{key} must be finite and positive",
        )
    return value


def _parse_bool(key: str, source: str) -> bool:
    if source in {"1", "true", "yes", "on"}:
        return True
    if source in {"0", "false", "no", "off"}:
        return False
    raise TrajectoryError(
        "string-trajectory-invalid-inline-directive",
        f"{key}={source!r} is not a boolean",
    )


GARGANTA_CONTROL_COORDINATES = {
    "garganta.control.jaw_open": 0,
    "garganta.control.tongue_front": 1,
    "garganta.control.tongue_back": 2,
    "garganta.control.lip_round": 3,
    "garganta.control.apical_constriction": 4,
}
GARGANTA_SOURCE_COORDINATES = {
    "garganta.source.pitch_ratio",
    "garganta.source.lung_pressure",
    "garganta.source.velum_aperture",
}
GARGANTA_AREA_PREFIX = "garganta.tract_area_cm2."


@dataclass(frozen=True)
class GargantaTrajectoryState:
    at_sample: int
    articulation_controls: tuple[float, ...]
    pitch_ratio: float
    lung_pressure: float
    velum_aperture: float
    area_function_cm2: tuple[float, ...]


@dataclass(frozen=True)
class GargantaTrajectoryReceipt:
    trajectory: TrajectoryReceipt
    anatomy_static: bool
    articulation_coordinates: tuple[str, ...]
    source_coordinates: tuple[str, ...]
    area_override_indices: tuple[int, ...]
    area_cone_cm2: tuple[float, float]

    def to_dict(self) -> dict[str, Any]:
        return {
            "trajectory": self.trajectory.to_dict(),
            "anatomy_static": self.anatomy_static,
            "articulation_coordinates": list(self.articulation_coordinates),
            "source_coordinates": list(self.source_coordinates),
            "area_override_indices": list(self.area_override_indices),
            "area_cone_cm2": list(self.area_cone_cm2),
        }


class GargantaTrajectoryAdapter:
    """Apply a trajectory to GARGANTA while preserving static anatomy identity."""

    def __init__(self, program: TrajectoryProgram):
        program.validate()
        self.program = program
        self._tracks = {track.coordinate: track for track in program.tracks}
        area_indices: list[int] = []
        for track in program.tracks:
            coordinate = track.coordinate
            if coordinate in GARGANTA_CONTROL_COORDINATES:
                if track.contract.minimum < -1.0 or track.contract.maximum > 1.0:
                    raise TrajectoryError(
                        "garganta-control-contract-out-of-range",
                        "articulation controls must remain inside [-1, 1]",
                        coordinate,
                    )
                continue
            if coordinate in GARGANTA_SOURCE_COORDINATES:
                if coordinate == "garganta.source.pitch_ratio" and track.contract.minimum <= 0.0:
                    raise TrajectoryError(
                        "garganta-pitch-ratio-nonpositive",
                        "pitch ratios must be positive",
                        coordinate,
                    )
                continue
            if coordinate.startswith(GARGANTA_AREA_PREFIX):
                suffix = coordinate.removeprefix(GARGANTA_AREA_PREFIX)
                try:
                    index = int(suffix)
                except ValueError as exc:
                    raise TrajectoryError(
                        "garganta-invalid-area-coordinate",
                        "tract-area coordinates end in a non-negative section index",
                        coordinate,
                    ) from exc
                if index < 0:
                    raise TrajectoryError(
                        "garganta-invalid-area-coordinate",
                        "tract-area section index cannot be negative",
                        coordinate,
                    )
                if track.contract.minimum < 0.025 or track.contract.maximum > 18.0:
                    raise TrajectoryError(
                        "garganta-area-contract-out-of-cone",
                        "tract-area overrides must stay inside [0.025, 18.0] cm^2",
                        coordinate,
                    )
                area_indices.append(index)
                continue
            raise TrajectoryError(
                "garganta-unsupported-coordinate",
                "coordinate is not part of the GARGANTA trajectory contract",
                coordinate,
            )
        self.area_override_indices = tuple(sorted(area_indices))

    def state_at(
        self,
        anatomy: Any,
        sample: int,
        *,
        section_count: int = 24,
    ) -> GargantaTrajectoryState:
        from .garganta_anatomy import area_function_cm2

        controls = [0.0] * len(GARGANTA_CONTROL_COORDINATES)
        for coordinate, index in GARGANTA_CONTROL_COORDINATES.items():
            track = self._tracks.get(coordinate)
            if track is not None:
                controls[index] = track.value_at(sample, self.program.duration_samples)
        area = area_function_cm2(anatomy, controls, section_count=section_count)
        for index in self.area_override_indices:
            if index >= section_count:
                coordinate = f"{GARGANTA_AREA_PREFIX}{index}"
                raise TrajectoryError(
                    "garganta-area-index-out-of-range",
                    f"section {index} is outside a {section_count}-section tract",
                    coordinate,
                )
            area[index] = self._tracks[f"{GARGANTA_AREA_PREFIX}{index}"].value_at(
                sample, self.program.duration_samples
            )
        pitch_ratio = self._value_or("garganta.source.pitch_ratio", sample, 1.0)
        lung_pressure = self._value_or("garganta.source.lung_pressure", sample, 0.0)
        velum_aperture = self._value_or("garganta.source.velum_aperture", sample, 0.0)
        return GargantaTrajectoryState(
            at_sample=sample,
            articulation_controls=tuple(controls),
            pitch_ratio=pitch_ratio,
            lung_pressure=lung_pressure,
            velum_aperture=velum_aperture,
            area_function_cm2=tuple(float(value) for value in area),
        )

    def compile_states(
        self,
        anatomy: Any,
        *,
        section_count: int = 24,
    ) -> tuple[tuple[GargantaTrajectoryState, ...], GargantaTrajectoryReceipt]:
        compiled = self.program.compile()
        states = tuple(
            self.state_at(anatomy, frame.at_sample, section_count=section_count)
            for frame in compiled.frames
        )
        receipt = GargantaTrajectoryReceipt(
            trajectory=compiled.receipt,
            anatomy_static=True,
            articulation_coordinates=tuple(
                sorted(
                    coordinate
                    for coordinate in self._tracks
                    if coordinate in GARGANTA_CONTROL_COORDINATES
                )
            ),
            source_coordinates=tuple(
                sorted(
                    coordinate
                    for coordinate in self._tracks
                    if coordinate in GARGANTA_SOURCE_COORDINATES
                )
            ),
            area_override_indices=self.area_override_indices,
            area_cone_cm2=(0.025, 18.0),
        )
        return states, receipt

    def _value_or(self, coordinate: str, sample: int, default: float) -> float:
        track = self._tracks.get(coordinate)
        if track is None:
            return default
        return track.value_at(sample, self.program.duration_samples)


def _optional_float(value: Any) -> float | None:
    return None if value is None else float(value)


def _interpolation_factors(
    interpolation: Interpolation,
) -> tuple[float | None, float | None, float | None]:
    if interpolation is Interpolation.STEP:
        return None, None, None
    if interpolation is Interpolation.LINEAR:
        return 1.0, None, 1.0
    if interpolation is Interpolation.SMOOTHSTEP:
        return 1.5, 6.0, 6.0 / 5.0
    if interpolation is Interpolation.SMOOTHERSTEP:
        return 15.0 / 8.0, 10.0 * math.sqrt(3.0) / 3.0, 10.0 / 7.0
    return math.pi / 2.0, math.pi**2 / 2.0, math.pi**2 / 8.0


def _frame_semantic_bytes(frames: Iterable[TrajectoryFrame]) -> bytes:
    frames = tuple(frames)
    data = bytearray()
    _put_string(data, "mus-trajectory-frames-v1")
    _put_length(data, len(frames))
    for frame in frames:
        data.extend(struct.pack(">Q", frame.at_sample))
        sets = sorted(frame.sets, key=lambda item: item.coordinate)
        _put_length(data, len(sets))
        for item in sets:
            _put_string(data, item.coordinate)
            _put_f64(data, item.value)
    return bytes(data)


def _put_length(data: bytearray, length: int) -> None:
    data.extend(struct.pack(">Q", length))


def _put_string(data: bytearray, value: str) -> None:
    encoded = value.encode("utf-8")
    _put_length(data, len(encoded))
    data.extend(encoded)


def _put_f64(data: bytearray, value: float) -> None:
    data.extend(struct.pack(">d", value))


def _put_optional_f64(data: bytearray, value: float | None) -> None:
    if value is None:
        data.append(0)
    else:
        data.append(1)
        _put_f64(data, value)


def _f64_bits(value: float) -> int:
    return struct.unpack(">Q", struct.pack(">d", value))[0]


__all__ = [
    "TRAJECTORY_SCHEMA_VERSION",
    "TRAJECTORY_COMPILER_VERSION",
    "TrajectoryError",
    "Interpolation",
    "DisposalPolicy",
    "StateSemantics",
    "CoordinateContract",
    "TrajectoryKnot",
    "TrajectoryTrack",
    "CoordinateValue",
    "TrajectoryFrame",
    "TrackReceipt",
    "TrajectoryReceipt",
    "CompiledTrajectory",
    "TrajectoryProgram",
    "parse_arrow_values",
    "equally_spaced_track",
    "STRING_INLINE_COORDINATES",
    "STRING_ENGINE_NAMES",
    "STRING_COORDINATE_OBLIGATIONS",
    "StringControlFrame",
    "StringTrajectoryLowering",
    "string_coordinate_contract",
    "program_from_inline_string_params",
    "lower_string_trajectory",
    "GARGANTA_CONTROL_COORDINATES",
    "GARGANTA_SOURCE_COORDINATES",
    "GARGANTA_AREA_PREFIX",
    "GargantaTrajectoryState",
    "GargantaTrajectoryReceipt",
    "GargantaTrajectoryAdapter",
]
