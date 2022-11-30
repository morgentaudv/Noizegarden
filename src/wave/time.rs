/// https://qiita.com/osanshouo/items/4fb3d60e9ce321fa849e
/// https://zenn.dev/anchor_cable/articles/b073d510c6ff9ff7111e
use num_traits::{Float, FromPrimitive};
use std::ops::{Add, AddAssign, Sub, SubAssign};

/// 秒単位の値を示す。
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Second<T: Float + FromPrimitive>(T);

impl<T> Second<T>
where
    T: Float + FromPrimitive,
{
    pub fn from(second: T) -> Self {
        Second(second)
    }
}

impl<T> Add for Second<T>
where
    T: Float + FromPrimitive + Add,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::from(self.0 + rhs.0)
    }
}

impl<T> AddAssign for Second<T>
where
    T: Float + FromPrimitive + AddAssign,
{
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl<T> Sub for Second<T>
where
    T: Float + FromPrimitive + Sub,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::from(self.0 - rhs.0)
    }
}

impl<T> SubAssign for Second<T>
where
    T: Float + FromPrimitive + SubAssign,
{
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}
