use std::f64::consts::PI;

// WAVE (Waveform Audio File Format)
// https://so-zou.jp/software/tech/file/format/wav/
pub mod analyze;
pub mod complex;
pub mod container;
pub mod filter;
pub mod sample;
pub mod setting;
pub mod stretch;
pub mod time;

/// 2PIを示す。
pub(crate) const PI2: f64 = 2.0 * PI;

/// 秒を表す。
#[repr(transparent)]
pub struct Second(pub f64);
