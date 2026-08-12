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
            "mus-cli-trajectory-lifetime-{}-{nanos}-{id}",
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

fn score(events_bar_1: &str, couple: &str) -> String {
    format!(
        "# score: score-spanning trajectory lifetime\n\
# tempo: 120\n\
# time: 4/4\n\
# key: C\n\
# bars: 2\n\
# instruments:\n\
#   w = weave (treble, synth=weave, courses=3, strum=0, couple={couple}, body=0.25, sus=0.20, traj_curve=smootherstep, traj_hz=200, body_lifetime=track)\n\
\n\
bar 1: w={events_bar_1}\n\
bar 2: w=A3h Rh\n"
    )
}

fn run_render(root: &Path, source: &str, stem: &str) -> std::process::Output {
    let input = root.join(format!("{stem}.mus"));
    let output = root.join(format!("{stem}.wav"));
    fs::write(&input, source).expect("write score");
    Command::new(env!("CARGO_BIN_EXE_mus"))
        .args(["render", input.to_str().expect("score path"), "-o"])
        .arg(&output)
        .arg("--json")
        .output()
        .expect("run mus render")
}

fn success_json(root: &Path, source: &str, stem: &str) -> serde_json::Value {
    let result = run_render(root, source, stem);
    assert!(
        result.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).expect("receipt json")
}

#[test]
fn one_track_body_receives_three_attacks_across_note_and_bar_boundaries() {
    let root = TempDir::new();
    let source = score(
        "A3q[pan=-0.25->0.25] Rq A3q Rq",
        "0.10@0->0.34@0.52->0.16@1",
    );
    let first = success_json(root.path(), &source, "first");
    let second = success_json(root.path(), &source, "second");

    assert_eq!(first["eventCount"].as_u64(), Some(3), "{first}");
    assert_eq!(first["perTrackEvents"]["w"].as_u64(), Some(3));
    assert_eq!(first["renderDigest"], second["renderDigest"]);
    assert_eq!(
        first["trajectoryReceipts"], second["trajectoryReceipts"],
        "lifetime receipt identity must be deterministic"
    );

    let receipts = first["trajectoryReceipts"]
        .as_array()
        .expect("trajectory receipts");
    assert_eq!(receipts.len(), 1);
    let event = &receipts[0];
    assert_eq!(event["bar"].as_u64(), Some(1));
    assert_eq!(event["track"].as_str(), Some("w"));
    assert_eq!(event["onsetSeconds"].as_f64(), Some(0.0));
    assert_eq!(event["slotSeconds"].as_f64(), Some(4.0));
    assert_eq!(
        event["receipt"]["trajectory"]["claim_level"].as_str(),
        Some("score-spanning-preallocated-rearticulation")
    );
    assert_eq!(
        event["receipt"]["trajectory"]["evidence_class"].as_str(),
        Some(
            "validated-numerics+engine-execution+single-construction+preallocated-attacks+network-observer-off-specialization"
        )
    );
    assert!(
        event["receipt"]["unresolved_obligations"]
            .as_array()
            .expect("obligations")
            .iter()
            .any(|item| item.as_str() == Some("rearticulation-excitation-work-ledger")),
        "{event}"
    );
}

#[test]
fn track_lifetime_refuses_pitch_topology_changes() {
    let root = TempDir::new();
    let result = run_render(
        root.path(),
        &score("A3q B3q Rh", "0.10@0->0.30@1"),
        "topology-change",
    );
    assert!(!result.status.success());
    let error: serde_json::Value =
        serde_json::from_slice(&result.stdout).expect("structured render error");
    assert!(
        error["error"]
            .as_str()
            .expect("error text")
            .contains("fixed course topology"),
        "{error}"
    );
}

#[test]
fn track_lifetime_refuses_a_scalar_only_patch() {
    let root = TempDir::new();
    let result = run_render(root.path(), &score("A3w", "0.10"), "no-trajectory");
    assert!(!result.status.success());
    let error: serde_json::Value =
        serde_json::from_slice(&result.stdout).expect("structured render error");
    assert!(
        error["error"]
            .as_str()
            .expect("error text")
            .contains("requires at least one registered arrow-valued physical trajectory"),
        "{error}"
    );
}
