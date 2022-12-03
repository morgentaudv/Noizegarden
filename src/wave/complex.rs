use num_traits::{Float, FromPrimitive};
use std::ops::{Add, AddAssign, Mul, MulAssign, Sub, SubAssign};

#[derive(Default, Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Complex<T: Float + FromPrimitive> {
    pub real: T,
    pub imag: T,
}

impl<T> Complex<T>
where
    T: Float + FromPrimitive,
{
    pub fn from_exp(value: T) -> Self {
        let real = value.cos();
        let imag = value.sin();
        Self { real, imag }
    }

    pub fn conjugate(&self) -> Self {
        Self {
            real: self.real,
            imag: self.imag * T::from_f32(-1.0f32).unwrap(),
        }
    }

    ///
    pub fn absolute(&self) -> T {
        (self.real.powi(2) + self.imag.powi(2)).sqrt()
    }

    ///
    pub fn phase(&self) -> T {
        (self.imag).atan2(self.real)
    }
}

impl<T> Add for Complex<T>
where
    T: Float + FromPrimitive + Add,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            real: self.real + rhs.real,
            imag: self.imag + rhs.imag,
        }
    }
}

impl<T> AddAssign for Complex<T>
where
    T: Float + FromPrimitive + AddAssign,
{
    fn add_assign(&mut self, rhs: Self) {
        self.real += rhs.real;
        self.imag += rhs.imag;
    }
}

impl<T> Sub for Complex<T>
where
    T: Float + FromPrimitive + Sub,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            real: self.real - rhs.real,
            imag: self.imag - rhs.imag,
        }
    }
}

impl<T> SubAssign for Complex<T>
where
    T: Float + FromPrimitive + SubAssign,
{
    fn sub_assign(&mut self, rhs: Self) {
        self.real -= rhs.real;
        self.imag -= rhs.imag;
    }
}

impl<T> Mul for Complex<T>
where
    T: Float + FromPrimitive + Add + Mul + Sub,
{
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        // (a + ib)(c + id)
        // real : ac - bd
        // imag : bc + ad
        Self {
            real: (self.real * rhs.real) - (self.imag * rhs.imag),
            imag: (self.imag * rhs.real) + (self.real * rhs.imag),
        }
    }
}

impl Mul<Complex<f64>> for f64 {
    type Output = Complex<f64>;

    fn mul(self, rhs: Complex<f64>) -> Self::Output {
        Self::Output {
            real: self * rhs.real,
            imag: self * rhs.imag,
        }
    }
}

impl<T> MulAssign for Complex<T>
where
    T: Float + FromPrimitive + SubAssign,
{
    fn mul_assign(&mut self, rhs: Self) {
        let new = (*self) * rhs;
        self.real = new.real;
        self.imag = new.imag;
    }
}
