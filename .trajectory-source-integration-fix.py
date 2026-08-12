from pathlib import Path

path = Path("mus-rs/crates/mus-engine/src/source.rs")
text = path.read_text(encoding="utf-8")
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
path.write_text(text.replace(old, new, 1), encoding="utf-8")
