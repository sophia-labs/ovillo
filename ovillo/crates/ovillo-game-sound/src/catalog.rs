use crate::GameWorld;
pub fn catalog() -> serde_json::Value { serde_json::json!([]) }
pub fn create(kind: &str, fs: f64, _seed: u64) -> Option<Box<dyn GameWorld>> {
    match kind {
        "kart_engine" => Some(Box::new(crate::kart_engine::KartEngine::with_seed(fs, _seed))),
        "race_props" => Some(Box::new(crate::race_props::RaceProps::new(fs))),
        "f16" => Some(Box::new(crate::f16::F16::new(fs, _seed))),
        _ => None,
    }
}
