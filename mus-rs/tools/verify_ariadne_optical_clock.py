#!/usr/bin/env python3
"""Independent integer oracle for Ariadne's fixed optical scheduler."""

from __future__ import annotations

import argparse
import json
import random
from dataclasses import dataclass


@dataclass
class Clock:
    material_rate: int
    optical_rate: int
    batch: int
    completed: int = 0
    target: int = 0
    last_tick: int = 0

    def schedule(self, tick: int) -> int:
        if tick < self.last_tick:
            raise ValueError("backward")
        self.target = tick * self.optical_rate // self.material_rate
        self.last_tick = tick
        return min(self.target - self.completed, self.batch)

    def next_tick(self) -> int:
        step = self.completed + 1
        return (step * self.material_rate + self.optical_rate - 1) // self.optical_rate

    def complete(self, tick: int) -> None:
        if self.completed >= self.target:
            raise ValueError("unscheduled")
        if tick != self.next_tick():
            raise ValueError("wrong tick")
        self.completed += 1


def run(trials: int, seed: int) -> dict[str, int]:
    rng = random.Random(seed)
    maximum_pending = 0
    maximum_batches = 0
    for _ in range(trials):
        material = rng.randint(8_000, 384_000)
        optical = rng.randint(1, min(240, material))
        batch = rng.randint(1, 64)
        horizon = rng.randint(0, material * 3)
        cuts = sorted({0, horizon, *(rng.randint(0, horizon) for _ in range(rng.randint(0, 12)))})
        clock = Clock(material, optical, batch)
        batches = 0
        previous_step_tick = 0
        for target in cuts[1:]:
            clock.schedule(target)
            while clock.completed < clock.target:
                due = min(clock.target - clock.completed, clock.batch)
                assert due <= batch
                maximum_pending = max(maximum_pending, clock.target - clock.completed)
                batches += 1
                for _ in range(due):
                    tick = clock.next_tick()
                    assert tick > previous_step_tick
                    before = clock.completed
                    try:
                        clock.complete(tick + 1)
                    except ValueError:
                        pass
                    else:
                        raise AssertionError("wrong tick completed")
                    assert clock.completed == before
                    clock.complete(tick)
                    previous_step_tick = tick
        expected = horizon * optical // material
        assert clock.completed == expected
        assert clock.target == expected
        assert clock.completed <= horizon
        maximum_batches = max(maximum_batches, batches)

        single = Clock(material, optical, max(1, expected + 1))
        single.schedule(horizon)
        while single.completed < single.target:
            single.complete(single.next_tick())
        assert single.completed == clock.completed
    return {
        "trials": trials,
        "seed": seed,
        "maximum_pending_steps": maximum_pending,
        "maximum_batches": maximum_batches,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--trials", type=int, default=10_000)
    parser.add_argument("--seed", type=int, default=0xA71AD0E)
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    receipt = run(max(1, args.trials), args.seed)
    if args.json:
        print(json.dumps(receipt, sort_keys=True))
    else:
        print("Ariadne optical-clock oracle passed", receipt)


if __name__ == "__main__":
    main()
