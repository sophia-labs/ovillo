#!/usr/bin/env python3
"""Reproducible fictional Ariadne anatomies; all changes are design hypotheses.

Uses the canonical steel-string plan as a structural template, not as evidence
that the resulting instruments reproduce any manufactured instrument.
"""
from copy import deepcopy
import hashlib
import json
import math
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
BASE = ROOT / "profiles/ariadne-steel-string-v0.plan.json"

# Course pitches are low to high; these are authored tuning choices.
RECIPES = {
    "sunwire": dict(
        tuning=[50, 50, 55, 55, 59, 59, 64, 64],
        cents=[-1.7, 1.7] * 4, inharmonicity=0.000035,
        loss_constant=0.75, loss_frequency=8.0e-8,
        resonator_frequency_scale=1.27, resonator_decay_scale=1.35,
        bridge_rate_scale=1.08, room_length_scale=0.83, position=-0.48,
        intention="Paired-course melodic voice, with separately evolving unisons.",
    ),
    "tumbler": dict(
        tuning=[28, 33, 38, 43], cents=[0.0] * 4,
        inharmonicity=0.00009, loss_constant=1.35, loss_frequency=1.0e-7,
        resonator_frequency_scale=0.57, resonator_decay_scale=0.8,
        bridge_rate_scale=1.10, room_length_scale=0.68, position=-0.08,
        intention="Low-register pulse and melodic bass, with palm loss in the score.",
    ),
    "prism": dict(
        tuning=[48, 53, 57, 60, 64, 67], cents=[-0.8, 0.5, -0.4, 0.7, -0.5, 0.8],
        inharmonicity=0.00018, loss_constant=1.6, loss_frequency=5.0e-8,
        resonator_frequency_scale=1.65, resonator_decay_scale=1.55,
        bridge_rate_scale=1.32, room_length_scale=0.64, position=0.65,
        intention="Plucked chord replies with an independently designed modal spectrum.",
    ),
}


def build():
    baseline_bytes = BASE.read_bytes()
    baseline = json.loads(baseline_bytes)
    strings = [b for b in baseline["bodies"] if b["id"].startswith("string/")]
    boxes = [b for b in baseline["bodies"] if b["id"].startswith("box/")]
    for name, recipe in RECIPES.items():
        plan = deepcopy(baseline)
        plan["world_name"] = f"sunskip-{name}-v1"
        plan["bodies"] = []
        plan["direct_couplings"] = []
        source_receipts = []
        # Reverse into MUS string numbers, highest course first.
        for number, (midi, cents) in enumerate(
            reversed(list(zip(recipe["tuning"], recipe["cents"]))), 1
        ):
            body = deepcopy(strings[0])
            body["id"] = f"string/{number}"
            body["body"]["anatomy_name"] = f"{name}-course-{number}"
            path = body["body"]["configuration_path"]
            f1 = 440.0 * 2.0 ** ((midi - 69 + cents / 100) / 12)
            b = recipe["inharmonicity"]
            for endpoint, multiplier in (("left", 1.0), ("right", 2.0)):
                metric = path[endpoint]["metric"]
                frequencies = [
                    f1 * multiplier * n * math.sqrt((1 + b*n*n) / (1 + b))
                    for n in range(1, 25)
                ]
                assert max(frequencies) < 21600, (name, number, frequencies[-1])
                metric["frequency_hz"] = frequencies
                metric["t60_seconds"] = [
                    6.91 / (recipe["loss_constant"] + recipe["loss_frequency"]*(2*math.pi*f)**2)
                    for f in frequencies
                ]
            body["pose"]["position"][0] = recipe["position"] + (number % 2 - 0.5)*0.09
            plan["bodies"].append(body)
            source_receipts.append({"string": number, "nominalMidi": midi,
                                    "designedCents": cents, "firstModeHz": f1})
            for band in range(3):
                edge = deepcopy(next(e for e in baseline["direct_couplings"]
                                     if e["id"] == f"bridge/6/{band}"))
                edge["id"] = f"bridge/{number}/{band}"
                edge["left_body"] = body["id"]
                edge["rate_hz"] *= recipe["bridge_rate_scale"]
                # Paired courses share a reciprocal resonator, with small
                # unequal coupling strengths as an explicit design choice.
                if name == "sunwire":
                    edge["weight"] *= 0.88 if number % 2 else 0.94
                plan["direct_couplings"].append(edge)
        for original in boxes:
            body = deepcopy(original)
            body["body"]["anatomy_name"] = f"{name}-{body['id']}"
            body["pose"]["position"][0] = recipe["position"]
            for endpoint in ("left", "right"):
                metric = body["body"]["configuration_path"][endpoint]["metric"]
                metric["frequency_hz"] = [f*recipe["resonator_frequency_scale"]
                                          for f in metric["frequency_hz"]]
                metric["t60_seconds"] = [t*recipe["resonator_decay_scale"]
                                         for t in metric["t60_seconds"]]
            plan["bodies"].append(body)
        plan["shared_space"]["delays_seconds"] = [
            d*recipe["room_length_scale"] for d in baseline["shared_space"]["delays_seconds"]
        ]
        path = HERE / f"{name}.plan.json"
        path.write_text(json.dumps(plan, indent=2) + "\n")
        receipt = {
            "schema": "sunskip.instrument-design.v1",
            "name": name, "classification": "design-hypothesis",
            "baseline": str(BASE.relative_to(ROOT)),
            "baselineFileSha256": hashlib.sha256(baseline_bytes).hexdigest(),
            "planFileSha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            "recipe": recipe, "courses": source_receipts,
            "partialRule": "f_n = f1*n*sqrt((1+B*n*n)/(1+B)); 24 modes per course",
            "lossRule": "T60 = 6.91/(loss_constant + loss_frequency*(2*pi*f)^2)",
            "limits": [
                "Fictional modal design; no real-instrument reconstruction claim.",
                "All inherited steel-string v0 playing limitations still apply.",
                "Declared mode tuning is not a measurement of the coupled sounding pitch.",
                "MUS hands overwrite source direction and contact frequency/damping.",
                "Each instrument is its own canonical World; the ensemble is a studio mix.",
            ],
        }
        (HERE / f"{name}.design.json").write_text(json.dumps(receipt, indent=2) + "\n")
        print(f"{name}: {len(plan['bodies'])} persistent bodies, {len(plan['direct_couplings'])} reciprocal edges")


if __name__ == "__main__":
    build()
