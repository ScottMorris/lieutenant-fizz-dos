// The f64 functions `core` does not have, from the `libm` crate so every target (web, DOS) computes the same values.
//
// (c) Copyright 2026 Liminal HQ, Scott Morris
// SPDX-License-Identifier: Apache-2.0 OR MIT

/// Method-call syntax for the `libm` functions, so `x.floor()` reads the same with and without `std`.
pub trait FloatMath: Sized {
    fn floor(self) -> Self;
    fn ceil(self) -> Self;
    fn sqrt(self) -> Self;
    fn sin(self) -> Self;
    fn cos(self) -> Self;
    fn exp(self) -> Self;
    fn hypot(self, other: Self) -> Self;
    fn atan2(self, x: Self) -> Self;
    fn powi(self, n: i32) -> Self;
    fn fract(self) -> Self;
}

impl FloatMath for f64 {
    fn floor(self) -> f64 {
        libm::floor(self)
    }
    fn ceil(self) -> f64 {
        libm::ceil(self)
    }
    fn sqrt(self) -> f64 {
        libm::sqrt(self)
    }
    fn sin(self) -> f64 {
        libm::sin(self)
    }
    fn cos(self) -> f64 {
        libm::cos(self)
    }
    fn exp(self) -> f64 {
        libm::exp(self)
    }
    fn hypot(self, other: f64) -> f64 {
        libm::hypot(self, other)
    }
    fn atan2(self, x: f64) -> f64 {
        libm::atan2(self, x)
    }
    fn powi(self, n: i32) -> f64 {
        libm::pow(self, f64::from(n))
    }
    fn fract(self) -> f64 {
        self - libm::trunc(self)
    }
}

/// The same for `f32`.
pub trait FloatMath32: Sized {
    fn sin_cos(self) -> (Self, Self);
    fn sqrt(self) -> Self;
}

impl FloatMath32 for f32 {
    fn sin_cos(self) -> (f32, f32) {
        (libm::sinf(self), libm::cosf(self))
    }
    fn sqrt(self) -> f32 {
        libm::sqrtf(self)
    }
}
