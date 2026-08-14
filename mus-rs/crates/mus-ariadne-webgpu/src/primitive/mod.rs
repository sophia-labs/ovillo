mod impulse_raster;
mod wave_sheet;

pub use impulse_raster::{ImpulseRaster2D, ImpulseRasterConfig};
pub use wave_sheet::{WaveSheet2D, WaveSheetConfig};

use crate::law::PrimitiveDescriptor;
use thiserror::Error;

pub trait GpuPrimitive {
    fn descriptor(&self) -> &'static PrimitiveDescriptor;
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum PrimitiveError {
    #[error("{name} must be greater than zero")]
    ZeroDimension { name: &'static str },
    #[error("observed {observed} impulses, but the primitive capacity is {capacity}")]
    TooManyImpulses { observed: usize, capacity: usize },
    #[error("parameter {name} must be finite")]
    NonFiniteParameter { name: &'static str },
}
