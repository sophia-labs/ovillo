"""Synthetic lexical world with noncommuting predictive operators.

The world deliberately separates:

- latent operator identity;
- context-dependent surface realization;
- event boundary;
- predictive state;
- exact byte surface.

The same surface ``b"ba"`` realizes different operators in the two modes, and
rotation/reflection do not commute. Surface realizations have variable length
and overlapping substrings so a frequency codec is not identical to the planted
operator ontology.
"""

from __future__ import annotations

from dataclasses import dataclass
import random
from typing import Iterable

OPS: tuple[str, ...] = ("R", "L", "F", "T")
OP_TO_ID = {name: index for index, name in enumerate(OPS)}

NEXT_TABLE: dict[tuple[int, int, int], str] = {
    (0, 0, 0): "T",
    (0, 0, 1): "F",
    (0, 1, 0): "L",
    (0, 1, 1): "T",
    (1, 0, 0): "F",
    (1, 0, 1): "L",
    (1, 1, 0): "R",
    (1, 1, 1): "L",
    (2, 0, 0): "R",
    (2, 0, 1): "L",
    (2, 1, 0): "F",
    (2, 1, 1): "F",
    (3, 0, 0): "T",
    (3, 0, 1): "R",
    (3, 1, 0): "L",
    (3, 1, 1): "T",
}

SURFACES: dict[int, dict[str, tuple[bytes, ...]]] = {
    0: {
        "R": (b"ab", b"cda", b"ba"),
        "L": (b"ca", b"abc"),
        "F": (b"bc", b"dab"),
        "T": (b"da", b"bcd"),
    },
    1: {
        "R": (b"ac", b"cba"),
        "L": (b"cb", b"abd"),
        "F": (b"bd", b"dac", b"ba"),
        "T": (b"dc", b"bca"),
    },
}

HEADER_GROUP: tuple[int, ...] = tuple(range(ord("p"), ord("x")))
HEADER_MODE: tuple[int, int] = (ord("0"), ord("1"))


@dataclass(frozen=True, slots=True)
class EventSpan:
    start: int
    end: int
    surface: bytes
    operator: int
    mode_before: int


@dataclass(frozen=True, slots=True)
class LexicalExample:
    surface: bytes
    boundaries: tuple[bool, ...]
    operator_labels: tuple[int, ...]
    next_operator_labels: tuple[int, ...]
    mode_labels: tuple[int, ...]
    group_labels: tuple[int, ...]
    spans: tuple[EventSpan, ...]
    initial_state: tuple[int, int, int]
    latent_operators: tuple[int, ...]

    def validate(self) -> None:
        length = len(self.surface)
        vectors = (
            self.boundaries,
            self.operator_labels,
            self.next_operator_labels,
            self.mode_labels,
            self.group_labels,
        )
        if any(len(vector) != length for vector in vectors):
            raise ValueError("all byte-aligned labels must match the surface length")
        if not self.boundaries[1]:
            raise ValueError("the two-byte header must end at position one")
        previous = 2
        for span in self.spans:
            if span.start != previous or span.end <= span.start:
                raise ValueError("event spans must be contiguous and nonempty")
            if self.surface[span.start : span.end] != span.surface:
                raise ValueError("event span surface does not match byte stream")
            if not self.boundaries[span.end - 1]:
                raise ValueError("every event span must end at a registered boundary")
            previous = span.end
        if previous != length:
            raise ValueError("event spans must cover the post-header surface")


def apply_operator(
    state: tuple[int, int, int], operator: str
) -> tuple[int, int, int]:
    rotation, reflected, mode = state
    if operator == "T":
        return rotation, reflected, 1 - mode
    op_rotation, op_reflected = {
        "R": (1, 0),
        "L": (3, 0),
        "F": (0, 1),
    }[operator]
    next_rotation = (
        op_rotation + (-rotation if op_reflected else rotation)
    ) % 4
    return next_rotation, op_reflected ^ reflected, mode


def compose_operators(first: str, second: str) -> tuple[int, int, int]:
    state = (0, 0, 0)
    state = apply_operator(state, second)
    state = apply_operator(state, first)
    return state


def operator_sequence(
    initial_state: tuple[int, int, int], event_count: int
) -> tuple[int, ...]:
    state = initial_state
    operators: list[int] = []
    for _ in range(event_count):
        operator = NEXT_TABLE[state]
        operators.append(OP_TO_ID[operator])
        state = apply_operator(state, operator)
    return tuple(operators)


def render_surface(
    state: tuple[int, int, int],
    operator_id: int,
    event_index: int,
    *,
    rng: random.Random,
    policy: str = "random",
) -> bytes:
    operator = OPS[operator_id]
    variants = SURFACES[state[2]][operator]
    if policy == "random":
        return rng.choice(variants)
    if policy == "counterfactual":
        return variants[(3 * state[0] + state[1] + event_index) % len(variants)]
    if policy == "homograph_heavy":
        if operator in ("R", "F") and b"ba" in variants and event_index % 3 == 1:
            return b"ba"
        return variants[(event_index + state[0] + 2 * state[1]) % len(variants)]
    raise ValueError(f"unknown surface policy: {policy}")


def generate_example(
    *,
    latent_seed: int,
    surface_seed: int,
    event_count: int = 12,
    surface_policy: str = "homograph_heavy",
) -> LexicalExample:
    latent_rng = random.Random(latent_seed)
    state = (
        latent_rng.randrange(4),
        latent_rng.randrange(2),
        latent_rng.randrange(2),
    )
    initial_state = state
    surface_rng = random.Random(surface_seed)

    data = bytearray(
        [HEADER_GROUP[state[0] + 4 * state[1]], HEADER_MODE[state[2]]]
    )
    boundaries: list[bool] = [False, True]
    operator_labels: list[int] = [-100, -100]
    next_operator_labels: list[int] = [-100, OP_TO_ID[NEXT_TABLE[state]]]
    mode_labels: list[int] = [state[2], state[2]]
    group_labels: list[int] = [
        state[0] + 4 * state[1],
        state[0] + 4 * state[1],
    ]
    spans: list[EventSpan] = []
    latent_operators: list[int] = []

    for event_index in range(event_count):
        operator_name = NEXT_TABLE[state]
        operator_id = OP_TO_ID[operator_name]
        surface = render_surface(
            state,
            operator_id,
            event_index,
            rng=surface_rng,
            policy=surface_policy,
        )
        start = len(data)
        mode_before = state[2]
        data.extend(surface)
        for offset in range(len(surface)):
            at_end = offset == len(surface) - 1
            next_state = apply_operator(state, operator_name) if at_end else state
            boundaries.append(at_end)
            operator_labels.append(operator_id if at_end else -100)
            next_operator_labels.append(
                OP_TO_ID[NEXT_TABLE[next_state]] if at_end else -100
            )
            mode_labels.append(next_state[2])
            group_labels.append(next_state[0] + 4 * next_state[1])
        state = apply_operator(state, operator_name)
        spans.append(EventSpan(start, len(data), surface, operator_id, mode_before))
        latent_operators.append(operator_id)

    result = LexicalExample(
        surface=bytes(data),
        boundaries=tuple(boundaries),
        operator_labels=tuple(operator_labels),
        next_operator_labels=tuple(next_operator_labels),
        mode_labels=tuple(mode_labels),
        group_labels=tuple(group_labels),
        spans=tuple(spans),
        initial_state=initial_state,
        latent_operators=tuple(latent_operators),
    )
    result.validate()
    return result


def rerender_example(
    example: LexicalExample,
    *,
    surface_seed: int,
    surface_policy: str = "random",
) -> LexicalExample:
    state = example.initial_state
    rng = random.Random(surface_seed)
    data = bytearray(
        [HEADER_GROUP[state[0] + 4 * state[1]], HEADER_MODE[state[2]]]
    )
    boundaries: list[bool] = [False, True]
    operator_labels: list[int] = [-100, -100]
    next_operator_labels: list[int] = [-100, OP_TO_ID[NEXT_TABLE[state]]]
    mode_labels: list[int] = [state[2], state[2]]
    group_labels: list[int] = [
        state[0] + 4 * state[1],
        state[0] + 4 * state[1],
    ]
    spans: list[EventSpan] = []
    for event_index, operator_id in enumerate(example.latent_operators):
        operator_name = OPS[operator_id]
        if operator_name != NEXT_TABLE[state]:
            raise ValueError("latent operator sequence is inconsistent with state law")
        surface = render_surface(
            state,
            operator_id,
            event_index,
            rng=rng,
            policy=surface_policy,
        )
        start = len(data)
        mode_before = state[2]
        data.extend(surface)
        for offset in range(len(surface)):
            at_end = offset == len(surface) - 1
            next_state = apply_operator(state, operator_name) if at_end else state
            boundaries.append(at_end)
            operator_labels.append(operator_id if at_end else -100)
            next_operator_labels.append(
                OP_TO_ID[NEXT_TABLE[next_state]] if at_end else -100
            )
            mode_labels.append(next_state[2])
            group_labels.append(next_state[0] + 4 * next_state[1])
        state = apply_operator(state, operator_name)
        spans.append(EventSpan(start, len(data), surface, operator_id, mode_before))
    result = LexicalExample(
        surface=bytes(data),
        boundaries=tuple(boundaries),
        operator_labels=tuple(operator_labels),
        next_operator_labels=tuple(next_operator_labels),
        mode_labels=tuple(mode_labels),
        group_labels=tuple(group_labels),
        spans=tuple(spans),
        initial_state=example.initial_state,
        latent_operators=example.latent_operators,
    )
    result.validate()
    return result


def all_surface_bytes() -> tuple[int, ...]:
    values = set(HEADER_GROUP) | set(HEADER_MODE)
    for by_mode in SURFACES.values():
        for variants in by_mode.values():
            for surface in variants:
                values.update(surface)
    return tuple(sorted(values))


class ByteCodec:
    def __init__(self, alphabet: Iterable[int] | None = None):
        values = tuple(sorted(alphabet if alphabet is not None else all_surface_bytes()))
        if not values:
            raise ValueError("byte alphabet may not be empty")
        self.values = values
        self.to_id = {value: index for index, value in enumerate(values)}
        self.pad_id = len(values)
        self.bos_id = len(values) + 1
        self.eos_id = len(values) + 2

    @property
    def model_classes(self) -> int:
        return len(self.values) + 3

    @property
    def source_classes(self) -> int:
        return len(self.values) + 2

    def encode(self, surface: bytes) -> tuple[int, ...]:
        return tuple(self.to_id[value] for value in surface)

    def decode(self, ids: Iterable[int]) -> bytes:
        return bytes(self.values[index] for index in ids)
