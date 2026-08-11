//! Minimal complex scalar for the lawful conservative control group.
//!
//! The unified-instrument contract (RUST-API-CONTRACT.md §2) requires an
//! unambiguous `[re, im]` little-endian binary64 convention. This crate
//! stays dependency-free, so the scalar is defined here rather than
//! pulled from an external crate.

use std::ops::{Add, Mul, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct Complex64 {
    pub re: f64,
    pub im: f64,
}

impl Complex64 {
    pub const ZERO: Self = Self { re: 0.0, im: 0.0 };
    pub const ONE: Self = Self { re: 1.0, im: 0.0 };

    pub fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    pub fn conj(self) -> Self {
        Self {
            re: self.re,
            im: -self.im,
        }
    }

    pub fn abs_squared(self) -> f64 {
        self.re * self.re + self.im * self.im
    }

    pub fn abs(self) -> f64 {
        self.abs_squared().sqrt()
    }

    pub fn scale(self, factor: f64) -> Self {
        Self {
            re: self.re * factor,
            im: self.im * factor,
        }
    }

    pub fn is_finite(self) -> bool {
        self.re.is_finite() && self.im.is_finite()
    }

    /// `exp(i * phase)`.
    pub fn unit_phasor(phase: f64) -> Self {
        Self {
            re: phase.cos(),
            im: phase.sin(),
        }
    }
}

impl Add for Complex64 {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self {
            re: self.re + other.re,
            im: self.im + other.im,
        }
    }
}

impl Sub for Complex64 {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self {
            re: self.re - other.re,
            im: self.im - other.im,
        }
    }
}

impl Mul for Complex64 {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        Self {
            re: self.re * other.re - self.im * other.im,
            im: self.re * other.im + self.im * other.re,
        }
    }
}

impl Neg for Complex64 {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            re: -self.re,
            im: -self.im,
        }
    }
}

/// Squared Euclidean norm of a complex state, `z* z`.
pub fn state_energy(state: &[Complex64]) -> f64 {
    state.iter().map(|value| value.abs_squared()).sum()
}
