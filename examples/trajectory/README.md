# Persistent-body trajectory examples

`string-breathing.json` is a two-second Weave path through coupling, body,
curvature, orbit rate, and the narrow native pitch subset. Compile it in Python:

```bash
python - <<'PY'
from pathlib import Path
from mus_analysis.trajectory import TrajectoryProgram, lower_string_trajectory
p = TrajectoryProgram.from_json(Path("examples/trajectory/string-breathing.json").read_text())
print(lower_string_trajectory(p, [110.0, 165.0], 2.0).to_dict())
PY
```

Render the equivalent live Rust etude:

```bash
cargo run --release -p mus-dsp --example trajectory_etude -- trajectory-etude.wav
```

`garganta-ba.json` is a 300 ms `/ba/` control oracle. It moves the existing
speaker-static area map, includes a declared near-closure section override, and
keeps source controls separate. It does not claim the still-open G1/G2/G3 laws:

```bash
python - <<'PY'
from pathlib import Path
from mus_analysis.garganta_anatomy import AnatomyGenome
from mus_analysis.trajectory import GargantaTrajectoryAdapter, TrajectoryProgram
p = TrajectoryProgram.from_json(Path("examples/trajectory/garganta-ba.json").read_text())
anatomy = AnatomyGenome.from_unit_cube([0.5] * 10)
states, receipt = GargantaTrajectoryAdapter(p).compile_states(anatomy)
print(receipt.to_dict())
print(states[-1])
PY
```
