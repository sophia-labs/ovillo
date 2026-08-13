#!/usr/bin/env python3
"""Independent standard-library laws for Ariadne's maximal same-body runtime.

This intentionally does not import the Rust implementation. It checks the
shared mathematical obligations behind the arbitrary modal metric, reciprocal
room coupling, orthogonal room scattering/radiation, and finite-reservoir
active transfer.
"""

from __future__ import annotations

import argparse
import json
import math
import random
from pathlib import Path

N = 24
COURSES = 8
SAMPLE_RATE = 96_000.0
DT = 1.0 / SAMPLE_RATE


def norm(values: list[float]) -> list[float]:
    magnitude = math.sqrt(sum(value * value for value in values))
    return [value / magnitude for value in values]


def dot(left: list[float], right: list[float]) -> float:
    return sum(a * b for a, b in zip(left, right, strict=True))


def givens(left: float, right: float, angle: float) -> tuple[float, float]:
    cosine = math.cos(angle)
    sine = math.sin(angle)
    return cosine * left - sine * right, sine * left + cosine * right


def kinetic(values: list[float]) -> float:
    return 0.5 * DT * sum(value * value for value in values)


def metric_energy(q: list[float], p: list[float], frequencies: list[float]) -> float:
    return 0.5 * DT * sum(
        momentum * momentum + (2.0 * math.pi * frequency * displacement) ** 2
        for displacement, momentum, frequency in zip(q, p, frequencies, strict=True)
    )


def guitar_metric() -> list[float]:
    fundamentals = [
        82.406_889_228,
        110.0,
        146.832_383_959,
        195.997_717_991,
        246.941_650_628,
        329.627_556_913,
    ]
    result: list[float] = []
    for fundamental in fundamentals:
        for partial in range(1, 5):
            result.append(
                fundamental
                * partial
                * math.sqrt(1.0 + 1.5e-5 * partial * partial)
            )
    return result


def trombone_metric() -> list[float]:
    fundamental = 116.540_940_380
    return [
        fundamental
        * harmonic
        * math.sqrt(1.0 + 1.9e-5 * harmonic * harmonic)
        for harmonic in range(1, N + 1)
    ]


def room_directions() -> list[list[float]]:
    result: list[list[float]] = []
    for course in range(COURSES):
        direction = [0.0] * N
        first = (course * 3 + 1) % N
        second = (first + 5) % N
        direction[first] = 1.0
        direction[second] = 0.35 if course % 2 == 0 else -0.35
        result.append(norm(direction))
    return result


def run(trials: int, seed: int) -> dict[str, object]:
    rng = random.Random(seed)
    guitar = guitar_metric()
    trombone = trombone_metric()
    directions = room_directions()
    weights = [1.0, 0.91, 0.82, 0.74, 0.67, 0.60, 0.54, 0.48]
    scatter = norm([1.0, -0.73, 0.88, -0.61, 0.79, -0.52, 0.69, -0.44])
    radiation_left = norm([1.0, 0.62, 0.18, -0.31, -0.70, -0.54, 0.08, 0.48])
    radiation_right = [0.17, -0.51, -0.72, -0.24, 0.36, 0.61, 0.31, -0.22]
    projection = dot(radiation_right, radiation_left)
    radiation_right = norm(
        [
            value - projection * left
            for value, left in zip(radiation_right, radiation_left, strict=True)
        ]
    )

    maxima = {
        "maximum_relative_metric_configuration_work_residual": 0.0,
        "maximum_room_body_coupling_residual": 0.0,
        "maximum_householder_scattering_residual": 0.0,
        "maximum_two_channel_radiation_residual": 0.0,
        "maximum_active_reservoir_transfer_residual": 0.0,
    }

    for trial in range(trials):
        q = [rng.uniform(-0.01, 0.01) for _ in range(N)]
        p = [rng.uniform(-1.0, 1.0) for _ in range(N)]
        before = metric_energy(q, p, guitar)
        after = metric_energy(q, p, trombone)
        work = 0.5 * DT * sum(
            (
                (2.0 * math.pi * target) ** 2
                - (2.0 * math.pi * source) ** 2
            )
            * displacement
            * displacement
            for displacement, source, target in zip(q, guitar, trombone, strict=True)
        )
        relative = abs(after - before - work) / max(before, after, 1.0e-30)
        maxima["maximum_relative_metric_configuration_work_residual"] = max(
            maxima["maximum_relative_metric_configuration_work_residual"], relative
        )

        body = [rng.uniform(-1.0, 1.0) for _ in range(N)]
        heads = [rng.uniform(-1.0, 1.0) for _ in range(COURSES)]
        body_before = kinetic(body)
        heads_before = kinetic(heads)
        angle = (
            2.0
            * math.pi
            * (2.0 + 118.0 * rng.random() ** 2)
            / SAMPLE_RATE
        )
        for course in range(COURSES):
            material_projection = dot(body, directions[course])
            next_projection, next_head = givens(
                material_projection,
                heads[course],
                angle * weights[course],
            )
            increment = next_projection - material_projection
            body = [
                value + increment * basis
                for value, basis in zip(body, directions[course], strict=True)
            ]
            heads[course] = next_head
        coupling_residual = (
            kinetic(body) - body_before + kinetic(heads) - heads_before
        )
        maxima["maximum_room_body_coupling_residual"] = max(
            maxima["maximum_room_body_coupling_residual"],
            abs(coupling_residual),
        )

        scattering_before = kinetic(heads)
        scatter_projection = dot(heads, scatter)
        scattered = [
            value - 2.0 * basis * scatter_projection
            for value, basis in zip(heads, scatter, strict=True)
        ]
        maxima["maximum_householder_scattering_residual"] = max(
            maxima["maximum_householder_scattering_residual"],
            abs(kinetic(scattered) - scattering_before),
        )

        radiation_fraction = 0.08 * rng.random()
        left_projection = dot(scattered, radiation_left)
        right_projection = dot(scattered, radiation_right)
        emitted_left = math.sqrt(radiation_fraction) * left_projection
        emitted_right = math.sqrt(radiation_fraction) * right_projection
        removal = 1.0 - math.sqrt(1.0 - radiation_fraction)
        retained = [
            value
            - removal
            * (left * left_projection + right * right_projection)
            for value, left, right in zip(
                scattered, radiation_left, radiation_right, strict=True
            )
        ]
        radiated = 0.5 * DT * (
            emitted_left * emitted_left + emitted_right * emitted_right
        )
        maxima["maximum_two_channel_radiation_residual"] = max(
            maxima["maximum_two_channel_radiation_residual"],
            abs(kinetic(retained) - kinetic(scattered) + radiated),
        )

        direction = directions[trial % COURSES]
        material = [rng.uniform(-1.0, 1.0) for _ in range(N)]
        reservoir = rng.uniform(0.0, 0.002)
        material_projection = dot(material, direction)
        accepted = min(rng.uniform(0.0, 0.001), reservoir)
        total_before = kinetic(material) + reservoir
        if abs(material_projection) > 1.0e-15 and accepted > 0.0:
            next_projection = math.copysign(
                math.sqrt(
                    material_projection * material_projection
                    + 2.0 * accepted / DT
                ),
                material_projection,
            )
            increment = next_projection - material_projection
            material = [
                value + increment * basis
                for value, basis in zip(material, direction, strict=True)
            ]
            reservoir -= accepted
        total_after = kinetic(material) + reservoir
        maxima["maximum_active_reservoir_transfer_residual"] = max(
            maxima["maximum_active_reservoir_transfer_residual"],
            abs(total_after - total_before),
        )

    return {
        "schema_version": "1.0.0",
        "oracle": "independent_standard_library_maximal_body_laws",
        "trials": trials,
        "seed": seed,
        "sample_rate_hz": SAMPLE_RATE,
        **maxima,
        "guitar_course_fundamentals_hz": [guitar[4 * course] for course in range(6)],
        "trombone_first_resonance_hz": trombone[0],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--trials", type=int, default=50_000)
    parser.add_argument("--seed", type=int, default=0xA71AD9E)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    result = run(args.trials, args.seed)
    rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    print(rendered, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
