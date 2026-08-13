#!/usr/bin/env python3
"""Independent scalar oracle for the shared string/brass Ariadne laws.

This is not a second renderer. It independently checks the identities claimed
by `ariadne_morphing_field.rs` and `ariadne_compliant_aperture.rs`:

* exact modal rotation in a configuration-dependent Hamiltonian metric;
* exact configuration work at fixed live state;
* finite-aperture radiation as an orthogonal energy split;
* source/aperture/field scattering from incoming-minus-returned boundary work;
* reciprocal zero-work sweeping exchange;
* exact localized saturable source work/loss;
* discrete-gradient aperture propagation;
* inverse length/frequency scaling and the exact zero state.
"""

from __future__ import annotations

import argparse
import json
import math
import random


def rotate(left: float, right: float, angle: float) -> tuple[float, float]:
    cosine = math.cos(angle)
    sine = math.sin(angle)
    return (
        cosine * left - sine * right,
        sine * left + cosine * right,
    )


def run(trials: int, seed: int) -> dict[str, object]:
    rng = random.Random(seed)
    maxima = {
        "modal_absolute": 0.0,
        "modal_relative": 0.0,
        "configuration_absolute": 0.0,
        "configuration_relative": 0.0,
        "radiation": 0.0,
        "source_boundary": 0.0,
        "sweep": 0.0,
        "active": 0.0,
        "aperture_absolute": 0.0,
        "aperture_relative": 0.0,
        "inverse_length": 0.0,
    }

    for _ in range(trials):
        sample_rate = rng.uniform(8_000.0, 384_000.0)
        dt = 1.0 / sample_rate

        # Exact one-mode Hamiltonian propagation.
        frequency = rng.uniform(20.0, min(7_000.0, sample_rate / 4.0))
        omega = 2.0 * math.pi * frequency
        displacement = rng.uniform(-2.0, 2.0)
        momentum = rng.uniform(-2.0, 2.0)
        energy_before = 0.5 * dt * (
            momentum * momentum + (omega * displacement) ** 2
        )
        next_displacement, next_momentum = rotate(
            displacement,
            momentum / omega,
            omega * dt,
        )
        next_momentum *= omega
        energy_after = 0.5 * dt * (
            next_momentum * next_momentum
            + (omega * next_displacement) ** 2
        )
        residual = abs(energy_after - energy_before)
        maxima["modal_absolute"] = max(maxima["modal_absolute"], residual)
        maxima["modal_relative"] = max(
            maxima["modal_relative"], residual / max(abs(energy_before), 1.0e-30)
        )

        # Metric/configuration work at fixed state.
        old_length = rng.uniform(0.5, 2.0)
        new_length = rng.uniform(0.5, 2.0)
        old_omega = omega / old_length
        new_omega = omega / new_length
        energy_before = 0.5 * dt * (
            momentum * momentum + (old_omega * displacement) ** 2
        )
        energy_after = 0.5 * dt * (
            momentum * momentum + (new_omega * displacement) ** 2
        )
        control_work = 0.5 * dt * (
            new_omega * new_omega - old_omega * old_omega
        ) * displacement * displacement
        residual = abs(energy_after - energy_before - control_work)
        maxima["configuration_absolute"] = max(
            maxima["configuration_absolute"], residual
        )
        maxima["configuration_relative"] = max(
            maxima["configuration_relative"],
            residual
            / max(
                abs(energy_before) + abs(energy_after) + abs(control_work),
                1.0e-30,
            ),
        )
        maxima["inverse_length"] = max(
            maxima["inverse_length"],
            abs(old_omega / new_omega - new_length / old_length),
        )

        # Finite-aperture radiation.
        dimension = 8
        direction = [rng.uniform(-1.0, 1.0) for _ in range(dimension)]
        norm = math.sqrt(sum(value * value for value in direction))
        direction = [value / norm for value in direction]
        field = [rng.uniform(-2.0, 2.0) for _ in range(dimension)]
        fraction = rng.random()
        projection = sum(
            value * basis for value, basis in zip(field, direction, strict=True)
        )
        energy_before = 0.5 * dt * sum(value * value for value in field)
        emitted = math.sqrt(fraction) * projection
        removal = 1.0 - math.sqrt(1.0 - fraction)
        field = [
            value - removal * basis * projection
            for value, basis in zip(field, direction, strict=True)
        ]
        energy_after = 0.5 * dt * sum(value * value for value in field)
        radiation = 0.5 * dt * emitted * emitted
        maxima["radiation"] = max(
            maxima["radiation"], abs(energy_after - energy_before + radiation)
        )

        # Incoming source wave scatters with aperture and field.
        source = rng.uniform(-1.0, 1.0)
        aperture_momentum = rng.uniform(-1.0, 1.0)
        field = [rng.uniform(-1.0, 1.0) for _ in range(dimension)]
        field_flow = sum(
            value * basis for value, basis in zip(field, direction, strict=True)
        )
        local_before = 0.5 * dt * (
            aperture_momentum * aperture_momentum
            + sum(value * value for value in field)
        )
        incoming = source
        source, aperture_momentum = rotate(
            source, aperture_momentum, rng.uniform(-0.2, 0.2)
        )
        source, next_flow = rotate(source, field_flow, rng.uniform(-0.2, 0.2))
        increment = next_flow - field_flow
        field = [
            value + increment * basis
            for value, basis in zip(field, direction, strict=True)
        ]
        local_after = 0.5 * dt * (
            aperture_momentum * aperture_momentum
            + sum(value * value for value in field)
        )
        boundary_work = 0.5 * dt * (incoming * incoming - source * source)
        maxima["source_boundary"] = max(
            maxima["source_boundary"],
            abs(local_after - local_before - boundary_work),
        )

        # Aperture sweeping is an internal reciprocal exchange.
        field_flow = sum(
            value * basis for value, basis in zip(field, direction, strict=True)
        )
        local_before = 0.5 * dt * (
            aperture_momentum * aperture_momentum
            + sum(value * value for value in field)
        )
        aperture_momentum, next_flow = rotate(
            aperture_momentum, field_flow, rng.uniform(-0.3, 0.3)
        )
        increment = next_flow - field_flow
        field = [
            value + increment * basis
            for value, basis in zip(field, direction, strict=True)
        ]
        local_after = 0.5 * dt * (
            aperture_momentum * aperture_momentum
            + sum(value * value for value in field)
        )
        maxima["sweep"] = max(maxima["sweep"], abs(local_after - local_before))

        # Localized saturable source work/loss.
        if abs(aperture_momentum) < 1.0e-12:
            aperture_momentum = 0.1
        target = rng.uniform(0.05, 0.8)
        signed_work = (
            dt
            * rng.uniform(0.0, 100.0)
            * aperture_momentum
            * aperture_momentum
            * (1.0 - aperture_momentum * aperture_momentum / (target * target))
        )
        energy_before = 0.5 * dt * aperture_momentum * aperture_momentum
        active_work = 0.0
        loss = 0.0
        if signed_work > 0.0:
            aperture_momentum = math.copysign(
                math.sqrt(
                    aperture_momentum * aperture_momentum
                    + 2.0 * signed_work / dt
                ),
                aperture_momentum,
            )
            active_work = signed_work
        elif signed_work < 0.0:
            available = 0.5 * dt * aperture_momentum * aperture_momentum
            loss = min(-signed_work, available)
            aperture_momentum = math.copysign(
                math.sqrt(
                    max(
                        0.0,
                        aperture_momentum * aperture_momentum - 2.0 * loss / dt,
                    )
                ),
                aperture_momentum,
            )
        energy_after = 0.5 * dt * aperture_momentum * aperture_momentum
        maxima["active"] = max(
            maxima["active"],
            abs(energy_after - energy_before - active_work + loss),
        )

        # Nonlinear aperture discrete-gradient propagation.
        aperture_frequency = rng.uniform(20.0, min(2_000.0, sample_rate / 8.0))
        aperture_omega = 2.0 * math.pi * aperture_frequency
        quartic = 10.0 ** rng.uniform(3.0, 8.0)
        asymmetry = rng.uniform(-0.05, 0.05)
        equilibrium = rng.uniform(-0.02, 0.02)
        old_displacement = rng.uniform(-0.05, 0.05)
        old_momentum = rng.uniform(-2.0, 2.0)
        quadratic = 0.5 * aperture_omega * aperture_omega + quartic * asymmetry**2
        cubic = quartic * asymmetry
        fourth = 0.25 * quartic

        def potential(displacement: float) -> float:
            centered = displacement - equilibrium
            return (
                quadratic * centered**2
                + cubic * centered**3
                + fourth * centered**4
            )

        def divided_gradient(old: float, new: float) -> float:
            old -= equilibrium
            new -= equilibrium
            return (
                quadratic * (old + new)
                + cubic * (old * old + old * new + new * new)
                + fourth
                * (old**3 + old * old * new + old * new * new + new**3)
            )

        def divided_gradient_derivative(old: float, new: float) -> float:
            old -= equilibrium
            new -= equilibrium
            return (
                quadratic
                + cubic * (old + 2.0 * new)
                + fourth * (old * old + 2.0 * old * new + 3.0 * new * new)
            )

        next_displacement = old_displacement + dt * old_momentum
        for _newton in range(12):
            equation = (
                2.0 * (next_displacement - old_displacement) / dt
                - 2.0 * old_momentum
                + dt * divided_gradient(old_displacement, next_displacement)
            )
            derivative = 2.0 / dt + dt * divided_gradient_derivative(
                old_displacement, next_displacement
            )
            next_displacement -= max(-4.0, min(4.0, equation / derivative))
        next_momentum = (
            2.0 * (next_displacement - old_displacement) / dt - old_momentum
        )
        energy_before = dt * (
            0.5 * old_momentum * old_momentum + potential(old_displacement)
        )
        energy_after = dt * (
            0.5 * next_momentum * next_momentum + potential(next_displacement)
        )
        residual = abs(energy_after - energy_before)
        maxima["aperture_absolute"] = max(
            maxima["aperture_absolute"], residual
        )
        maxima["aperture_relative"] = max(
            maxima["aperture_relative"],
            residual / max(abs(energy_before), 1.0e-30),
        )

    result: dict[str, object] = {
        "seed": seed,
        "trials": trials,
        "maximum_modal_propagation_absolute_residual": maxima["modal_absolute"],
        "maximum_modal_propagation_relative_residual": maxima["modal_relative"],
        "maximum_configuration_work_absolute_residual": maxima[
            "configuration_absolute"
        ],
        "maximum_configuration_work_relative_residual": maxima[
            "configuration_relative"
        ],
        "maximum_radiation_residual": maxima["radiation"],
        "maximum_source_boundary_residual": maxima["source_boundary"],
        "maximum_sweep_metric_residual": maxima["sweep"],
        "maximum_active_work_residual": maxima["active"],
        "maximum_aperture_energy_absolute_residual": maxima["aperture_absolute"],
        "maximum_aperture_energy_relative_residual": maxima["aperture_relative"],
        "maximum_inverse_length_error": maxima["inverse_length"],
        "exact_zero": rotate(0.0, 0.0, 0.37) == (0.0, 0.0),
    }
    result["passed"] = bool(
        result["maximum_modal_propagation_relative_residual"] < 5.0e-14
        and result["maximum_configuration_work_relative_residual"] < 5.0e-14
        and result["maximum_radiation_residual"] < 1.0e-15
        and result["maximum_source_boundary_residual"] < 1.0e-15
        and result["maximum_sweep_metric_residual"] < 1.0e-15
        and result["maximum_active_work_residual"] < 1.0e-15
        and result["maximum_aperture_energy_relative_residual"] < 1.0e-9
        and result["maximum_inverse_length_error"] < 1.0e-12
        and result["exact_zero"]
    )
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--trials", type=int, default=10_000)
    parser.add_argument("--seed", type=lambda value: int(value, 0), default=0xA71AD3)
    parser.add_argument("--json", action="store_true")
    arguments = parser.parse_args()
    result = run(arguments.trials, arguments.seed)
    if arguments.json:
        print(json.dumps(result, indent=2, sort_keys=True))
    else:
        for key, value in result.items():
            print(f"{key}: {value}")
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
