use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::schema::{validate_digest, SchemaError};

pub const RUN_IDENTITY_VERSION: &str = "ariadne-run-identity-v1";

/// Content identity of one immutable physical run and every program required to
/// interpret it. File paths, display state, and presentation settings are not
/// part of this semantic identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunIdentity {
    #[serde(rename = "schema")]
    pub schema_version: String,
    pub physical_world_digest: String,
    pub listener_digest: String,
    pub intervention_digest: String,
    pub observation_program_digest: String,
    pub observation_schema_digest: String,
    pub optical_constitution_digest: String,
    pub sample_rate_hz: u32,
    pub observation_rate_hz: u32,
    pub source_frames: u64,
    pub tail_frames: u64,
    pub horizon_ticks: u64,
}

impl RunIdentity {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        physical_world_digest: String,
        listener_digest: String,
        intervention_digest: String,
        observation_program_digest: String,
        observation_schema_digest: String,
        optical_constitution_digest: String,
        sample_rate_hz: u32,
        observation_rate_hz: u32,
        source_frames: u64,
        tail_frames: u64,
    ) -> Result<Self, RunIdentityError> {
        let horizon_ticks = source_frames
            .checked_add(tail_frames)
            .ok_or(RunIdentityError::HorizonOverflow)?;
        let identity = Self {
            schema_version: RUN_IDENTITY_VERSION.to_owned(),
            physical_world_digest,
            listener_digest,
            intervention_digest,
            observation_program_digest,
            observation_schema_digest,
            optical_constitution_digest,
            sample_rate_hz,
            observation_rate_hz,
            source_frames,
            tail_frames,
            horizon_ticks,
        };
        identity.validate()?;
        Ok(identity)
    }

    pub fn validate(&self) -> Result<(), RunIdentityError> {
        if self.schema_version != RUN_IDENTITY_VERSION {
            return Err(RunIdentityError::UnsupportedVersion(
                self.schema_version.clone(),
            ));
        }
        for (name, digest) in [
            ("physical_world_digest", &self.physical_world_digest),
            ("listener_digest", &self.listener_digest),
            ("intervention_digest", &self.intervention_digest),
            ("observation_program_digest", &self.observation_program_digest),
            ("observation_schema_digest", &self.observation_schema_digest),
            ("optical_constitution_digest", &self.optical_constitution_digest),
        ] {
            validate_digest(name, digest)?;
        }
        if self.sample_rate_hz == 0
            || self.observation_rate_hz == 0
            || self.observation_rate_hz > self.sample_rate_hz
        {
            return Err(RunIdentityError::InvalidRate {
                sample_rate_hz: self.sample_rate_hz,
                observation_rate_hz: self.observation_rate_hz,
            });
        }
        let expected = self
            .source_frames
            .checked_add(self.tail_frames)
            .ok_or(RunIdentityError::HorizonOverflow)?;
        if self.horizon_ticks != expected {
            return Err(RunIdentityError::HorizonMismatch {
                source_frames: self.source_frames,
                tail_frames: self.tail_frames,
                horizon_ticks: self.horizon_ticks,
            });
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<String, RunIdentityError> {
        self.validate()?;
        digest_json(self)
    }
}

/// Stable SHA-256 identity of one serde JSON value using the workspace's
/// canonical `serde_json` map ordering and float representation.
pub fn digest_json(value: &(impl Serialize + ?Sized)) -> Result<String, RunIdentityError> {
    let canonical = serde_json::to_vec(value)
        .map_err(|error| RunIdentityError::Serialization(error.to_string()))?;
    Ok(format!("sha256:{:x}", Sha256::digest(canonical)))
}

#[derive(Debug, Error)]
pub enum RunIdentityError {
    #[error(transparent)]
    Schema(#[from] SchemaError),
    #[error("unsupported Ariadne run identity {0:?}")]
    UnsupportedVersion(String),
    #[error(
        "invalid material/observation rates: material={sample_rate_hz}, observation={observation_rate_hz}"
    )]
    InvalidRate {
        sample_rate_hz: u32,
        observation_rate_hz: u32,
    },
    #[error("run horizon overflows u64")]
    HorizonOverflow,
    #[error(
        "run horizon {horizon_ticks} does not equal source {source_frames} + tail {tail_frames}"
    )]
    HorizonMismatch {
        source_frames: u64,
        tail_frames: u64,
        horizon_ticks: u64,
    },
    #[error("run identity serialization failed: {0}")]
    Serialization(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        format!("sha256:{}", byte.to_string().repeat(64))
    }

    fn fixture() -> RunIdentity {
        RunIdentity::new(
            digest('a'),
            digest('b'),
            digest('c'),
            digest('d'),
            digest('e'),
            digest('f'),
            48_000,
            120,
            48_000,
            4_800,
        )
        .unwrap()
    }

    #[test]
    fn digest_survives_pretty_json_roundtrip() {
        let identity = fixture();
        let pretty = serde_json::to_string_pretty(&identity).unwrap();
        let decoded: RunIdentity = serde_json::from_str(&pretty).unwrap();
        assert_eq!(decoded, identity);
        assert_eq!(decoded.digest().unwrap(), identity.digest().unwrap());
    }

    #[test]
    fn horizon_and_rates_are_part_of_validation() {
        let mut identity = fixture();
        identity.horizon_ticks += 1;
        assert!(matches!(
            identity.validate(),
            Err(RunIdentityError::HorizonMismatch { .. })
        ));
        identity = fixture();
        identity.observation_rate_hz = identity.sample_rate_hz + 1;
        assert!(matches!(
            identity.validate(),
            Err(RunIdentityError::InvalidRate { .. })
        ));
    }
}
