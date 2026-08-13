#!/usr/bin/env python3
"""Independent scalar oracle for Ariadne Terminal's local laws.

This is not a second renderer. It deliberately omits profiles, host plumbing,
and delay-line history. It independently replays the algebraic identities that
the Rust live engine claims: discrete-gradient material closure, Givens and
closed-commutator norm preservation, boundary exchange, reservoir-paid
positive deformation, reciprocal room coupling, orthogonal radiation, and one
composite work frame.
"""

from __future__ import annotations

import argparse
import json
import math
import random
from dataclasses import asdict, dataclass

MATERIAL_CELLS = 12
ROOM_COURSES = 12
DT = 1.0 / 48_000.0
NEWTON_STEPS = 8


def potential(q: float, curvature: float, asymmetry: float) -> float:
    shifted = q * (q + 2.0 * asymmetry)
    return 0.5 * q * q + 0.25 * curvature * shifted * shifted


def divided_gradient(old: float, new: float, curvature: float, asymmetry: float) -> float:
    quadratic = 0.5 + curvature * asymmetry * asymmetry
    cubic = curvature * asymmetry
    quartic = 0.25 * curvature
    return (
        quadratic * (old + new)
        + cubic * (old * old + old * new + new * new)
        + quartic
        * (old**3 + old * old * new + old * new * new + new**3)
    )


def divided_gradient_derivative(
    old: float, new: float, curvature: float, asymmetry: float
) -> float:
    quadratic = 0.5 + curvature * asymmetry * asymmetry
    cubic = curvature * asymmetry
    quartic = 0.25 * curvature
    return (
        quadratic
        + cubic * (old + 2.0 * new)
        + quartic * (old * old + 2.0 * old * new + 3.0 * new * new)
    )


def advance_cell(
    q: float,
    p: float,
    phase_step: float,
    curvature: float,
    asymmetry: float,
) -> tuple[float, float, float]:
    next_q = q + phase_step * p
    for _ in range(NEWTON_STEPS):
        gradient = divided_gradient(q, next_q, curvature, asymmetry)
        equation = (
            2.0 * (next_q - q) / phase_step
            - 2.0 * p
            + phase_step * gradient
        )
        derivative = 2.0 / phase_step + phase_step * divided_gradient_derivative(
            q, next_q, curvature, asymmetry
        )
        step = max(-4.0, min(4.0, equation / derivative))
        next_q -= step
    next_p = 2.0 * (next_q - q) / phase_step - p
    residual = (
        2.0 * (next_q - q) / phase_step
        - 2.0 * p
        + phase_step * divided_gradient(q, next_q, curvature, asymmetry)
    )
    return next_q, next_p, residual


def givens(a: float, b: float, angle: float) -> tuple[float, float]:
    sine = math.sin(angle)
    cosine = math.cos(angle)
    return cosine * a - sine * b, sine * a + cosine * b


def apply_closed_commutator(state: list[float], first: int, a: float, b: float) -> None:
    i, j, k = first, first + 1, first + 2
    state[j], state[k] = givens(state[j], state[k], -b)
    state[i], state[j] = givens(state[i], state[j], -a)
    state[j], state[k] = givens(state[j], state[k], b)
    state[i], state[j] = givens(state[i], state[j], a)


def norm_squared(values: list[float]) -> float:
    return sum(value * value for value in values)


def normalize(values: list[float]) -> list[float]:
    norm = math.sqrt(norm_squared(values))
    return [value / norm for value in values]


def project(values: list[float], direction: list[float]) -> float:
    return sum(value * basis for value, basis in zip(values, direction, strict=True))


def remove_projection(
    values: list[float], direction: list[float], multiplier: float
) -> None:
    coefficient = project(values, direction) * multiplier
    for index, basis in enumerate(direction):
        values[index] -= coefficient * basis


def rank_one_pump(
    momenta: list[float], direction: list[float], requested_work: float, reservoir: float
) -> tuple[float, float, float]:
    direction = normalize(direction)
    scalar = project(momenta, direction)
    if abs(scalar) < 1.0e-15:
        return reservoir, 0.0, 0.0
    accepted = min(max(0.0, requested_work), reservoir)
    before = 0.5 * norm_squared(momenta) * DT
    next_scalar = math.copysign(
        math.sqrt(scalar * scalar + 2.0 * accepted / DT), scalar
    )
    increment = next_scalar - scalar
    for index, basis in enumerate(direction):
        momenta[index] += increment * basis
    after = 0.5 * norm_squared(momenta) * DT
    return reservoir - accepted, accepted, after - before - accepted


def boundary_exchange(
    momenta: list[float], left: float, right: float, rows: tuple[list[float], list[float]]
) -> tuple[float, float]:
    common = (left + right) / math.sqrt(2.0)
    difference = (left - right) / math.sqrt(2.0)
    before = 0.5 * norm_squared(momenta) * DT
    for index, weight in enumerate(rows[0]):
        common, momenta[index] = givens(common, momenta[index], 0.63 * weight)
    for index in range(len(momenta) - 1, -1, -1):
        difference, momenta[index] = givens(
            difference, momenta[index], 0.63 * rows[1][index]
        )
    after = 0.5 * norm_squared(momenta) * DT
    incoming = 0.5 * (left * left + right * right) * DT
    returned = 0.5 * (common * common + difference * difference) * DT
    work = incoming - returned
    return work, after - before - work


def orthonormal_rows(length: int) -> tuple[list[float], list[float]]:
    left = normalize([1.0] * length)
    center = 0.5 * (length - 1)
    right = [index - center for index in range(length)]
    coefficient = project(right, left)
    right = [
        value - coefficient * basis
        for value, basis in zip(right, left, strict=True)
    ]
    return left, normalize(right)


def radiation(values: list[float], eta: float) -> tuple[float, float, float]:
    left, right = orthonormal_rows(len(values))
    left_projection = project(values, left)
    right_projection = project(values, right)
    removal = 1.0 - math.sqrt(1.0 - eta)
    remove_projection(values, left, removal)
    remove_projection(values, right, removal)
    emitted_left = math.sqrt(eta) * left_projection
    emitted_right = math.sqrt(eta) * right_projection
    loss = 0.5 * (emitted_left**2 + emitted_right**2) * DT
    return emitted_left, emitted_right, loss


@dataclass
class OracleReceipt:
    minimum_potential: float = math.inf
    maximum_cell_energy_residual: float = 0.0
    maximum_cell_equation_residual: float = 0.0
    maximum_givens_norm_residual: float = 0.0
    maximum_commutator_norm_residual: float = 0.0
    minimum_commutator_displacement: float = math.inf
    maximum_boundary_metric_residual: float = 0.0
    maximum_pump_metric_residual: float = 0.0
    maximum_room_coupling_residual: float = 0.0
    maximum_radiation_residual: float = 0.0
    composite_work_frame_residual: float = 0.0


def run(seed: int, trials: int) -> OracleReceipt:
    rng = random.Random(seed)
    receipt = OracleReceipt()

    for _ in range(trials):
        q = rng.uniform(-4.0, 4.0)
        curvature = rng.uniform(0.0, 2.3)
        asymmetry = rng.uniform(-0.4, 0.4)
        receipt.minimum_potential = min(
            receipt.minimum_potential, potential(q, curvature, asymmetry)
        )

        p = rng.uniform(-4.0, 4.0)
        phase = rng.uniform(0.005, 0.48)
        before = 0.5 * p * p + potential(q, curvature, asymmetry)
        next_q, next_p, equation = advance_cell(
            q, p, phase, curvature, asymmetry
        )
        after = 0.5 * next_p * next_p + potential(
            next_q, curvature, asymmetry
        )
        receipt.maximum_cell_energy_residual = max(
            receipt.maximum_cell_energy_residual, abs(after - before)
        )
        receipt.maximum_cell_equation_residual = max(
            receipt.maximum_cell_equation_residual, abs(equation)
        )

        a, b = rng.uniform(-3.0, 3.0), rng.uniform(-3.0, 3.0)
        angle = rng.uniform(-math.pi, math.pi)
        rotated = givens(a, b, angle)
        receipt.maximum_givens_norm_residual = max(
            receipt.maximum_givens_norm_residual,
            abs(rotated[0] ** 2 + rotated[1] ** 2 - a * a - b * b),
        )

        state = [rng.uniform(-2.0, 2.0) for _ in range(3)]
        original = state.copy()
        angle_a = rng.uniform(0.05, 0.8)
        angle_b = rng.uniform(0.05, 0.8)
        apply_closed_commutator(state, 0, angle_a, angle_b)
        receipt.maximum_commutator_norm_residual = max(
            receipt.maximum_commutator_norm_residual,
            abs(norm_squared(state) - norm_squared(original)),
        )
        receipt.minimum_commutator_displacement = min(
            receipt.minimum_commutator_displacement,
            math.sqrt(
                sum(
                    (x - y) ** 2
                    for x, y in zip(state, original, strict=True)
                )
            ),
        )

        momenta = [rng.uniform(-1.0, 1.0) for _ in range(MATERIAL_CELLS)]
        rows = orthonormal_rows(MATERIAL_CELLS)
        _, residual = boundary_exchange(
            momenta, rng.uniform(-1.0, 1.0), rng.uniform(-1.0, 1.0), rows
        )
        receipt.maximum_boundary_metric_residual = max(
            receipt.maximum_boundary_metric_residual, abs(residual)
        )

        momenta = [rng.uniform(-1.0, 1.0) for _ in range(MATERIAL_CELLS)]
        direction = [rng.uniform(-1.0, 1.0) for _ in range(MATERIAL_CELLS)]
        reservoir = rng.uniform(1.0e-5, 0.002)
        _, _, residual = rank_one_pump(
            momenta, direction, rng.uniform(0.0, 0.004), reservoir
        )
        receipt.maximum_pump_metric_residual = max(
            receipt.maximum_pump_metric_residual, abs(residual)
        )

        body = [rng.uniform(-1.0, 1.0) for _ in range(MATERIAL_CELLS)]
        room = [rng.uniform(-1.0, 1.0) for _ in range(ROOM_COURSES)]
        before = 0.5 * (norm_squared(body) + norm_squared(room)) * DT
        for course in range(ROOM_COURSES):
            cell = (course * 5 + 1) % MATERIAL_CELLS
            body[cell], room[course] = givens(
                body[cell], room[course], rng.uniform(-0.1, 0.1)
            )
        after = 0.5 * (norm_squared(body) + norm_squared(room)) * DT
        receipt.maximum_room_coupling_residual = max(
            receipt.maximum_room_coupling_residual, abs(after - before)
        )

        state = [rng.uniform(-1.0, 1.0) for _ in range(MATERIAL_CELLS)]
        before = 0.5 * norm_squared(state) * DT
        _, _, emitted = radiation(state, rng.uniform(0.001, 0.12))
        after = 0.5 * norm_squared(state) * DT
        receipt.maximum_radiation_residual = max(
            receipt.maximum_radiation_residual,
            abs(before - after - emitted),
        )

    # One composite frame: boundary work + internal reservoir exchange +
    # conservative propagation + damping + radiation.
    q = [rng.uniform(-0.4, 0.4) for _ in range(MATERIAL_CELLS)]
    p = [rng.uniform(-0.4, 0.4) for _ in range(MATERIAL_CELLS)]
    curvature = [rng.uniform(0.1, 1.5) for _ in range(MATERIAL_CELLS)]
    asymmetry = [rng.uniform(-0.3, 0.3) for _ in range(MATERIAL_CELLS)]
    reservoir = 0.002

    def material_energy() -> float:
        return DT * sum(
            0.5 * momentum * momentum + potential(position, k, s)
            for position, momentum, k, s in zip(
                q, p, curvature, asymmetry, strict=True
            )
        )

    energy_start = material_energy() + reservoir
    source_work, boundary_residual = boundary_exchange(
        p, 0.23, -0.11, orthonormal_rows(MATERIAL_CELLS)
    )
    receipt.maximum_boundary_metric_residual = max(
        receipt.maximum_boundary_metric_residual, abs(boundary_residual)
    )
    reservoir, pump_work, pump_residual = rank_one_pump(
        p,
        [1.0 + 0.1 * math.cos(index) for index in range(MATERIAL_CELLS)],
        8.0e-5,
        reservoir,
    )
    receipt.maximum_pump_metric_residual = max(
        receipt.maximum_pump_metric_residual, abs(pump_residual)
    )
    for first in range(0, MATERIAL_CELLS - 2, 3):
        apply_closed_commutator(p, first, 0.13, -0.09)
    for index in range(MATERIAL_CELLS):
        q[index], p[index], _ = advance_cell(
            q[index],
            p[index],
            0.08 + 0.02 * index,
            curvature[index],
            asymmetry[index],
        )
    before_damping = 0.5 * norm_squared(p) * DT
    p[:] = [value * 0.997 for value in p]
    after_damping = 0.5 * norm_squared(p) * DT
    internal_loss = before_damping - after_damping
    _, _, radiation_loss = radiation(p, 0.04)
    energy_end = material_energy() + reservoir
    receipt.composite_work_frame_residual = energy_end - (
        energy_start + source_work - internal_loss - radiation_loss
    )
    # Pump work is intentionally absent: it is an internal reservoir/material exchange.
    assert pump_work >= 0.0

    return receipt


def verify(receipt: OracleReceipt) -> None:
    limits = {
        "minimum_potential": (-1.0e-14, math.inf),
        "maximum_cell_energy_residual": (0.0, 2.0e-11),
        "maximum_cell_equation_residual": (0.0, 2.0e-11),
        "maximum_givens_norm_residual": (0.0, 2.0e-14),
        "maximum_commutator_norm_residual": (0.0, 3.0e-14),
        "minimum_commutator_displacement": (1.0e-8, math.inf),
        "maximum_boundary_metric_residual": (0.0, 3.0e-18),
        "maximum_pump_metric_residual": (0.0, 3.0e-18),
        "maximum_room_coupling_residual": (0.0, 3.0e-18),
        "maximum_radiation_residual": (0.0, 3.0e-18),
        "composite_work_frame_residual": (-3.0e-14, 3.0e-14),
    }
    data = asdict(receipt)
    failures = []
    for field, (minimum, maximum) in limits.items():
        value = data[field]
        if not minimum <= value <= maximum:
            failures.append(
                f"{field}={value:.17g} outside [{minimum}, {maximum}]"
            )
    if failures:
        raise SystemExit(
            "Ariadne Terminal law verification failed:\n" + "\n".join(failures)
        )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--seed", type=int, default=0xA71AD9E)
    parser.add_argument("--trials", type=int, default=10_000)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    receipt = run(args.seed, args.trials)
    verify(receipt)
    if args.json:
        print(json.dumps(asdict(receipt), indent=2, sort_keys=True))
    else:
        for field, value in asdict(receipt).items():
            print(f"{field}: {value:.17g}")


if __name__ == "__main__":
    main()
