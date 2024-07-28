use crate::wave::complex::Complex;

/// サイン波形の周波数の特性を表す。
#[derive(Default, Debug, Clone, Copy)]
pub struct SineFrequency {
    pub frequency: f64,
    pub amplitude: f64,
    pub phase: f64,
}

impl SineFrequency {
    pub fn from(frequency: f64, (freq_real, freq_imag): (f32, f32)) -> Self {
        Self {
            frequency,
            amplitude: (freq_real.powi(2) + freq_imag.powi(2)).sqrt() as f64,
            phase: (freq_imag / freq_real).atan() as f64,
        }
    }

    pub fn from_complex_f32(frequency: f32, complex: Complex<f32>) -> Self {
        Self {
            frequency: frequency as f64,
            amplitude: complex.absolute() as f64,
            phase: complex.phase() as f64,
        }
    }

    pub fn from_complex_f64(frequency: f64, complex: Complex<f64>) -> Self {
        Self {
            frequency,
            amplitude: complex.absolute(),
            phase: complex.phase(),
        }
    }

    pub fn to_complex_f64(&self) -> Complex<f64> {
        let real = self.phase.cos() * self.amplitude;
        let imag = self.phase.sin() * self.amplitude;
        Complex::<f64> { real, imag }
    }
}
