use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn new() -> Self {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        path.push(format!(
            "mus-cli-trajectory-receipt-{}-{nanos}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create temp directory");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn write_score(root: &Path, event_text: &str) {
    fs::write(
        root.join("trajectory.mus"),
        format!(
            "# score: trajectory receipt\n\
# tempo: 120\n\
# time: 4/4\n\
# key: C\n\
# bars: 1\n\
# instruments:\n\
#   w = weave (treble, synth=weave, courses=3, couple=0.10, body=0.25, sus=0.4)\n\
\n\
bar 1: w={event_text}\n"
        ),
    )
    .expect("write score");
}

fn render(root: &Path, stem: &str) -> serde_json::Value {
    let input = root.join("trajectory.mus");
    let output = root.join(format!("{stem}.wav"));
    let result = Command::new(env!("CARGO_BIN_EXE_mus"))
        .args(["render", input.to_str().expect("score path"), "-o"])
        .arg(&output)
        .arg("--json")
        .output()
        .expect("run mus render");
    assert!(
        result.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(output.exists(), "rendered wave exists");
    serde_json::from_slice(&result.stdout).expect("receipt json")
}

#[test]
fn ordinary_render_emits_one_deterministic_trajectory_receipt() {
    let root = TempDir::new();
    write_score(
        root.path(),
        "A3q[couple=0.10@0->0.30@1,traj_curve=smoothstep,traj_hz=200] \
         A3q[pan=-1->1,lpf=800->2400] Rh",
    );

    let first = render(root.path(), "first");
    let second = render(root.path(), "second");
    let first_receipts = first["trajectoryReceipts"]
        .as_array()
        .expect("trajectory receipts array");
    let second_receipts = second["trajectoryReceipts"]
        .as_array()
        .expect("trajectory receipts array");
    assert_eq!(first_receipts.len(), 1, "{first}");
    assert_eq!(
        first_receipts, second_receipts,
        "path and placement receipts are deterministic"
    );

    let event = &first_receipts[0];
    assert_eq!(event["bar"].as_u64(), Some(1));
    assert_eq!(event["track"].as_str(), Some("w"));
    assert_eq!(event["onsetSeconds"].as_f64(), Some(0.0));
    assert_eq!(event["slotSeconds"].as_f64(), Some(0.5));
    assert_eq!(
        event["receipt"]["adapter_version"].as_str(),
        Some("mus-dsp-string-trajectory/0.1.0")
    );
    assert!(
        event["receipt"]["control_frame_count"]
            .as_u64()
            .expect("control frame count")
            > 0
    );
    assert!(event["receipt"]["trajectory"]["program_digest"]
        .as_str()
        .expect("program digest")
        .starts_with("sha256:"));
    assert_eq!(
        first["eventCount"].as_u64(),
        Some(2),
        "the unrelated MUS-A arrow event still renders normally"
    );
}

#[test]
fn scalar_render_structurally_omits_the_trajectory_surface() {
    let root = TempDir::new();
    write_score(root.path(), "A3q[couple=0.20] Rq Rh");
    let receipt = render(root.path(), "scalar");
    assert!(
        receipt.get("trajectoryReceipts").is_none(),
        "empty trajectory metadata must be structurally absent: {receipt}"
    );
}

#[test]
fn native_pitch_trajectory_reaches_the_persistent_retune_path() {
    let root = TempDir::new();
    write_score(
        root.path(),
        "A3q[pitch_ratio=1@0->1.0594630943592953@1,traj_hz=200] Rq Rh",
    );
    let receipt = render(root.path(), "pitch");
    let event = &receipt["trajectoryReceipts"][0];
    assert_eq!(
        event["receipt"]["native_pitch_transport"].as_str(),
        Some("persistent-linear-f0-to-f1")
    );
    assert_eq!(event["receipt"]["control_frame_count"].as_u64(), Some(0));
    assert!(
        (event["receipt"]["pitch_end_ratio"]
            .as_f64()
            .expect("pitch end ratio")
            - 1.0594630943592953)
            .abs()
            < 1e-14
    );
}
