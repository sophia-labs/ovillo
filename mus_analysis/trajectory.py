"""Language-neutral continuous control paths for persistent instrument bodies.

A trajectory is the direct, low-level artifact beneath both authored MUS
motion and inverse/learned gestures.  It names scalar body coordinates and
places values on an explicit sample clock.  Body adapters decide what target
names mean; this module owns interpolation, validation, refusal semantics, and
receipts.

The Python implementation mirrors ``mus-trajectory`` in the Rust workspace.
It is intentionally dependency-light so research tools can use the same
artifact without importing the audio engine.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from hashlib import sha256
import json
import math
from pathlib import Path
from typing import Any, Mapping, MutableSequence

FORMAT_V1 = "mus-trajectory/v1"
SCHEMA_V1 = "https://sophia.systems/schemas/mus-trajectory-program-v1.json"

_CURVES = {"step", "linear", "smoothstep", "smootherstep"}
_VALUE_SPACES = {"linear", "log2"}
_CONTINUITY = {
    "step": "discontinuous",
    "linear": "c0",
    "smoothstep": "c1",
    "smootherstep": "c2",
}
_PEAK_SLOPE = {
    "step": None,
    "linear": 1.0,
    "smoothstep": 1.5,
    "smootherstep": 1.875,
}
_CURVE_ALIASES = {
    "lin": "linear",
    "ease": "smoothstep",
    "ease_in_out": "smoothstep",
    "c2": "smootherstep",
}


class TrajectoryError(ValueError):
    """Typed refusal with a stable machine-readable code."""

    def __init__(self, code: str, message: str):
        super().__init__(message)
        self.code = code


def _refuse(code: str, message: str) -> None:
    raise TrajectoryError(code, message)


def normalize_curve(name: str) -> str:
    normalized = _CURVE_ALIASES.get(name, name)
    if normalized not in _CURVES:
        _refuse("trajectory-unknown-curve", f"unknown trajectory curve {name!r}")
    return normalized


def _curve_weight(curve: str, value: float) -> float:
    t = min(max(float(value), 0.0), 1.0)
    if curve == "step":
        return 0.0
    if curve == "linear":
        return t
    if curve == "smoothstep":
        return t * t * (3.0 - 2.0 * t)
    if curve == "smootherstep":
        return t * t * t * (t * (6.0 * t - 15.0) + 10.0)
    _refuse("trajectory-unknown-curve", f"unknown trajectory curve {curve!r}")
    raise AssertionError("unreachable")


def _encode(value_space: str, target: str, value: float) -> float:
    if value_space == "linear":
        return value
    if value_space == "log2":
        if value <= 0.0:
            _refuse(
                "trajectory-log-value-nonpositive",
                f"trajectory target {target!r} uses log2 space but value {value} is not positive",
            )
        return math.log2(value)
    _refuse(
        "trajectory-unknown-value-space",
        f"trajectory target {target!r} uses unknown value space {value_space!r}",
    )
    raise AssertionError("unreachable")


def _decode(value_space: str, value: float) -> float:
    if value_space == "linear":
        return value
    if value_space == "log2":
        return 2.0**value
    raise AssertionError(f"unvalidated value space {value_space!r}")


@dataclass(frozen=True)
class Knot:
    at_sample: int
    value: float

    @classmethod
    def from_dict(cls, raw: Mapping[str, Any]) -> "Knot":
        return cls(at_sample=int(raw["at_sample"]), value=float(raw["value"]))

    def to_dict(self) -> dict[str, Any]:
        return {"at_sample": self.at_sample, "value": self.value}


@dataclass(frozen=True)
class Lane:
    target: str
    unit: str
    curve: str = "linear"
    value_space: str = "linear"
    min_value: float | None = None
    max_value: float | None = None
    max_rate_per_second: float | None = None
    knots: tuple[Knot, ...] = field(default_factory=tuple)

    @classmethod
    def from_dict(cls, raw: Mapping[str, Any]) -> "Lane":
        return cls(
            target=str(raw["target"]),
            unit=str(raw["unit"]),
            curve=str(raw.get("curve", "linear")),
            value_space=str(raw.get("value_space", "linear")),
            min_value=(None if raw.get("min_value") is None else float(raw["min_value"])),
            max_value=(None if raw.get("max_value") is None else float(raw["max_value"])),
            max_rate_per_second=(
                None
                if raw.get("max_rate_per_second") is None
                else float(raw["max_rate_per_second"])
            ),
            knots=tuple(Knot.from_dict(knot) for knot in raw.get("knots", ())),
        )

    def to_dict(self) -> dict[str, Any]:
        raw: dict[str, Any] = {
            "target": self.target,
            "unit": self.unit,
            "curve": self.curve,
            "value_space": self.value_space,
        }
        if self.min_value is not None:
            raw["min_value"] = self.min_value
        if self.max_value is not None:
            raw["max_value"] = self.max_value
        if self.max_rate_per_second is not None:
            raw["max_rate_per_second"] = self.max_rate_per_second
        raw["knots"] = [knot.to_dict() for knot in self.knots]
        return raw

    def value_at(self, sample: int) -> float:
        if not self.knots:
            _refuse("trajectory-empty-lane", f"trajectory target {self.target!r} has no knots")
        if sample <= self.knots[0].at_sample:
            return self.knots[0].value
        if sample >= self.knots[-1].at_sample:
            return self.knots[-1].value
        right = 1
        while self.knots[right].at_sample <= sample:
            right += 1
        left = right - 1
        start = self.knots[left]
        end = self.knots[right]
        t = (sample - start.at_sample) / (end.at_sample - start.at_sample)
        weight = _curve_weight(self.curve, t)
        a = _encode(self.value_space, self.target, start.value)
        b = _encode(self.value_space, self.target, end.value)
        return _decode(self.value_space, a + (b - a) * weight)

    def _rate_observation(self, sample_rate_hz: int) -> tuple[float | None, bool]:
        maximum = 0.0
        discontinuous = False
        for left, right in zip(self.knots, self.knots[1:]):
            a = _encode(self.value_space, self.target, left.value)
            b = _encode(self.value_space, self.target, right.value)
            if a == b:
                continue
            seconds = (right.at_sample - left.at_sample) / sample_rate_hz
            multiplier = _PEAK_SLOPE[self.curve]
            if multiplier is None:
                discontinuous = True
            else:
                maximum = max(maximum, multiplier * abs(b - a) / seconds)
        return (None if discontinuous else maximum), discontinuous


@dataclass(frozen=True)
class LaneTemplate:
    target: str
    unit: str
    curve: str = "linear"
    value_space: str = "linear"
    min_value: float | None = None
    max_value: float | None = None
    max_rate_per_second: float | None = None

    def compile_arrow(self, raw: str, duration_samples: int) -> Lane | None:
        values = parse_arrow_values(self.target, raw)
        if values is None:
            return None
        if duration_samples < len(values):
            _refuse(
                "trajectory-too-many-knots",
                f"trajectory target {self.target!r} has {len(values)} knots but only "
                f"{duration_samples} samples",
            )
        denominator = len(values) - 1
        horizon = duration_samples - 1
        knots = tuple(
            Knot(
                at_sample=(index * horizon + denominator // 2) // denominator,
                value=value,
            )
            for index, value in enumerate(values)
        )
        return Lane(
            target=self.target,
            unit=self.unit,
            curve=normalize_curve(self.curve),
            value_space=self.value_space,
            min_value=self.min_value,
            max_value=self.max_value,
            max_rate_per_second=self.max_rate_per_second,
            knots=knots,
        )


@dataclass(frozen=True)
class TrajectoryProgram:
    format: str
    sample_rate_hz: int
    duration_samples: int
    hold_final: bool
    lanes: tuple[Lane, ...] = field(default_factory=tuple)

    @classmethod
    def empty(cls, sample_rate_hz: int, duration_samples: int) -> "TrajectoryProgram":
        return cls(
            format=FORMAT_V1,
            sample_rate_hz=sample_rate_hz,
            duration_samples=duration_samples,
            hold_final=True,
            lanes=(),
        )

    @classmethod
    def from_dict(cls, raw: Mapping[str, Any]) -> "TrajectoryProgram":
        return cls(
            format=str(raw["format"]),
            sample_rate_hz=int(raw["sample_rate_hz"]),
            duration_samples=int(raw["duration_samples"]),
            hold_final=bool(raw["hold_final"]),
            lanes=tuple(Lane.from_dict(lane) for lane in raw.get("lanes", ())),
        )

    @classmethod
    def load(cls, path: str | Path) -> "TrajectoryProgram":
        with Path(path).open("r", encoding="utf-8") as handle:
            program = cls.from_dict(json.load(handle))
        program.validate()
        return program

    def to_dict(self) -> dict[str, Any]:
        return {
            "format": self.format,
            "sample_rate_hz": self.sample_rate_hz,
            "duration_samples": self.duration_samples,
            "hold_final": self.hold_final,
            "lanes": [lane.to_dict() for lane in self.lanes],
        }

    def canonical_json(self) -> bytes:
        self.validate()
        return json.dumps(
            self.to_dict(),
            ensure_ascii=False,
            allow_nan=False,
            separators=(",", ":"),
        ).encode("utf-8")

    def digest_hex(self) -> str:
        return sha256(self.canonical_json()).hexdigest()

    def values_at(self, sample: int) -> dict[str, float]:
        self.validate()
        sample = min(max(int(sample), 0), self.duration_samples - 1)
        return {lane.target: lane.value_at(sample) for lane in self.lanes}

    def write_values(self, sample: int, output: MutableSequence[float]) -> None:
        if len(output) != len(self.lanes):
            _refuse(
                "trajectory-output-length",
                f"trajectory output buffer has length {len(output)}; expected {len(self.lanes)}",
            )
        sample = min(max(int(sample), 0), self.duration_samples - 1)
        for index, lane in enumerate(self.lanes):
            output[index] = lane.value_at(sample)

    def validate(self) -> dict[str, Any]:
        if self.format != FORMAT_V1:
            _refuse(
                "trajectory-invalid-format",
                f"trajectory format {self.format!r} is not {FORMAT_V1!r}",
            )
        if self.sample_rate_hz <= 0:
            _refuse("trajectory-invalid-sample-rate", "trajectory sample rate must be positive")
        if self.duration_samples <= 0:
            _refuse("trajectory-invalid-duration", "trajectory duration must be positive")
        if not self.hold_final:
            _refuse(
                "trajectory-release-policy-unsupported",
                "trajectory v1 only supports hold_final=true; baseline return needs an explicit law",
            )

        targets: set[str] = set()
        lane_summaries: list[dict[str, Any]] = []
        program_zero_motion = True
        for lane_index, lane in enumerate(self.lanes):
            if not lane.target.strip():
                _refuse(
                    "trajectory-empty-target",
                    f"trajectory lane {lane_index} has an empty target",
                )
            if lane.target in targets:
                _refuse(
                    "trajectory-duplicate-target",
                    f"trajectory target {lane.target!r} appears more than once",
                )
            targets.add(lane.target)
            curve = normalize_curve(lane.curve)
            if curve != lane.curve:
                _refuse(
                    "trajectory-unknown-curve",
                    f"program must carry canonical curve spelling, got {lane.curve!r}",
                )
            if lane.value_space not in _VALUE_SPACES:
                _refuse(
                    "trajectory-unknown-value-space",
                    f"trajectory target {lane.target!r} uses unknown value space "
                    f"{lane.value_space!r}",
                )
            if not lane.knots:
                _refuse(
                    "trajectory-empty-lane",
                    f"trajectory target {lane.target!r} has no knots",
                )
            if lane.knots[0].at_sample != 0:
                _refuse(
                    "trajectory-first-knot-not-zero",
                    f"trajectory target {lane.target!r} begins at sample "
                    f"{lane.knots[0].at_sample}, not sample 0",
                )
            if lane.min_value is not None and not math.isfinite(lane.min_value):
                _refuse("trajectory-invalid-bounds", f"invalid bounds for {lane.target!r}")
            if lane.max_value is not None and not math.isfinite(lane.max_value):
                _refuse("trajectory-invalid-bounds", f"invalid bounds for {lane.target!r}")
            if (
                lane.min_value is not None
                and lane.max_value is not None
                and lane.min_value > lane.max_value
            ):
                _refuse("trajectory-invalid-bounds", f"invalid bounds for {lane.target!r}")
            if lane.max_rate_per_second is not None and (
                not math.isfinite(lane.max_rate_per_second)
                or lane.max_rate_per_second < 0.0
            ):
                _refuse(
                    "trajectory-invalid-rate-bound",
                    f"invalid rate bound {lane.max_rate_per_second} for {lane.target!r}",
                )

            previous: int | None = None
            first_value = lane.knots[0].value
            zero_motion = True
            minimum = math.inf
            maximum = -math.inf
            for knot in lane.knots:
                if previous is not None and knot.at_sample <= previous:
                    _refuse(
                        "trajectory-knot-order",
                        f"trajectory target {lane.target!r} has non-increasing knots "
                        f"{previous} then {knot.at_sample}",
                    )
                previous = knot.at_sample
                if knot.at_sample < 0 or knot.at_sample >= self.duration_samples:
                    _refuse(
                        "trajectory-knot-outside-duration",
                        f"trajectory target {lane.target!r} knot {knot.at_sample} is outside "
                        f"duration {self.duration_samples}",
                    )
                if not math.isfinite(knot.value):
                    _refuse(
                        "trajectory-nonfinite-value",
                        f"trajectory target {lane.target!r} has a non-finite value at "
                        f"sample {knot.at_sample}",
                    )
                _encode(lane.value_space, lane.target, knot.value)
                if lane.min_value is not None and knot.value < lane.min_value:
                    _refuse(
                        "trajectory-below-minimum",
                        f"trajectory target {lane.target!r} value {knot.value} is below "
                        f"minimum {lane.min_value}",
                    )
                if lane.max_value is not None and knot.value > lane.max_value:
                    _refuse(
                        "trajectory-above-maximum",
                        f"trajectory target {lane.target!r} value {knot.value} is above "
                        f"maximum {lane.max_value}",
                    )
                minimum = min(minimum, knot.value)
                maximum = max(maximum, knot.value)
                zero_motion = zero_motion and knot.value == first_value

            observed_rate, discontinuous = lane._rate_observation(self.sample_rate_hz)
            if lane.max_rate_per_second is not None and (
                observed_rate is None or observed_rate > lane.max_rate_per_second
            ):
                description = "discontinuous" if observed_rate is None else str(observed_rate)
                _refuse(
                    "trajectory-rate-exceeded",
                    f"trajectory target {lane.target!r} rate {description} exceeds "
                    f"{lane.max_rate_per_second} per second",
                )
            program_zero_motion = program_zero_motion and zero_motion
            lane_summary: dict[str, Any] = {
                "target": lane.target,
                "unit": lane.unit,
                "curve": lane.curve,
                "continuity": _CONTINUITY[lane.curve],
                "value_space": lane.value_space,
                "knot_count": len(lane.knots),
                "zero_motion": zero_motion,
                "minimum": minimum,
                "maximum": maximum,
                "discontinuous": discontinuous,
            }
            if observed_rate is not None:
                lane_summary["observed_max_rate_per_second"] = observed_rate
            lane_summaries.append(lane_summary)

        return {
            "sample_rate_hz": self.sample_rate_hz,
            "duration_samples": self.duration_samples,
            "lane_count": len(self.lanes),
            "zero_motion": program_zero_motion,
            "lanes": lane_summaries,
        }

    def receipt(self) -> dict[str, Any]:
        summary = self.validate()
        return {
            "format": FORMAT_V1,
            "program_sha256": self.digest_hex(),
            **summary,
        }


def parse_arrow_values(target: str, raw: str) -> tuple[float, ...] | None:
    if "->" not in raw:
        return None
    tokens = tuple(token.strip() for token in raw.split("->"))
    if len(tokens) < 2 or any(not token for token in tokens):
        _refuse(
            "trajectory-invalid-arrow",
            f"trajectory target {target!r} has malformed arrow expression {raw!r}",
        )
    values: list[float] = []
    for token in tokens:
        try:
            values.append(float(token))
        except ValueError as error:
            raise TrajectoryError(
                "trajectory-invalid-arrow-number",
                f"trajectory target {target!r} arrow token {token!r} is not a number",
            ) from error
    return tuple(values)


def program_from_arrow_params(
    params: Mapping[str, str],
    templates: Mapping[str, LaneTemplate],
    *,
    sample_rate_hz: int,
    duration_samples: int,
    curve: str = "linear",
) -> tuple[dict[str, str], TrajectoryProgram]:
    """Compile adapter-approved arrow params and return a static parameter map.

    The first knot replaces each arrow expression in the static map. Unknown
    params—including unrelated MUS-A arrows such as pan—remain untouched.
    """

    curve = normalize_curve(curve)
    static = dict(params)
    lanes: list[Lane] = []
    for target, template in templates.items():
        raw = params.get(target)
        if raw is None:
            continue
        values = parse_arrow_values(target, raw)
        if values is None:
            continue
        bound = LaneTemplate(
            target=template.target,
            unit=template.unit,
            curve=curve,
            value_space=template.value_space,
            min_value=template.min_value,
            max_value=template.max_value,
            max_rate_per_second=template.max_rate_per_second,
        )
        lane = bound.compile_arrow(raw, duration_samples)
        assert lane is not None
        static[target] = repr(values[0])
        lanes.append(lane)
    program = TrajectoryProgram(
        format=FORMAT_V1,
        sample_rate_hz=sample_rate_hz,
        duration_samples=duration_samples,
        hold_final=True,
        lanes=tuple(lanes),
    )
    program.validate()
    return static, program


def dump_program(program: TrajectoryProgram, path: str | Path) -> None:
    program.validate()
    Path(path).write_text(
        json.dumps(program.to_dict(), indent=2, ensure_ascii=False, allow_nan=False) + "\n",
        encoding="utf-8",
    )
