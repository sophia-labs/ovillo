use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const OBSERVATION_SCHEMA_VERSION: &str = "ariadne-observation-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScalarType {
    F32,
    F64,
    U64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Aggregation {
    Instant,
    IntervalDelta,
    Mean,
    Rms,
    Peak,
    MaximumAbsolute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Interpolation {
    Step,
    Linear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClaimDomain {
    Physical,
    Observer,
    Performance,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelSpec {
    pub id: String,
    pub unit: String,
    pub scalar_type: ScalarType,
    pub aggregation: Aggregation,
    pub interpolation: Interpolation,
    pub claim_domain: ClaimDomain,
}

impl ChannelSpec {
    #[must_use]
    pub fn physical(
        id: impl Into<String>,
        unit: impl Into<String>,
        scalar_type: ScalarType,
        aggregation: Aggregation,
        interpolation: Interpolation,
    ) -> Self {
        Self {
            id: id.into(),
            unit: unit.into(),
            scalar_type,
            aggregation,
            interpolation,
            claim_domain: ClaimDomain::Physical,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationSchema {
    pub schema_version: String,
    pub physical_world_digest: String,
    pub sample_rate_hz: u32,
    pub cadence_ticks: u64,
    pub channels: Vec<ChannelSpec>,
}

impl ObservationSchema {
    #[must_use]
    pub fn new(
        physical_world_digest: impl Into<String>,
        sample_rate_hz: u32,
        cadence_ticks: u64,
        channels: Vec<ChannelSpec>,
    ) -> Self {
        Self {
            schema_version: OBSERVATION_SCHEMA_VERSION.to_owned(),
            physical_world_digest: physical_world_digest.into(),
            sample_rate_hz,
            cadence_ticks,
            channels,
        }
    }

    pub fn validate(&self) -> Result<(), SchemaError> {
        if self.schema_version != OBSERVATION_SCHEMA_VERSION {
            return Err(SchemaError::UnsupportedVersion(self.schema_version.clone()));
        }
        validate_digest("physical_world_digest", &self.physical_world_digest)?;
        if self.sample_rate_hz == 0 {
            return Err(SchemaError::InvalidSampleRate);
        }
        if self.cadence_ticks == 0 {
            return Err(SchemaError::InvalidCadence);
        }
        if self.channels.is_empty() {
            return Err(SchemaError::EmptyChannels);
        }
        let mut ids = BTreeSet::new();
        for (index, channel) in self.channels.iter().enumerate() {
            if channel.id.trim().is_empty() {
                return Err(SchemaError::EmptyChannelId(index));
            }
            if channel.unit.trim().is_empty() {
                return Err(SchemaError::EmptyChannelUnit(index));
            }
            if !ids.insert(channel.id.clone()) {
                return Err(SchemaError::DuplicateChannel(channel.id.clone()));
            }
            if matches!(channel.scalar_type, ScalarType::U64)
                && matches!(channel.interpolation, Interpolation::Linear)
            {
                return Err(SchemaError::IntegerLinearInterpolation(channel.id.clone()));
            }
        }
        Ok(())
    }

    pub fn digest(&self) -> Result<String, SchemaError> {
        self.validate()?;
        let canonical = serde_json::to_vec(self)
            .map_err(|error| SchemaError::Serialization(error.to_string()))?;
        Ok(format!("sha256:{:x}", Sha256::digest(canonical)))
    }

    #[must_use]
    pub fn frame_stride_bytes(&self) -> usize {
        8 + 8 * self.channels.len()
    }
}

pub fn validate_digest(field: &'static str, digest: &str) -> Result<(), SchemaError> {
    let Some(hex) = digest.strip_prefix("sha256:") else {
        return Err(SchemaError::InvalidDigest {
            field,
            value: digest.to_owned(),
        });
    };
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(SchemaError::InvalidDigest {
            field,
            value: digest.to_owned(),
        });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SchemaError {
    #[error("unsupported observation schema version {0:?}")]
    UnsupportedVersion(String),
    #[error("{field} is not a sha256 digest: {value:?}")]
    InvalidDigest { field: &'static str, value: String },
    #[error("observation sample rate must be positive")]
    InvalidSampleRate,
    #[error("observation cadence must be at least one material tick")]
    InvalidCadence,
    #[error("observation schema must contain at least one channel")]
    EmptyChannels,
    #[error("observation channel {0} has an empty id")]
    EmptyChannelId(usize),
    #[error("observation channel {0} has an empty unit")]
    EmptyChannelUnit(usize),
    #[error("duplicate observation channel {0:?}")]
    DuplicateChannel(String),
    #[error("integer observation channel {0:?} cannot use linear interpolation")]
    IntegerLinearInterpolation(String),
    #[error("observation schema serialization failed: {0}")]
    Serialization(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest() -> String {
        format!("sha256:{}", "ab".repeat(32))
    }

    #[test]
    fn schema_is_content_addressed() {
        let schema = ObservationSchema::new(
            digest(),
            48_000,
            800,
            vec![ChannelSpec::physical(
                "world.energy",
                "joule-normalized",
                ScalarType::F64,
                Aggregation::Instant,
                Interpolation::Linear,
            )],
        );
        assert!(schema.digest().unwrap().starts_with("sha256:"));
        assert_eq!(schema.frame_stride_bytes(), 16);
    }

    #[test]
    fn duplicate_and_ambiguous_integer_channels_refuse() {
        let channel = ChannelSpec::physical(
            "events",
            "count",
            ScalarType::U64,
            Aggregation::Instant,
            Interpolation::Step,
        );
        let duplicate =
            ObservationSchema::new(digest(), 48_000, 800, vec![channel.clone(), channel]);
        assert!(matches!(
            duplicate.validate(),
            Err(SchemaError::DuplicateChannel(_))
        ));

        let linear_integer = ObservationSchema::new(
            digest(),
            48_000,
            800,
            vec![ChannelSpec::physical(
                "events",
                "count",
                ScalarType::U64,
                Aggregation::Instant,
                Interpolation::Linear,
            )],
        );
        assert!(matches!(
            linear_integer.validate(),
            Err(SchemaError::IntegerLinearInterpolation(_))
        ));
    }
}
