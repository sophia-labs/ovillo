//! A choir of tract Worlds: every singer is its own `TractWorldNd`, each with
//! its own anatomy, driven from a voice list. Nothing is synthesized: each
//! singer's lungs pay for every sample its glottis valve lets through, and
//! what is heard is each World's own radiation. The choir is a listener's mix
//! of those radiations (one fixed gain and pan per singer); the singers do not
//! share a room or hear one another.
//!
//! ```text
//! cargo run --release -p ovillo-tract-world --example tract_choir -- --calibrate <singers.tsv> <skill-dir>
//! cargo run --release -p ovillo-tract-world --example tract_choir -- <singers.tsv> <voices.tsv> <skill-dir> <out-dir> <stem>
//! ```
//!
//! **Singers** (`singers.tsv`, tab separated, a header line names the columns):
//! `singer voice tract_scale fold_scale vib_rate_hz wobble timing_ms
//! timing_jitter_ms intonation_cents intonation_jitter_cents breath_scale
//! tongue_offset lips_offset pan gain_db`.
//!
//! - `voice` is one of the anatomies in [`VOICES`]: `bass`, `baritone`,
//!   `tenor`, `alto`, `soprano`, `child`. Each starts from
//!   `TractNdAnatomy::public_v1()` and changes only declared anatomy fields:
//!   the oral and nasal tract lengths, the fold's length, thickness and mass,
//!   its tuning range, and the lungs. The singer's `tract_scale` multiplies the
//!   tract lengths; `fold_scale` multiplies the fold's length and thickness, and
//!   its mass by `fold_scale^3`. All are design hypotheses (see [`VOICES`]).
//!   The area function (cross-sections) is the adult's for every singer: the
//!   anatomy has no area scale.
//! - Between vowels a singer's target articulation moves over 0.2 s
//!   (`ART_GLIDE_S`), unless the note's optional `glide_s` column says how long
//!   (a short glide out of a hum's sealed lips is the choir's "b"/"d": the
//!   lips are released fast, and the World's articulators follow at their own
//!   speed, every move booked).
//! - The rest is the singer's own way of singing, all World controls: a
//!   constant lateness (`timing_ms`) and a per-note scatter
//!   (`timing_jitter_ms`); a constant intonation offset and a per-note scatter
//!   (cents; the pitch the singer aims for); vibrato rate and wobble (the
//!   World's booked fold retuning); a breath scale (the valve's supply); and a
//!   personal vowel shape, offsets to the vowel's tongue index and lips. The
//!   per-note scatters are deterministic (a hash of singer and note index), so
//!   a render repeats bit for bit.
//! - `pan` (-1..1) and `gain_db` are the listener's mix, outside every World.
//!
//! **Voices** (`voices.tsv`): `singer start_s end_s midi vowel breath vib_depth`,
//! one line per written note, each singer's notes in time order. `vowel` is
//! `a`, `i`, `u` (`PUBLIC_ART_*`, fitted for the adult public tract), `m`
//! (`PUBLIC_ART_M`, a hum), or `o`, the midpoint of `a` and `u` in
//! articulation space, **not fitted** to any formant target. On a shorter
//! tract the same articulation gives higher formants; that is the anatomy.
//!
//! **The singer's skill.** The fold tuning is not the pitch the World sounds:
//! the fold, the tract's air and the jet pull the cycle off it. As the
//! trombone player learned its cells by playing its World (`TROMBONE-WORLD.md`),
//! `--calibrate` plays each singer's own World: for each vowel the singer uses,
//! it holds every fold tuning in the voice's range in quarter-tone steps for
//! 1.2 s at breath 0.55 and reads the fold's own cycle rate
//! (`Observers::measured_f0_hz`, a non-loading observer) over the last 0.5 s.
//! The table it writes (`<skill-dir>/<singer>.tsv`) is that singer's knowledge:
//! to sing a pitch, the singer asks which tuning sounded it on that vowel
//! (log-interpolated between the two calibrated tunings around it). Pitches
//! outside what the calibration heard are refused (an error), not clamped.
//!
//! Writes `<stem>.wav` (stereo f32 48 kHz: the mix, unnormalized), and
//! `<stem>-receipt.json` with every singer's anatomy and World books.

use mus_ariadne::{ConnectionPolicy, DisposalPolicy};
use ovillo_tract_world::world::{self as tw, Articulation, TractControls, TractWorldNd, Vibrato};
use ovillo_tract_world::TractNdAnatomy;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const SR: f64 = 48_000.0;
const TAIL_S: f64 = 1.5;
/// A singer moves from one vowel to the next over this long: the target
/// articulation is ramped, and the World's articulators follow it at their own
/// speed, every move booked. (A jumped target is reached in about 25 ms; from a
/// hum's closed lips that releases like a plosive.)
const ART_GLIDE_S: f64 = 0.20;

fn mix_art(a: Articulation, b: Articulation, k: f64) -> Articulation {
    let k = k.clamp(0.0, 1.0);
    let l = |x: f64, y: f64| x + (y - x) * k;
    Articulation {
        tongue_index: l(a.tongue_index, b.tongue_index),
        tongue_diameter: l(a.tongue_diameter, b.tongue_diameter),
        lips: l(a.lips, b.lips),
        velum: l(a.velum, b.velum),
    }
}

/// A voice type: lengths in metres, fold mass in kg, tuning range in Hz, lungs
/// in J and W. `adult_v0` (the public tract) is oral 0.157, nasal 0.11, fold
/// 14 x 3 mm, 0.15 g, 60-420 Hz, lungs 3 J / 0.3 W. The others are design
/// hypotheses scaled from it in the directions commonly reported: female and
/// child vocal tracts shorter than adult male ones, shorter and lighter vocal
/// folds for higher voices. None of these numbers is a measurement.
pub struct VoiceType {
    pub name: &'static str,
    pub oral_m: f64,
    pub nasal_m: f64,
    pub fold_length_m: f64,
    pub fold_thickness_m: f64,
    pub fold_mass_kg: f64,
    pub min_tuning_hz: f64,
    pub max_tuning_hz: f64,
    pub lung_capacity_j: f64,
    pub lung_refill_w: f64,
}

pub static VOICES: [VoiceType; 6] = [
    VoiceType {
        name: "bass",
        oral_m: 0.170,
        nasal_m: 0.118,
        fold_length_m: 0.016,
        fold_thickness_m: 0.0033,
        fold_mass_kg: 1.8e-4,
        min_tuning_hz: 45.0,
        max_tuning_hz: 340.0,
        lung_capacity_j: 3.4,
        lung_refill_w: 0.32,
    },
    VoiceType {
        name: "baritone",
        oral_m: 0.163,
        nasal_m: 0.114,
        fold_length_m: 0.015,
        fold_thickness_m: 0.0031,
        fold_mass_kg: 1.6e-4,
        min_tuning_hz: 55.0,
        max_tuning_hz: 400.0,
        lung_capacity_j: 3.2,
        lung_refill_w: 0.30,
    },
    VoiceType {
        name: "tenor",
        oral_m: 0.155,
        nasal_m: 0.108,
        fold_length_m: 0.013,
        fold_thickness_m: 0.0029,
        fold_mass_kg: 1.3e-4,
        min_tuning_hz: 70.0,
        max_tuning_hz: 520.0,
        lung_capacity_j: 3.0,
        lung_refill_w: 0.30,
    },
    VoiceType {
        name: "alto",
        oral_m: 0.142,
        nasal_m: 0.099,
        fold_length_m: 0.011,
        fold_thickness_m: 0.0026,
        fold_mass_kg: 0.95e-4,
        min_tuning_hz: 110.0,
        max_tuning_hz: 760.0,
        lung_capacity_j: 2.4,
        lung_refill_w: 0.25,
    },
    VoiceType {
        name: "soprano",
        oral_m: 0.136,
        nasal_m: 0.095,
        fold_length_m: 0.0095,
        fold_thickness_m: 0.0024,
        fold_mass_kg: 0.75e-4,
        min_tuning_hz: 150.0,
        max_tuning_hz: 1100.0,
        lung_capacity_j: 2.2,
        lung_refill_w: 0.24,
    },
    VoiceType {
        name: "child",
        oral_m: 0.118,
        nasal_m: 0.082,
        fold_length_m: 0.0075,
        fold_thickness_m: 0.0021,
        fold_mass_kg: 0.5e-4,
        min_tuning_hz: 170.0,
        max_tuning_hz: 1000.0,
        lung_capacity_j: 1.4,
        lung_refill_w: 0.16,
    },
];

#[derive(Clone)]
struct Singer {
    name: String,
    voice: &'static VoiceType,
    tract_scale: f64,
    fold_scale: f64,
    vib_rate_hz: f64,
    wobble: f64,
    timing_ms: f64,
    timing_jitter_ms: f64,
    intonation_cents: f64,
    intonation_jitter_cents: f64,
    breath_scale: f64,
    tongue_offset: f64,
    lips_offset: f64,
    pan: f64,
    gain_db: f64,
}

impl Singer {
    fn anatomy(&self) -> TractNdAnatomy {
        let v = self.voice;
        let mut a = TractNdAnatomy::public_v1();
        a.oral_length_m = v.oral_m * self.tract_scale;
        a.nasal_length_m = v.nasal_m * self.tract_scale;
        a.valve.length_m = v.fold_length_m * self.fold_scale;
        a.valve.thickness_m = v.fold_thickness_m * self.fold_scale;
        a.valve.mass_kg = v.fold_mass_kg * self.fold_scale.powi(3);
        a.valve.min_tuning_hz = v.min_tuning_hz;
        a.valve.max_tuning_hz = v.max_tuning_hz;
        a.valve.initial_tuning_hz = (v.min_tuning_hz * v.max_tuning_hz).sqrt();
        a.lung_capacity_j = v.lung_capacity_j;
        a.lung_refill_w = v.lung_refill_w;
        a
    }

    /// This singer's own shape of a vowel.
    fn vowel(&self, c: char) -> Articulation {
        let mut a = vowel(c);
        a.tongue_index = (a.tongue_index + self.tongue_offset).clamp(0.0, 1.0);
        if c != 'm' {
            a.lips = (a.lips + self.lips_offset).clamp(0.0, 1.0);
        }
        a
    }

    fn anatomy_json(&self) -> Value {
        let a = self.anatomy();
        json!({
            "voice": self.voice.name, "tractScale": self.tract_scale, "foldScale": self.fold_scale,
            "oralLengthM": a.oral_length_m, "nasalLengthM": a.nasal_length_m,
            "foldLengthM": a.valve.length_m, "foldThicknessM": a.valve.thickness_m, "foldMassKg": a.valve.mass_kg,
            "tuningRangeHz": [a.valve.min_tuning_hz, a.valve.max_tuning_hz],
            "lungCapacityJ": a.lung_capacity_j, "lungRefillW": a.lung_refill_w,
            "areaFunction": "adult (public_v1), unscaled",
        })
    }
}

fn vowel(c: char) -> Articulation {
    let a = tw::PUBLIC_ART_A;
    let u = tw::PUBLIC_ART_U;
    match c {
        'i' => tw::PUBLIC_ART_I,
        'u' => u,
        'm' => tw::PUBLIC_ART_M,
        'o' => Articulation {
            tongue_index: 0.5 * (a.tongue_index + u.tongue_index),
            tongue_diameter: 0.5 * (a.tongue_diameter + u.tongue_diameter),
            lips: 0.5 * (a.lips + u.lips),
            velum: 0.5 * (a.velum + u.velum),
        },
        _ => a,
    }
}

/// A TSV with a header line, as rows of named fields.
fn table(path: &str) -> Result<Vec<BTreeMap<String, String>>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let mut lines = text
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'));
    let header: Vec<String> = lines
        .next()
        .ok_or(format!("{path}: empty"))?
        .split('\t')
        .map(|s| s.trim().to_owned())
        .collect();
    lines
        .enumerate()
        .map(|(i, l)| {
            let f: Vec<&str> = l.split('\t').map(str::trim).collect();
            if f.len() != header.len() {
                return Err(format!(
                    "{path}: row {} has {} fields, the header {}",
                    i + 1,
                    f.len(),
                    header.len()
                ));
            }
            Ok(header
                .iter()
                .cloned()
                .zip(f.iter().map(|s| (*s).to_owned()))
                .collect())
        })
        .collect()
}

fn num(row: &BTreeMap<String, String>, key: &str) -> Result<f64, String> {
    row.get(key)
        .ok_or(format!("missing column {key}"))?
        .parse::<f64>()
        .map_err(|_| format!("{key}={:?} is not a number", row[key]))
}

fn singers(path: &str) -> Result<BTreeMap<String, Singer>, String> {
    let mut out = BTreeMap::new();
    for row in table(path)? {
        let name = row.get("singer").ok_or("missing column singer")?.clone();
        let voice = VOICES
            .iter()
            .find(|v| Some(v.name) == row.get("voice").map(String::as_str))
            .ok_or(format!("{name}: unknown voice {:?}", row.get("voice")))?;
        let s = Singer {
            name: name.clone(),
            voice,
            tract_scale: num(&row, "tract_scale")?,
            fold_scale: num(&row, "fold_scale")?,
            vib_rate_hz: num(&row, "vib_rate_hz")?,
            wobble: num(&row, "wobble")?,
            timing_ms: num(&row, "timing_ms")?,
            timing_jitter_ms: num(&row, "timing_jitter_ms")?,
            intonation_cents: num(&row, "intonation_cents")?,
            intonation_jitter_cents: num(&row, "intonation_jitter_cents")?,
            breath_scale: num(&row, "breath_scale")?,
            tongue_offset: num(&row, "tongue_offset")?,
            lips_offset: num(&row, "lips_offset")?,
            pan: num(&row, "pan")?.clamp(-1.0, 1.0),
            gain_db: num(&row, "gain_db")?,
        };
        if !(0.8..=1.2).contains(&s.tract_scale) || !(0.8..=1.2).contains(&s.fold_scale) {
            return Err(format!(
                "{name}: tract_scale and fold_scale must lie in 0.8..1.2"
            ));
        }
        out.insert(name, s);
    }
    Ok(out)
}

#[derive(Clone, Copy)]
struct Sung {
    start_s: f64,
    end_s: f64,
    midi: f64,
    vowel: char,
    breath: f64,
    vib: f64,
    /// Seconds the target articulation takes to move from the previous vowel.
    glide_s: f64,
}

fn voices(path: &str) -> Result<BTreeMap<String, Vec<Sung>>, String> {
    let mut out: BTreeMap<String, Vec<Sung>> = BTreeMap::new();
    for row in table(path)? {
        let name = row.get("singer").ok_or("missing column singer")?.clone();
        let v = row
            .get("vowel")
            .and_then(|v| v.chars().next())
            .filter(|c| "aiumo".contains(*c))
            .ok_or(format!("{name}: vowel must be a|i|u|m|o"))?;
        let s = Sung {
            start_s: num(&row, "start_s")?,
            end_s: num(&row, "end_s")?,
            midi: num(&row, "midi")?,
            vowel: v,
            breath: num(&row, "breath")?.clamp(0.0, 1.0),
            vib: num(&row, "vib_depth")?.clamp(0.0, 2.0),
            glide_s: match row.get("glide_s") {
                Some(v) if !v.trim().is_empty() && v.trim() != "-" => num(&row, "glide_s")?.clamp(0.005, 2.0),
                _ => ART_GLIDE_S,
            },
        };
        if !(s.end_s > s.start_s) {
            return Err(format!("{name}: end_s must exceed start_s"));
        }
        let notes = out.entry(name.clone()).or_default();
        if notes.last().is_some_and(|p| s.start_s < p.start_s) {
            return Err(format!("{name}: notes must be in time order"));
        }
        notes.push(s);
    }
    Ok(out)
}

/// The calibration table, per vowel: (tuning_hz, sounding_hz), tuning ascending.
struct SkillTable(BTreeMap<char, Vec<(f64, f64)>>);

impl SkillTable {
    fn read(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut t: BTreeMap<char, Vec<(f64, f64)>> = BTreeMap::new();
        for line in text
            .lines()
            .filter(|l| !l.starts_with('#') && !l.starts_with("vowel") && !l.trim().is_empty())
        {
            let f: Vec<&str> = line.split('\t').collect();
            let (Some(v), Ok(tuning), Ok(sounding)) = (
                f[0].chars().next(),
                f[1].parse::<f64>(),
                f[2].parse::<f64>(),
            ) else {
                return Err(format!("{}: bad line {line:?}", path.display()));
            };
            if sounding > 0.0 {
                t.entry(v).or_default().push((tuning, sounding));
            }
        }
        Ok(Self(t))
    }

    /// The fold tuning that sounded `hz` on this vowel: the first rising
    /// stretch of the table that brackets it, log-interpolated.
    fn tuning_for(&self, vowel: char, hz: f64) -> Result<f64, String> {
        let rows = self
            .0
            .get(&vowel)
            .ok_or_else(|| format!("no calibration for vowel {vowel}"))?;
        for w in rows.windows(2) {
            let ((t0, s0), (t1, s1)) = (w[0], w[1]);
            if s1 > s0 && s0 <= hz && hz <= s1 {
                let k = (hz.ln() - s0.ln()) / (s1.ln() - s0.ln());
                return Ok((t0.ln() + k * (t1.ln() - t0.ln())).exp());
            }
        }
        let (lo, hi) = self.range(vowel);
        Err(format!("{hz:.1} Hz on /{vowel}/ is outside what the calibration heard this singer sing ({lo:.1}-{hi:.1} Hz)"))
    }

    /// The sounding range the calibration heard (the rising stretches).
    fn range(&self, vowel: char) -> (f64, f64) {
        let rows = self.0.get(&vowel).map_or(&[][..], Vec::as_slice);
        let mut lo = f64::INFINITY;
        let mut hi = 0.0f64;
        for w in rows.windows(2) {
            if w[1].1 > w[0].1 {
                lo = lo.min(w[0].1);
                hi = hi.max(w[1].1);
            }
        }
        (lo, hi)
    }
}

fn world(s: &Singer, art: Articulation) -> TractWorldNd {
    TractWorldNd::with(
        SR,
        s.anatomy(),
        ConnectionPolicy::MassMomentum {
            disposal: DisposalPolicy::Dissipate,
        },
        art,
    )
    .expect("a tract World")
}

fn calibrate_one(s: &Singer, c: char) -> String {
    let art = s.vowel(c);
    let mut rows = String::new();
    let mut tuning = s.voice.min_tuning_hz;
    while tuning <= s.voice.max_tuning_hz * 1.0001 {
        let mut w = world(s, art);
        w.set_controls(TractControls {
            articulation: art,
            pitch_hz: tuning,
            tenseness: 0.6,
            breath: 0.0,
        });
        w.snap().unwrap();
        w.set_controls(TractControls {
            articulation: art,
            pitch_hz: tuning,
            tenseness: 0.6,
            breath: 0.55,
        });
        let n = (1.2 * SR) as usize;
        let mut f0s = Vec::new();
        let mut sq = 0.0;
        for i in 0..n {
            let y = w.tick().unwrap();
            if i >= (0.7 * SR) as usize {
                sq += y * y;
                if i % 240 == 0 {
                    f0s.push(w.observers().measured_f0_hz);
                }
            }
        }
        f0s.sort_by(f64::total_cmp);
        let f0 = f0s[f0s.len() / 2];
        let rms = (sq / (0.5 * SR)).sqrt();
        rows += &format!("{c}\t{tuning:.3}\t{f0:.3}\t{rms:.6e}\n");
        tuning *= 2f64.powf(1.0 / 24.0);
    }
    rows
}

/// Runs `jobs` on all cores, results in order.
fn parallel<T: Send + 'static>(jobs: Vec<Box<dyn FnOnce() -> T + Send>>) -> Vec<T> {
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let queue = std::sync::Arc::new(std::sync::Mutex::new(
        jobs.into_iter().enumerate().collect::<Vec<_>>(),
    ));
    let results = std::sync::Arc::new(std::sync::Mutex::new(BTreeMap::new()));
    let handles: Vec<_> = (0..threads)
        .map(|_| {
            let (q, r) = (queue.clone(), results.clone());
            std::thread::spawn(move || loop {
                let job = q.lock().unwrap().pop();
                let Some((i, f)) = job else { break };
                let out = f();
                r.lock().unwrap().insert(i, out);
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    let mut map = std::sync::Arc::try_unwrap(results)
        .ok()
        .unwrap()
        .into_inner()
        .unwrap();
    (0..map.len()).map(|i| map.remove(&i).unwrap()).collect()
}

fn calibrate(singers_path: &str, dir: &Path) {
    let singers = singers(singers_path).unwrap_or_else(|e| fail(&e));
    std::fs::create_dir_all(dir).unwrap();
    let mut jobs: Vec<Box<dyn FnOnce() -> (String, String) + Send>> = Vec::new();
    for s in singers.values() {
        for c in ['a', 'o', 'm'] {
            let s = s.clone();
            jobs.push(Box::new(move || (s.name.clone(), calibrate_one(&s, c))));
        }
    }
    let t0 = std::time::Instant::now();
    let mut tables: BTreeMap<String, String> = BTreeMap::new();
    for (name, rows) in parallel(jobs) {
        tables.entry(name).or_default().push_str(&rows);
    }
    for (name, rows) in &tables {
        let s = &singers[name];
        let head = format!(
            "# tract_choir --calibrate: singer {name}, voice {}, {}; MassMomentum/Dissipate, tenseness 0.6, breath 0.55, 1.2 s per tuning, median Observers::measured_f0_hz over the last 0.5 s\nvowel\ttuning_hz\tsounding_hz\trms\n",
            s.voice.name,
            s.anatomy_json()
        );
        std::fs::write(dir.join(format!("{name}.tsv")), head + rows).unwrap();
        let skill = SkillTable::read(&dir.join(format!("{name}.tsv"))).unwrap();
        let r = |c| {
            let (lo, hi) = skill.range(c);
            format!("/{c}/ {lo:.0}-{hi:.0} Hz")
        };
        println!(
            "{name:10} {:9} {} {} {}",
            s.voice.name,
            r('a'),
            r('o'),
            r('m')
        );
    }
    println!(
        "calibrated {} singers in {:.0} s wall",
        tables.len(),
        t0.elapsed().as_secs_f64()
    );
}

fn hz(midi: f64) -> f64 {
    440.0 * 2f64.powf((midi - 69.0) / 12.0)
}

/// A deterministic number in -1..1 from a singer's name, a note and a salt.
fn scatter(name: &str, note: usize, salt: u64) -> f64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325
        ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (note as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93);
    for b in name.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h ^= h >> 29;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 32;
    2.0 * ((h >> 11) as f64 / (1u64 << 53) as f64) - 1.0
}

/// The notes as this singer sings them: timing and intonation scattered.
struct Performed {
    start_s: f64,
    end_s: f64,
    target_hz: f64,
    tuning_hz: f64,
    art: Articulation,
    vowel: char,
    breath: f64,
    vib: f64,
    glide_s: f64,
}

fn perform(s: &Singer, notes: &[Sung], skill: &SkillTable) -> Result<Vec<Performed>, String> {
    let mut out: Vec<Performed> = Vec::new();
    for (k, n) in notes.iter().enumerate() {
        let dt = (s.timing_ms + s.timing_jitter_ms * scatter(&s.name, k, 1)) / 1000.0;
        let de = (s.timing_ms + s.timing_jitter_ms * scatter(&s.name, k, 2)) / 1000.0;
        let cents = s.intonation_cents + s.intonation_jitter_cents * scatter(&s.name, k, 3);
        let target = hz(n.midi + cents / 100.0);
        let mut start = (n.start_s + dt).max(0.0);
        if let Some(p) = out.last_mut() {
            // a singer is one voice: a scattered entry never overlaps the last note
            start = start.max(p.start_s + 0.01);
            p.end_s = p.end_s.min(start);
        }
        let end = (n.end_s + de).max(start + 0.05);
        out.push(Performed {
            start_s: start,
            end_s: end,
            target_hz: target,
            tuning_hz: skill.tuning_for(n.vowel, target)?,
            art: s.vowel(n.vowel),
            vowel: n.vowel,
            breath: (n.breath * s.breath_scale).clamp(0.0, 1.0),
            vib: n.vib,
            glide_s: n.glide_s,
        });
    }
    Ok(out)
}

fn sing(s: &Singer, notes: &[Performed], n: usize) -> (Vec<f64>, Value) {
    let first = &notes[0];
    let mut w = world(s, first.art);
    let mut controls = TractControls {
        articulation: first.art,
        pitch_hz: first.tuning_hz,
        tenseness: 0.6,
        breath: 0.0,
    };
    w.set_controls(controls);
    w.snap().expect("snap to the first vowel");
    let mut y = vec![0.0f64; n];
    let (mut peak, mut sq) = (0.0f64, 0.0f64);
    let mut k = 0usize;
    // The World is built at rest 0.5 s before the singer's first note (the
    // lungs refill meanwhile); before that the singer is silent.
    let begin = ((first.start_s - 0.5).max(0.0) * SR) as usize;
    for (i, out) in y.iter_mut().enumerate().skip(begin) {
        let t = i as f64 / SR;
        while k + 1 < notes.len() && notes[k + 1].start_s <= t {
            k += 1;
        }
        let note = &notes[k];
        if t >= note.start_s && t < note.end_s {
            let art = match k.checked_sub(1).map(|j| &notes[j]) {
                Some(prev) if prev.vowel != note.vowel => {
                    mix_art(prev.art, note.art, (t - note.start_s) / note.glide_s)
                }
                _ => note.art,
            };
            controls = TractControls {
                articulation: art,
                pitch_hz: note.tuning_hz,
                tenseness: 0.6,
                breath: note.breath,
            };
            w.set_vibrato(Vibrato {
                rate_hz: s.vib_rate_hz,
                depth_semitones: note.vib,
                wobble: s.wobble,
            });
        } else {
            controls.breath = 0.0;
        }
        w.set_controls(controls);
        let v = w.tick().expect("the World stepped");
        peak = peak.max(v.abs());
        sq += v * v;
        *out = v;
    }
    let r = w.world().receipt();
    let st = w.stats();
    let o = w.observers();
    let book = json!({
        "singer": s.name,
        "anatomy": s.anatomy_json(),
        "manner": {"vibRateHz": s.vib_rate_hz, "wobble": s.wobble, "timingMs": s.timing_ms, "timingJitterMs": s.timing_jitter_ms,
                   "intonationCents": s.intonation_cents, "intonationJitterCents": s.intonation_jitter_cents, "breathScale": s.breath_scale,
                   "tongueOffset": s.tongue_offset, "lipsOffset": s.lips_offset},
        "notes": notes.iter().map(|x| json!({"start_s": x.start_s, "end_s": x.end_s, "targetHz": x.target_hz, "foldTuningHz": x.tuning_hz,
                                             "vowel": x.vowel.to_string(), "breath": x.breath, "vibDepthSemitones": x.vib, "glideS": x.glide_s})).collect::<Vec<_>>(),
        "pan": s.pan, "gainDb": s.gain_db,
        "peak": peak, "rms": (sq / (n - begin).max(1) as f64).sqrt(),
        "outputGain": w.output_gain,
        "safetyLimited": o.safety_limited, "nonFinite": o.non_finite,
        "ledger": {
            "planDigest": r.plan_digest, "sampleFrames": r.sample_frames,
            "initialEnergy": r.initial_energy, "finalEnergy": r.current_energy,
            "sourceWork": r.source_work, "controlWork": r.control_work, "internalLoss": r.internal_loss, "radiation": r.radiation,
            "cumulativeClosureResidual": r.cumulative_closure_residual, "globalClosureResidual": r.global_closure_residual,
            "maximumFrameResidual": r.maximum_frame_residual, "maximumLocalResidual": r.maximum_local_residual,
            "moves": st.moves, "moveControlWork": st.control_work, "disposal": st.disposal, "valveRetunes": st.valve_retunes, "valveWork": st.valve_work,
        },
    });
    (y, book)
}

fn fail(e: &str) -> ! {
    eprintln!("{e}");
    std::process::exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--calibrate") {
        if args.len() < 4 {
            fail("usage: tract_choir --calibrate <singers.tsv> <skill-dir>");
        }
        calibrate(&args[2], Path::new(&args[3]));
        return;
    }
    if args.len() < 6 {
        fail("usage: tract_choir <singers.tsv> <voices.tsv> <skill-dir> <out-dir> <stem> | --calibrate <singers.tsv> <skill-dir>");
    }
    let cast = singers(&args[1]).unwrap_or_else(|e| fail(&e));
    let parts = voices(&args[2]).unwrap_or_else(|e| fail(&e));
    let skill_dir = PathBuf::from(&args[3]);
    let out_dir = PathBuf::from(&args[4]);
    let stem = args[5].clone();
    let mut performed = Vec::new();
    for (name, notes) in &parts {
        let s = cast
            .get(name)
            .unwrap_or_else(|| fail(&format!("{name} sings but is not in {}", args[1])));
        let skill =
            SkillTable::read(&skill_dir.join(format!("{name}.tsv"))).unwrap_or_else(|e| fail(&e));
        let p = perform(s, notes, &skill).unwrap_or_else(|e| fail(&format!("singer {name}: {e}")));
        performed.push((s.clone(), p));
    }
    let end_s = performed
        .iter()
        .flat_map(|(_, p)| p.iter().map(|x| x.end_s))
        .fold(0.0, f64::max)
        + TAIL_S;
    let n = (end_s * SR) as usize;
    let t0 = std::time::Instant::now();
    let jobs: Vec<Box<dyn FnOnce() -> (Singer, Vec<f64>, Value) + Send>> = performed
        .into_iter()
        .map(
            |(s, p)| -> Box<dyn FnOnce() -> (Singer, Vec<f64>, Value) + Send> {
                Box::new(move || {
                    let (y, book) = sing(&s, &p, n);
                    (s, y, book)
                })
            },
        )
        .collect();
    let mut left = vec![0.0f64; n];
    let mut right = vec![0.0f64; n];
    let mut books = Vec::new();
    for (s, y, book) in parallel(jobs) {
        // Equal-power pan; the gain is the listener's, not the singer's.
        let g = 10f64.powf(s.gain_db / 20.0);
        let theta = (s.pan + 1.0) * std::f64::consts::FRAC_PI_4;
        let (gl, gr) = (g * theta.cos(), g * theta.sin());
        for (i, v) in y.iter().enumerate() {
            left[i] += gl * v;
            right[i] += gr * v;
        }
        println!(
            "{:10} {:9} {} notes, peak {:.2} dBFS, max frame {:.2e} J, cumulative {:.2e} J",
            s.name,
            s.voice.name,
            book["notes"].as_array().map_or(0, Vec::len),
            20.0 * book["peak"].as_f64().unwrap().max(1e-12).log10(),
            book["ledger"]["maximumFrameResidual"].as_f64().unwrap(),
            book["ledger"]["cumulativeClosureResidual"]
                .as_f64()
                .unwrap()
        );
        books.push(book);
    }
    let took = t0.elapsed().as_secs_f64();
    std::fs::create_dir_all(&out_dir).unwrap();
    let wav = out_dir.join(format!("{stem}.wav"));
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: SR as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut wr = hound::WavWriter::create(&wav, spec).unwrap();
    let mut peak = 0.0f64;
    for i in 0..n {
        peak = peak.max(left[i].abs()).max(right[i].abs());
        wr.write_sample(left[i] as f32).unwrap();
        wr.write_sample(right[i] as f32).unwrap();
    }
    wr.finalize().unwrap();
    let worst = books
        .iter()
        .map(|b| b["ledger"]["maximumFrameResidual"].as_f64().unwrap().abs())
        .fold(0.0, f64::max);
    let receipt = json!({
        "schema": "ovillo.tract-choir-render.v1",
        "piece": stem,
        "singerList": args[1],
        "voiceList": args[2],
        "skillDir": args[3],
        "baseAnatomy": "TractNdAnatomy::public_v1(), per-singer lengths, folds and lungs (see each singer's anatomy)",
        "connection": "MassMomentum { disposal: Dissipate }",
        "sampleRateHz": SR,
        "seconds": end_s,
        "renderWallSeconds": took,
        "mix": "each singer's World emission x its output gain, x the listener's gain_db, equal-power pan; summed; not normalized",
        "mixPeak": peak,
        "worstMaximumFrameResidual": worst,
        "singers": books,
    });
    std::fs::write(
        out_dir.join(format!("{stem}-receipt.json")),
        serde_json::to_string_pretty(&receipt).unwrap(),
    )
    .unwrap();
    println!("{stem}: {} singers, {end_s:.1} s in {took:.1} s wall, mix peak {:.2} dBFS, worst frame residual {worst:.2e} J", books.len(), 20.0 * peak.max(1e-12).log10());
    println!("wrote {}", wav.display());
}
