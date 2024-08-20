use std::ops::Mul;

use num_traits::{Float, FromPrimitive};

pub mod frequency;

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

pub(crate) trait ULawConstant: Sized + Mul<Self, Output = Self> {
    fn ulaw_constant() -> Self;
}

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

/// [-1, 1]までの値を連続的（Continuous）なu-lawの値に変換する。
///
/// ```
/// # use soundprog::math::to_ulaw_uniform_intensity;
/// assert_eq!(to_ulaw_uniform_intensity(1.0), 1.0);
/// assert_eq!(to_ulaw_uniform_intensity(-1.0), -1.0);
/// ```
pub fn to_ulaw_uniform_intensity<T>(v: T) -> T
where
    T: Float + FromPrimitive + ULawConstant,
{
    v.signum() * (T::one() + (T::ulaw_constant() * v.abs())).ln() * (T::one() + T::ulaw_constant()).ln().recip()
}
