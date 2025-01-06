use std::ops::Mul;

use num_traits::{Float, FromPrimitive};

pub mod frequency;
pub mod timer;
pub mod float;
pub mod window;

/// sinc関数。[`f32`]と[`f64`]のみサポート
pub fn sinc<T>(v: T) -> T
where
    T: Float + FromPrimitive,
{
    if v == T::zero() {
        T::one()
    } else {
        v.sin() / v
    }
}

#[allow(dead_code)]
pub(crate) trait ULawConstant: Sized + Mul<Self, Output = Self> {
    fn ulaw_constant() -> Self;
}

#[allow(dead_code)]
pub(crate) trait ConstUlawConstant: ULawConstant {
    const ULAW_CONSTANT: Self;
}

macro_rules! ulaw_impl {
    ($t:ty, $v:expr) => {
        impl ULawConstant for $t {
            #[inline]
            fn ulaw_constant() -> $t {
                $v
            }
        }

        impl ConstUlawConstant for $t {
            const ULAW_CONSTANT: Self = $v;
        }
    };
}

ulaw_impl!(f32, 255.0);
ulaw_impl!(f64, 255.0);

// ----------------------------------------------------------------------------

/// `sample_rate`から`time_second`分のサンプル数を取得する。
pub fn get_required_sample_count(time_second: f64, sample_rate: usize) -> usize {
    (time_second * sample_rate as f64).floor() as usize
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
