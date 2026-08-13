#!/usr/bin/env python3
"""Independent algebraic oracle for Ariadne's new active/resonant primitives.

This is intentionally not an audio renderer. It validates the exact metric laws
claimed by `ariadne_active_inclusion.rs` and `ariadne_resonant_transport.rs`.
"""
from __future__ import annotations

import argparse
import json
import math
import random
from dataclasses import asdict, dataclass

ACTIVE_RANKS = 3


def dot(a, b):
    return sum(x * y for x, y in zip(a, b, strict=True))


def norm2(a):
    return dot(a, a)


def add_projection(x, v, amount):
    for index, basis in enumerate(v):
        x[index] += amount * basis


def orthonormalize(rows):
    result = [row[:] for row in rows]
    for index in range(len(result)):
        for previous in range(index):
            projection = dot(result[index], result[previous])
            for coordinate in range(len(result[index])):
                result[index][coordinate] -= projection * result[previous][coordinate]
        norm = math.sqrt(norm2(result[index]))
        if not math.isfinite(norm) or norm <= 1.0e-12:
            raise ValueError(f"degenerate row {index}")
        result[index] = [value / norm for value in result[index]]
    return result


def givens(a, b, angle):
    sine = math.sin(angle)
    cosine = math.cos(angle)
    return cosine * a - sine * b, sine * a + cosine * b


def inclusion_step(
    momenta,
    directions,
    activation,
    reservoir,
    sample_rate,
    boundary_activity,
    room_activity,
    controls,
):
    drive, headroom, sustain, sag, bloom, feedback, space = controls
    dt = 1.0 / sample_rate
    kinetic_before = 0.5 * dt * norm2(momenta)
    material_rms = math.sqrt(norm2(momenta) / len(momenta))
    sensed = (
        abs(boundary_activity)
        + 0.08 * sustain * material_rms
        + 12.0 * feedback * space * abs(room_activity)
    )
    threshold = 0.025 + 0.25 * headroom
    excess = max(0.0, sensed - threshold)
    target = excess / (threshold + excess + 1.0e-30)
    attack = 0.012 + 0.060 * bloom
    release = 0.10 + 1.40 * sag * sag
    time_constant = attack if target > activation else release
    activation += (target - activation) * (1.0 - math.exp(-dt / time_constant))
    activation = min(1.0, max(0.0, activation))

    mu = (4.0 + 180.0 * drive * drive) * activation
    weights = (1.0, 0.62, 0.38)
    targets = (
        0.11 + 0.30 * headroom,
        0.09 + 0.23 * headroom,
        0.07 + 0.18 * headroom,
    )
    requested = 0.0
    accepted = 0.0
    lost = 0.0
    for direction, weight, target_amplitude in zip(
        directions, weights, targets, strict=True
    ):
        projection = dot(momenta, direction)
        if abs(projection) <= 1.0e-15 or mu == 0.0:
            continue
        work = (
            dt
            * mu
            * weight
            * projection
            * projection
            * (1.0 - projection * projection / (target_amplitude * target_amplitude))
        )
        if work > 0.0:
            requested += work
            take = min(work, reservoir)
            if take > 0.0:
                next_projection = math.copysign(
                    math.sqrt(projection * projection + 2.0 * take / dt),
                    projection,
                )
                add_projection(momenta, direction, next_projection - projection)
                reservoir -= take
                accepted += take
        elif work < 0.0:
            take = min(-work, 0.5 * dt * projection * projection)
            next_projection = math.copysign(
                math.sqrt(max(0.0, projection * projection - 2.0 * take / dt)),
                projection,
            )
            add_projection(momenta, direction, next_projection - projection)
            lost += take

    kinetic_after = 0.5 * dt * norm2(momenta)
    residual = kinetic_after - kinetic_before - accepted + lost
    return activation, reservoir, requested, accepted, lost, residual


def transport_step(
    displacements,
    momenta,
    pairs,
    weights,
    sample_rate,
    interlace,
    memory,
):
    dt = 1.0 / sample_rate
    kinetic_before = 0.5 * dt * norm2(momenta)
    transport_hz = 90.0 + 900.0 * interlace * interlace
    base_angle = 2.0 * math.pi * transport_hz / sample_rate
    maximum_angle = 0.0
    for (left, right), weight in zip(pairs, weights, strict=True):
        left_q = displacements[left]
        right_q = displacements[right]
        geometry = math.tanh(
            1.6 * left_q * right_q
            + 0.25 * (left_q * left_q - right_q * right_q)
        )
        angle = base_angle * (0.35 + 0.65 * memory) * weight * geometry
        momenta[left], momenta[right] = givens(
            momenta[left], momenta[right], angle
        )
        maximum_angle = max(maximum_angle, abs(angle))
    kinetic_after = 0.5 * dt * norm2(momenta)
    return kinetic_after - kinetic_before, maximum_angle


@dataclass
class Receipt:
    trials: int = 0
    max_orthogonality_residual: float = 0.0
    max_active_closure_residual: float = 0.0
    max_transport_metric_residual: float = 0.0
    min_positive_work: float = math.inf
    max_room_triggered_activation: float = 0.0
    frozen_rate_state_error: float = 0.0
    zero_state_exact: bool = True


def run(seed, trials):
    rng = random.Random(seed)
    directions = orthonormalize(
        [
            [1.0, 1.0, 0.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 1.0, 0.0, 1.0, 0.0],
            [1.0, -1.0, 1.0, -1.0, 1.0, -1.0],
        ]
    )
    receipt = Receipt(trials=trials)
    for index in range(ACTIVE_RANKS):
        for other in range(ACTIVE_RANKS):
            target = 1.0 if index == other else 0.0
            receipt.max_orthogonality_residual = max(
                receipt.max_orthogonality_residual,
                abs(dot(directions[index], directions[other]) - target),
            )

    momenta = [0.0] * 6
    activation = 0.0
    reservoir = 1.0
    for _ in range(1_000):
        activation, reservoir, _, accepted, lost, residual = inclusion_step(
            momenta,
            directions,
            activation,
            reservoir,
            48_000.0,
            0.0,
            0.0,
            (1.0, 0.5, 1.0, 1.0, 1.0, 1.0, 1.0),
        )
        receipt.zero_state_exact &= (
            accepted == 0.0
            and lost == 0.0
            and residual == 0.0
            and momenta == [0.0] * 6
        )

    for _ in range(trials):
        momenta = [rng.uniform(-0.5, 0.5) for _ in range(6)]
        activation = rng.random()
        reservoir = rng.random() * 0.05
        controls = (
            rng.random() * 1.5,
            0.02 + 0.98 * rng.random(),
            rng.random(),
            rng.random(),
            rng.random(),
            rng.random(),
            rng.random(),
        )
        activation, reservoir, _, accepted, _, residual = inclusion_step(
            momenta,
            directions,
            activation,
            reservoir,
            48_000.0,
            rng.random(),
            rng.random() * 0.1,
            controls,
        )
        receipt.max_active_closure_residual = max(
            receipt.max_active_closure_residual, abs(residual)
        )
        if accepted > 0.0:
            receipt.min_positive_work = min(receipt.min_positive_work, accepted)

        displacements = [rng.uniform(-2.0, 2.0) for _ in range(6)]
        momenta = [rng.uniform(-2.0, 2.0) for _ in range(6)]
        residual, _ = transport_step(
            displacements,
            momenta,
            [(0, 2), (1, 3), (2, 4)],
            (1.0, 0.75, -0.5),
            48_000.0,
            rng.random(),
            rng.random(),
        )
        receipt.max_transport_metric_residual = max(
            receipt.max_transport_metric_residual, abs(residual)
        )

    for room_activity in (0.0, 0.04):
        momenta = [0.02, 0.0, 0.0, 0.0, 0.0, 0.0]
        activation = 0.0
        reservoir = 1.0
        for _ in range(4_800):
            activation, reservoir, *_ = inclusion_step(
                momenta,
                directions,
                activation,
                reservoir,
                48_000.0,
                0.0,
                room_activity,
                (0.42, 0.72, 0.24, 0.35, 0.36, 1.0, 1.0),
            )
        if room_activity > 0.0:
            receipt.max_room_triggered_activation = activation
        else:
            assert activation == 0.0

    displacements = [0.7, 0.0, 0.9, 0.0, 0.0, 0.0]
    low_state = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0]
    high_state = low_state[:]
    for _ in range(480):
        transport_step(
            displacements,
            low_state,
            [(0, 2)],
            (1.0,),
            48_000.0,
            0.6,
            0.8,
        )
    for _ in range(960):
        transport_step(
            displacements,
            high_state,
            [(0, 2)],
            (1.0,),
            96_000.0,
            0.6,
            0.8,
        )
    receipt.frozen_rate_state_error = max(
        abs(left - right) for left, right in zip(low_state, high_state, strict=True)
    )
    if receipt.min_positive_work is math.inf:
        receipt.min_positive_work = 0.0
    return receipt


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--seed", type=int, default=20_260_813)
    parser.add_argument("--trials", type=int, default=10_000)
    parser.add_argument("--json", action="store_true")
    arguments = parser.parse_args()
    receipt = run(arguments.seed, arguments.trials)
    values = asdict(receipt)
    limits = {
        "max_orthogonality_residual": 2.0e-14,
        "max_active_closure_residual": 2.0e-13,
        "max_transport_metric_residual": 2.0e-14,
        "frozen_rate_state_error": 5.0e-12,
    }
    okay = (
        receipt.zero_state_exact
        and receipt.max_room_triggered_activation > 0.05
        and all(values[name] <= limit for name, limit in limits.items())
    )
    output = {
        "ok": okay,
        "seed": arguments.seed,
        "limits": limits,
        "receipt": values,
    }
    print(json.dumps(output, indent=2) if arguments.json else output)
    raise SystemExit(0 if okay else 1)


if __name__ == "__main__":
    main()
