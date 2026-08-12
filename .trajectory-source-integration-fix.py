from pathlib import Path

source_path = Path("mus-rs/crates/mus-engine/src/source.rs")
text = source_path.read_text(encoding="utf-8")
old = '''        if let Some(plan) = trajectory_plan.filter(|plan| !plan.program.lanes.is_empty()) {
'''
new = '''        if let Some(plan) = trajectory_plan
            .as_ref()
            .filter(|plan| !plan.program.lanes.is_empty())
        {
'''
if text.count(old) != 1:
    raise SystemExit(f"expected one trajectory-plan ownership site, found {text.count(old)}")
text = text.replace(old, new, 1)
old = '''            x = StringTrajectoryVoice::new(base_voice, plan.program)
'''
new = '''            x = StringTrajectoryVoice::new(base_voice, plan.program.clone())
'''
if text.count(old) != 1:
    raise SystemExit(f"expected one trajectory program move, found {text.count(old)}")
source_path.write_text(text.replace(old, new, 1), encoding="utf-8")

# The adapter emits lanes in its stable target-registry order, independent of
# the textual ordering of event parameters. Assert that contract explicitly.
test_path = Path("mus-rs/crates/mus-engine/tests/trajectory_source.rs")
test = test_path.read_text(encoding="utf-8")
old = '''        ["couple", "body", "curvature"]
'''
new = '''        ["couple", "curvature", "body"]
'''
if test.count(old) != 1:
    raise SystemExit(f"expected one trajectory lane-order assertion, found {test.count(old)}")
test_path.write_text(test.replace(old, new, 1), encoding="utf-8")
