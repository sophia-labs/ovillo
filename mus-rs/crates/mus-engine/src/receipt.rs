//! `mus.audio.render-receipt.v1`, the receipt `mus render --json` prints.
//! It lives here, not in `mus-cli`, so the browser renderer (`mus-wasm`)
//! prints the same receipt from the same code.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::{
    sha256_bytes, AriadneReceipt, EventTrajectoryReceipt, RenderOutcome, RenderStats,
    ENGINE_VERSION,
};

/// `mus.audio.render-receipt.v1`: peak/RMS/digests (the original WF1
/// surface, kept for `mus-cli/tests/render_receipt.rs`) extended per the
/// WF8 spec with the fields `mus-rs/tools/parity_render.py` (WF9) diffs
/// first — event/skip counts, per-track counts, the bad-token list, and
/// the sidechain hit count.
#[derive(Debug, Serialize)]
pub struct RenderReceipt {
    pub schema: &'static str,
    #[serde(rename = "renderDigest")]
    pub render_digest: String,
    #[serde(rename = "sourceDigest")]
    pub source_digest: String,
    #[serde(rename = "logDigest")]
    pub log_digest: Option<String>,
    #[serde(rename = "headSelection")]
    pub head_selection: serde_json::Value,
    #[serde(rename = "engineVersion")]
    pub engine_version: &'static str,
    #[serde(rename = "peakDbfs")]
    pub peak_dbfs: f32,
    #[serde(rename = "rmsDbfs")]
    pub rms_dbfs: f32,
    #[serde(rename = "wallSeconds")]
    pub wall_seconds: f64,
    #[serde(rename = "eventCount")]
    pub event_count: u32,
    #[serde(rename = "skippedCount")]
    pub skipped_count: u32,
    #[serde(rename = "voiceCount")]
    pub voice_count: usize,
    #[serde(rename = "durationSeconds")]
    pub duration_seconds: f64,
    #[serde(rename = "tuningA")]
    pub tuning_a: f64,
    #[serde(rename = "sidechainHits")]
    pub sidechain_hits: usize,
    #[serde(rename = "perTrackEvents")]
    pub per_track_events: BTreeMap<String, u32>,
    #[serde(rename = "badTokens")]
    pub bad_tokens: Vec<String>,
    #[serde(rename = "warnings")]
    pub warnings: Vec<String>,
    #[serde(rename = "trajectoryReceipts", skip_serializing_if = "Vec::is_empty")]
    pub trajectory_receipts: Vec<EventTrajectoryReceipt>,
    /// `--stems DIR` only: abbrev → written mono stem path (dry,
    /// pre-send, no reverb/master — the spatial scene's raw material).
    #[serde(rename = "stems", skip_serializing_if = "Option::is_none")]
    pub stems: Option<BTreeMap<String, String>>,
    /// Instruments played on Ariadne worlds (annotated tab, piano): plan, touches, ledger.
    #[serde(rename = "ariadneWorlds", skip_serializing_if = "Vec::is_empty")]
    pub ariadne_worlds: Vec<AriadneReceipt>,
}

impl RenderReceipt {
    pub fn from_stats(
        stats: &RenderStats,
        render_digest: String,
        source_digest: String,
        head_selection: serde_json::Value,
        peak_dbfs: f32,
        rms_dbfs: f32,
        wall_seconds: f64,
    ) -> Self {
        RenderReceipt {
            schema: "mus.audio.render-receipt.v1",
            render_digest,
            source_digest,
            log_digest: None,
            head_selection,
            engine_version: ENGINE_VERSION,
            peak_dbfs,
            rms_dbfs,
            wall_seconds,
            event_count: stats.notes,
            skipped_count: stats.skipped,
            voice_count: stats.voices,
            duration_seconds: stats.total_seconds,
            tuning_a: stats.tuning_a,
            sidechain_hits: stats.sidechain_hits,
            per_track_events: stats.per_track_events.clone(),
            bad_tokens: stats.bad.clone(),
            warnings: stats.warnings.clone(),
            trajectory_receipts: stats.trajectory_receipts.clone(),
            stems: None,
            ariadne_worlds: stats.ariadne_worlds.clone(),
        }
    }

    /// The receipt for a finished render whose WAV bytes are `wav`.
    pub fn for_render(
        source: &str,
        outcome: &RenderOutcome,
        wav: &[u8],
        wall_seconds: f64,
    ) -> Self {
        Self::from_stats(
            &outcome.stats,
            format!("sha256:{}", sha256_bytes(wav)),
            format!("sha256:{}", sha256_bytes(source.as_bytes())),
            head_selection(source),
            outcome.audio.peak_dbfs(),
            outcome.audio.rms_dbfs(),
            wall_seconds,
        )
    }
}

/// Which head each contested lineage resolved to. Parsed independently of
/// the render's own parse: this is the graph-shaped view `headSelection`
/// needs (a CRDT concept the render orchestrator itself has no reason to
/// carry). A freshly-parsed single-source score is never contested, so this
/// is cheap and always resolves to `"sole"` in practice.
pub fn head_selection(source: &str) -> serde_json::Value {
    let Ok(proposal) = mus_text::parse_score(source) else {
        return serde_json::Value::String("sole".into());
    };
    let graph = mus_text::adopt(&proposal);
    if graph.contested_lineages().is_empty() {
        return serde_json::Value::String("sole".into());
    }
    let mut selected = serde_json::Map::new();
    for (lineage, state) in &graph.lineages {
        if state.contested {
            if let Some(head) = state.chosen_head() {
                selected.insert(
                    lineage.0.clone(),
                    serde_json::Value::String(head.version.0.clone()),
                );
            }
        }
    }
    serde_json::Value::Object(selected)
}

/// `mus.ariadne.energy-trace.v1`, the file `mus render --energy` writes.
pub fn energy_trace_json(stats: &RenderStats) -> serde_json::Value {
    serde_json::json!({"schema": "mus.ariadne.energy-trace.v1", "worlds": stats.ariadne_energy})
}
