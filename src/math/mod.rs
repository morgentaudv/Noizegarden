use num_traits::{Float, FromPrimitive};

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
