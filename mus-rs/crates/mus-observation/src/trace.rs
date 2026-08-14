use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::schema::{validate_digest, Interpolation, ObservationSchema, ScalarType, SchemaError};

pub const TRACE_MAGIC: [u8; 8] = *b"ARIOBS01";
pub const TRACE_FORMAT_VERSION: u32 = 1;
const PREFIX_BYTES: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "kebab-case")]
pub enum ObservationValue {
    F32(f32),
    F64(f64),
    U64(u64),
}

impl ObservationValue {
    #[must_use]
    pub const fn scalar_type(self) -> ScalarType {
        match self {
            Self::F32(_) => ScalarType::F32,
            Self::F64(_) => ScalarType::F64,
            Self::U64(_) => ScalarType::U64,
        }
    }

    #[must_use]
    pub fn is_finite(self) -> bool {
        match self {
            Self::F32(value) => value.is_finite(),
            Self::F64(value) => value.is_finite(),
            Self::U64(_) => true,
        }
    }

    #[must_use]
    pub fn as_f64(self) -> f64 {
        match self {
            Self::F32(value) => value as f64,
            Self::F64(value) => value,
            Self::U64(value) => value as f64,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationFrame {
    pub tick: u64,
    pub values: Vec<ObservationValue>,
}

impl ObservationFrame {
    #[must_use]
    pub fn new(tick: u64, values: Vec<ObservationValue>) -> Self {
        Self { tick, values }
    }

    fn validate(&self, schema: &ObservationSchema) -> Result<(), TraceError> {
        if self.values.len() != schema.channels.len() {
            return Err(TraceError::ChannelCount {
                tick: self.tick,
                expected: schema.channels.len(),
                actual: self.values.len(),
            });
        }
        for (index, (value, channel)) in self.values.iter().zip(&schema.channels).enumerate() {
            if value.scalar_type() != channel.scalar_type {
                return Err(TraceError::ValueType {
                    tick: self.tick,
                    channel: index,
                    expected: channel.scalar_type,
                    actual: value.scalar_type(),
                });
            }
            if !value.is_finite() {
                return Err(TraceError::NonFiniteValue {
                    tick: self.tick,
                    channel: index,
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceMetadata {
    pub schema: ObservationSchema,
    pub run_digest: String,
    pub horizon_ticks: u64,
    pub frame_count: u64,
}

impl TraceMetadata {
    fn validate(&self) -> Result<(), TraceError> {
        self.schema.validate()?;
        validate_digest("run_digest", &self.run_digest)?;
        if self.frame_count == 0 {
            return Err(TraceError::EmptyTrace);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObservationTrace {
    pub metadata: TraceMetadata,
    pub frames: Vec<ObservationFrame>,
}

impl ObservationTrace {
    pub fn new(
        schema: ObservationSchema,
        run_digest: impl Into<String>,
        horizon_ticks: u64,
        frames: Vec<ObservationFrame>,
    ) -> Result<Self, TraceError> {
        let frame_count = u64::try_from(frames.len()).map_err(|_| TraceError::TooManyFrames)?;
        let result = Self {
            metadata: TraceMetadata {
                schema,
                run_digest: run_digest.into(),
                horizon_ticks,
                frame_count,
            },
            frames,
        };
        result.validate()?;
        Ok(result)
    }

    pub fn validate(&self) -> Result<(), TraceError> {
        self.metadata.validate()?;
        if self.metadata.frame_count as usize != self.frames.len() {
            return Err(TraceError::FrameCount {
                declared: self.metadata.frame_count,
                actual: self.frames.len(),
            });
        }
        let mut previous = None;
        for frame in &self.frames {
            frame.validate(&self.metadata.schema)?;
            if frame.tick > self.metadata.horizon_ticks {
                return Err(TraceError::BeyondHorizon {
                    tick: frame.tick,
                    horizon: self.metadata.horizon_ticks,
                });
            }
            if let Some(previous) = previous {
                if frame.tick <= previous {
                    return Err(TraceError::NonMonotone {
                        previous,
                        next: frame.tick,
                    });
                }
            }
            previous = Some(frame.tick);
        }
        Ok(())
    }

    #[must_use]
    pub fn duration_seconds(&self) -> f64 {
        self.metadata.horizon_ticks as f64 / self.metadata.schema.sample_rate_hz as f64
    }

    #[must_use]
    pub fn zero_value_buffer(&self) -> Vec<ObservationValue> {
        self.metadata
            .schema
            .channels
            .iter()
            .map(|channel| match channel.scalar_type {
                ScalarType::F32 => ObservationValue::F32(0.0),
                ScalarType::F64 => ObservationValue::F64(0.0),
                ScalarType::U64 => ObservationValue::U64(0),
            })
            .collect()
    }

    /// Sample into caller-owned storage. The method performs no allocation and
    /// applies each channel's declared interpolation law.
    pub fn sample_at_tick_into(
        &self,
        tick: u64,
        output: &mut [ObservationValue],
    ) -> Result<(), TraceError> {
        if output.len() != self.metadata.schema.channels.len() {
            return Err(TraceError::OutputWidth {
                expected: self.metadata.schema.channels.len(),
                actual: output.len(),
            });
        }
        if self.frames.len() == 1 || tick <= self.frames[0].tick {
            output.copy_from_slice(&self.frames[0].values);
            return Ok(());
        }
        let last = self.frames.len() - 1;
        if tick >= self.frames[last].tick {
            output.copy_from_slice(&self.frames[last].values);
            return Ok(());
        }
        let right = self
            .frames
            .binary_search_by(|frame| frame.tick.cmp(&tick))
            .unwrap_or_else(|index| index);
        if self.frames[right].tick == tick {
            output.copy_from_slice(&self.frames[right].values);
            return Ok(());
        }
        let left = right - 1;
        let left_frame = &self.frames[left];
        let right_frame = &self.frames[right];
        let span = (right_frame.tick - left_frame.tick).max(1) as f64;
        let alpha = (tick - left_frame.tick) as f64 / span;
        for (index, channel) in self.metadata.schema.channels.iter().enumerate() {
            output[index] = match channel.interpolation {
                Interpolation::Step => left_frame.values[index],
                Interpolation::Linear => interpolate(
                    left_frame.values[index],
                    right_frame.values[index],
                    alpha,
                ),
            };
        }
        Ok(())
    }

    #[must_use]
    pub fn sample_at_tick(&self, tick: u64) -> Vec<ObservationValue> {
        let mut output = self.zero_value_buffer();
        self.sample_at_tick_into(tick, &mut output)
            .expect("trace-owned sample buffer has the exact schema width");
        output
    }

    #[must_use]
    pub fn sample_at_seconds(&self, seconds: f64) -> Vec<ObservationValue> {
        let seconds = if seconds.is_finite() {
            seconds.max(0.0)
        } else {
            0.0
        };
        let tick = (seconds * self.metadata.schema.sample_rate_hz as f64)
            .round()
            .clamp(0.0, self.metadata.horizon_ticks as f64) as u64;
        self.sample_at_tick(tick)
    }
}

pub fn encode_trace(trace: &ObservationTrace) -> Result<Vec<u8>, TraceError> {
    trace.validate()?;
    let metadata = serde_json::to_vec(&trace.metadata)
        .map_err(|error| TraceError::Serialization(error.to_string()))?;
    let metadata_len = u32::try_from(metadata.len()).map_err(|_| TraceError::MetadataTooLarge)?;
    let frame_stride = trace.metadata.schema.frame_stride_bytes();
    let mut output =
        Vec::with_capacity(PREFIX_BYTES + metadata.len() + frame_stride * trace.frames.len());
    output.extend_from_slice(&TRACE_MAGIC);
    output.extend_from_slice(&TRACE_FORMAT_VERSION.to_le_bytes());
    output.extend_from_slice(&metadata_len.to_le_bytes());
    output.extend_from_slice(&metadata);
    for frame in &trace.frames {
        output.extend_from_slice(&frame.tick.to_le_bytes());
        for value in &frame.values {
            encode_value(&mut output, *value);
        }
    }
    Ok(output)
}

pub fn decode_trace(bytes: &[u8]) -> Result<ObservationTrace, TraceError> {
    if bytes.len() < PREFIX_BYTES {
        return Err(TraceError::Length {
            expected_at_least: PREFIX_BYTES,
            actual: bytes.len(),
        });
    }
    if bytes[..8] != TRACE_MAGIC {
        return Err(TraceError::Magic);
    }
    let version = read_u32(bytes, 8)?;
    if version != TRACE_FORMAT_VERSION {
        return Err(TraceError::FormatVersion(version));
    }
    let metadata_len = read_u32(bytes, 12)? as usize;
    let metadata_end = PREFIX_BYTES
        .checked_add(metadata_len)
        .ok_or(TraceError::MetadataTooLarge)?;
    if metadata_end > bytes.len() {
        return Err(TraceError::Length {
            expected_at_least: metadata_end,
            actual: bytes.len(),
        });
    }
    let metadata: TraceMetadata = serde_json::from_slice(&bytes[PREFIX_BYTES..metadata_end])
        .map_err(|error| TraceError::Serialization(error.to_string()))?;
    metadata.validate()?;
    let frame_stride = metadata.schema.frame_stride_bytes();
    let frames_len = usize::try_from(metadata.frame_count)
        .map_err(|_| TraceError::TooManyFrames)?
        .checked_mul(frame_stride)
        .ok_or(TraceError::TooManyFrames)?;
    let expected = metadata_end
        .checked_add(frames_len)
        .ok_or(TraceError::TooManyFrames)?;
    if bytes.len() != expected {
        return Err(TraceError::ExactLength {
            expected,
            actual: bytes.len(),
        });
    }
    let mut frames = Vec::with_capacity(metadata.frame_count as usize);
    let mut offset = metadata_end;
    for _ in 0..metadata.frame_count {
        let tick = read_u64(bytes, offset)?;
        offset += 8;
        let mut values = Vec::with_capacity(metadata.schema.channels.len());
        for channel in &metadata.schema.channels {
            values.push(decode_value(bytes, offset, channel.scalar_type)?);
            offset += 8;
        }
        frames.push(ObservationFrame { tick, values });
    }
    let trace = ObservationTrace { metadata, frames };
    trace.validate()?;
    Ok(trace)
}

fn interpolate(left: ObservationValue, right: ObservationValue, alpha: f64) -> ObservationValue {
    match (left, right) {
        (ObservationValue::F32(left), ObservationValue::F32(right)) => {
            ObservationValue::F32((left as f64 + alpha * (right as f64 - left as f64)) as f32)
        }
        (ObservationValue::F64(left), ObservationValue::F64(right)) => {
            ObservationValue::F64(left + alpha * (right - left))
        }
        (ObservationValue::U64(left), ObservationValue::U64(_)) => ObservationValue::U64(left),
        _ => left,
    }
}

fn encode_value(output: &mut Vec<u8>, value: ObservationValue) {
    match value {
        ObservationValue::F32(value) => {
            output.extend_from_slice(&value.to_bits().to_le_bytes());
            output.extend_from_slice(&0_u32.to_le_bytes());
        }
        ObservationValue::F64(value) => output.extend_from_slice(&value.to_bits().to_le_bytes()),
        ObservationValue::U64(value) => output.extend_from_slice(&value.to_le_bytes()),
    }
}

fn decode_value(
    bytes: &[u8],
    offset: usize,
    scalar_type: ScalarType,
) -> Result<ObservationValue, TraceError> {
    Ok(match scalar_type {
        ScalarType::F32 => ObservationValue::F32(f32::from_bits(read_u32(bytes, offset)?)),
        ScalarType::F64 => ObservationValue::F64(f64::from_bits(read_u64(bytes, offset)?)),
        ScalarType::U64 => ObservationValue::U64(read_u64(bytes, offset)?),
    })
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, TraceError> {
    let end = offset.checked_add(4).ok_or(TraceError::TooManyFrames)?;
    let slice = bytes.get(offset..end).ok_or(TraceError::Length {
        expected_at_least: end,
        actual: bytes.len(),
    })?;
    Ok(u32::from_le_bytes(
        slice.try_into().expect("four-byte slice"),
    ))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, TraceError> {
    let end = offset.checked_add(8).ok_or(TraceError::TooManyFrames)?;
    let slice = bytes.get(offset..end).ok_or(TraceError::Length {
        expected_at_least: end,
        actual: bytes.len(),
    })?;
    Ok(u64::from_le_bytes(
        slice.try_into().expect("eight-byte slice"),
    ))
}

#[derive(Debug, Error)]
pub enum TraceError {
    #[error(transparent)]
    Schema(#[from] SchemaError),
    #[error("observation trace has invalid magic")]
    Magic,
    #[error("unsupported observation trace format version {0}")]
    FormatVersion(u32),
    #[error("observation trace length is {actual}, expected at least {expected_at_least}")]
    Length {
        expected_at_least: usize,
        actual: usize,
    },
    #[error("observation trace length is {actual}, expected exactly {expected}")]
    ExactLength { expected: usize, actual: usize },
    #[error("observation trace metadata is too large")]
    MetadataTooLarge,
    #[error("observation trace is empty")]
    EmptyTrace,
    #[error("observation trace contains too many frames")]
    TooManyFrames,
    #[error("observation trace declares {declared} frames but contains {actual}")]
    FrameCount { declared: u64, actual: usize },
    #[error("observation frame {tick} has {actual} values; expected {expected}")]
    ChannelCount {
        tick: u64,
        expected: usize,
        actual: usize,
    },
    #[error("observation output has width {actual}; expected {expected}")]
    OutputWidth { expected: usize, actual: usize },
    #[error(
        "observation frame {tick} channel {channel} expected {expected:?}, found {actual:?}"
    )]
    ValueType {
        tick: u64,
        channel: usize,
        expected: ScalarType,
        actual: ScalarType,
    },
    #[error("observation frame {tick} channel {channel} is non-finite")]
    NonFiniteValue { tick: u64, channel: usize },
    #[error("observation ticks are not strictly increasing: {previous} then {next}")]
    NonMonotone { previous: u64, next: u64 },
    #[error("observation tick {tick} exceeds horizon {horizon}")]
    BeyondHorizon { tick: u64, horizon: u64 },
    #[error("observation serialization failed: {0}")]
    Serialization(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{Aggregation, ChannelSpec, Interpolation, ScalarType};

    fn digest(byte: &str) -> String {
        format!("sha256:{}", byte.repeat(64))
    }

    fn fixture() -> ObservationTrace {
        let schema = ObservationSchema::new(
            digest("a"),
            48_000,
            800,
            vec![
                ChannelSpec::physical(
                    "world.energy",
                    "normalized-joule",
                    ScalarType::F64,
                    Aggregation::Instant,
                    Interpolation::Linear,
                ),
                ChannelSpec::physical(
                    "events",
                    "count",
                    ScalarType::U64,
                    Aggregation::Instant,
                    Interpolation::Step,
                ),
            ],
        );
        ObservationTrace::new(
            schema,
            digest("b"),
            48_000,
            vec![
                ObservationFrame::new(
                    0,
                    vec![ObservationValue::F64(0.0), ObservationValue::U64(1)],
                ),
                ObservationFrame::new(
                    48_000,
                    vec![ObservationValue::F64(2.0), ObservationValue::U64(2)],
                ),
            ],
        )
        .unwrap()
    }

    #[test]
    fn binary_round_trip_is_exact() {
        let trace = fixture();
        let bytes = encode_trace(&trace).unwrap();
        assert_eq!(decode_trace(&bytes).unwrap(), trace);
    }

    #[test]
    fn interpolation_is_schema_driven_and_allocation_free_when_requested() {
        let trace = fixture();
        let mut output = trace.zero_value_buffer();
        trace.sample_at_tick_into(24_000, &mut output).unwrap();
        assert_eq!(
            output,
            vec![ObservationValue::F64(1.0), ObservationValue::U64(1)]
        );
        assert_eq!(trace.sample_at_tick(24_000), output);
    }

    #[test]
    fn wrong_output_width_refuses_without_mutating_storage() {
        let trace = fixture();
        let mut output = [ObservationValue::F64(99.0)];
        assert!(matches!(
            trace.sample_at_tick_into(0, &mut output),
            Err(TraceError::OutputWidth { .. })
        ));
        assert_eq!(output, [ObservationValue::F64(99.0)]);
    }

    #[test]
    fn malformed_length_order_and_value_refuse() {
        let bytes = encode_trace(&fixture()).unwrap();
        assert!(decode_trace(&bytes[..bytes.len() - 1]).is_err());

        let mut non_monotone = fixture();
        non_monotone.frames[1].tick = 0;
        assert!(matches!(
            non_monotone.validate(),
            Err(TraceError::NonMonotone { .. })
        ));

        let mut non_finite = fixture();
        non_finite.frames[0].values[0] = ObservationValue::F64(f64::NAN);
        assert!(matches!(
            non_finite.validate(),
            Err(TraceError::NonFiniteValue { .. })
        ));
    }
}
