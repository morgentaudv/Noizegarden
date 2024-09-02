use itertools::Itertools;
use rand::{rngs, Rng};

use crate::wave::{sample::UniformedSample, PI2};

/// ユニット単位で音波のサンプルを生成するための、時間に影響しない音型のカテゴリ。
#[derive(Debug, Clone)]
pub enum ESineEmitterType {
    /// サイン波形を出力する
    Sine { frequency: f64 },
    /// ノコギリ波形を出力する
    Sawtooth { frequency: f64 },
    /// 三角波形を出力する
    Triangle { frequency: f64 },
    /// 矩形波を出力する
    Square { duty_rate: f64, frequency: f64 },
    /// ホワイトノイズを出力する
    WhiteNoise,
    /// ピンクノイズを出力する
    PinkNoise {
        /// 内部処理専用
        rows: Vec<f64>,
        /// 内部処理専用
        pink_i: i32,
        /// 内部処理専用
        running_sum: f64,
    },
}

/// ユニット単位で音波のサンプルを生成するための、時間に影響しない音型のエミッタ。
#[derive(Debug, Clone)]
pub struct SineUnitSampleEmitter {
    emitter_type: ESineEmitterType,
    phase: f64,
    intensity: f64,
    next_sample_index: usize,
    sample_rate: usize,
    rng: rngs::ThreadRng,
}

impl SineUnitSampleEmitter {
    /// サイン波形を出力するEmitterを生成する
    pub fn new_sine(frequency: f64, phase: f64, intensity: f64, sample_rate: usize) -> Self {
        Self {
            emitter_type: ESineEmitterType::Sine { frequency },
            phase,
            intensity,
            next_sample_index: 0usize,
            sample_rate,
            rng: rand::thread_rng(),
        }
    }

    /// ノコギリ波形を出力するEmitterを生成する
    pub fn new_sawtooth(frequency: f64, phase: f64, intensity: f64, sample_rate: usize) -> Self {
        Self {
            emitter_type: ESineEmitterType::Sawtooth { frequency },
            phase,
            intensity,
            next_sample_index: 0usize,
            sample_rate,
            rng: rand::thread_rng(),
        }
    }

    /// 三角波形を出力するEmitterを生成する
    pub fn new_triangle(frequency: f64, phase: f64, intensity: f64, sample_rate: usize) -> Self {
        Self {
            emitter_type: ESineEmitterType::Triangle { frequency },
            phase,
            intensity,
            next_sample_index: 0usize,
            sample_rate,
            rng: rand::thread_rng(),
        }
    }

    /// 矩形波を出力するEmitterを生成する
    pub fn new_square(frequency: f64, duty_rate: f64, phase: f64, intensity: f64, sample_rate: usize) -> Self {
        Self {
            emitter_type: ESineEmitterType::Square { frequency, duty_rate },
            phase,
            intensity,
            next_sample_index: 0usize,
            sample_rate,
            rng: rand::thread_rng(),
        }
    }

    /// ホワイトノイズを出力するEmitterを出力する
    pub fn new_whitenoise(intensity: f64) -> Self {
        Self {
            emitter_type: ESineEmitterType::WhiteNoise,
            phase: 0.0,
            intensity,
            next_sample_index: 0usize,
            sample_rate: 48000, // sample_rateは使わないので、0じゃなきゃ何でもいい。
            rng: rand::thread_rng(),
        }
    }

    /// ピンクノイズを出力するEmitterを出力する
    pub fn new_pinknoise(intensity: f64) -> Self {
        Self {
            emitter_type: ESineEmitterType::PinkNoise {
                rows: vec![],
                pink_i: 0,
                running_sum: 0.0,
            },
            phase: 0.0,
            intensity,
            next_sample_index: 0usize,
            sample_rate: 48000, // sample_rateは使わないので、0じゃなきゃ何でもいい。
            rng: rand::thread_rng(),
        }
    }
}

impl SineUnitSampleEmitter {
    /// 次のサンプルを取得する。
    pub fn next_sample(&mut self) -> UniformedSample {
        // Sin波形に入れる値はf64として計算する。
        // そしてu32に変換する。最大値は`[2^32 - 1]`である。
        let sample_rate = self.sample_rate as f64;
        let coefficient = PI2 / sample_rate;

        let unittime = self.next_sample_index as f64;
        self.next_sample_index += 1;

        match &mut self.emitter_type {
            ESineEmitterType::Sine { frequency } => {
                // 振幅と周波数のエンベロープのため相対時間を計算
                let sin_input = (coefficient * *frequency * unittime) + self.phase;
                let sample = self.intensity * sin_input.sin();
                assert!(sample >= -1.0 && sample <= 1.0);

                UniformedSample::from_f64(sample)
            }
            ESineEmitterType::Sawtooth { frequency } => {
                // 振幅と周波数のエンベロープのため相対時間を計算
                let rel_time = unittime / sample_rate;
                let orig_intensity = 1.0 - (2.0 * (rel_time * *frequency).fract());

                let sample = self.intensity * orig_intensity;
                assert!(sample >= -1.0 && sample <= 1.0);

                UniformedSample::from_f64(sample)
            }
            ESineEmitterType::Triangle { frequency } => {
                let compute_intensity = |time_i: f64, sample_rate: f64, frequency: f64| {
                    // 振幅と周波数のエンベロープのため相対時間を計算
                    let rel_time = time_i / sample_rate;
                    let orig_time = rel_time * frequency;

                    let coeff = orig_time.fract();
                    if coeff < 0.5 {
                        // [0, 0.5)の範囲
                        (-1.0 + (4.0 * coeff), orig_time)
                    } else {
                        // [0.5, 1)の範囲
                        (3.0 - (4.0 * coeff), orig_time)
                    }
                };

                let (orig_intensity, _) = compute_intensity(unittime, sample_rate, *frequency);
                let sample = self.intensity * orig_intensity;
                assert!(sample >= -1.0 && sample <= 1.0);

                UniformedSample::from_f64(sample)
            }
            ESineEmitterType::Square { duty_rate, frequency } => {
                let herz = coefficient * *frequency;
                let duty_threshold = PI2 * duty_rate.clamp(0.0, 1.0);

                // 振幅と周波数のエンベロープのため相対時間を計算
                // 正弦波形の周期を計算する。そこでduty_rateを反映する。
                // phaseは後に入れてSignを計算する。
                let unittime = unittime as f64;
                let input = (herz * unittime) + self.phase;
                let sample = self.intensity * {
                    if (input % PI2) < duty_threshold {
                        1.0
                    } else {
                        -1.0
                    }
                };

                UniformedSample::from_f64(sample)
            }
            ESineEmitterType::WhiteNoise => {
                // 正規分布からの乱数を使ってWhiteNoiseを生成する。
                // 中ではどんな方法を使っているかわからないが、一番速いのはZiggurat法。
                // https://andantesoft.hatenablog.com/entry/2023/04/30/183032

                // [-1, 1]にする。
                let value: f64 = self.rng.sample(rand::distributions::Standard);
                let value = (value * 2.0) - 1.0;

                UniformedSample::from_f64(value * self.intensity)
            }
            ESineEmitterType::PinkNoise {
                rows,
                pink_i,
                running_sum,
            } => {
                // ピンクノイズを出力する
                // https://www.firstpr.com.au/dsp/pink-noise/#Voss-McCartney を参考

                // 実装アルゴリズムを見た感じでは、
                // 多段階のRowをSumしたのがサンプルの値とみなす形式で進めているので
                // 例えば時間軸で進むとしたらLSBからビットが1になるまでの0の数を見て
                // 1 * * * * * * * * * * * * * * * *
                // 2  *   *   *   *   *   *   *   *
                // 3    *       *       *       *
                // 4        *               *
                // 5                *
                // のように扱って各Rowに乱数の値を保持して計算することができる。（これがコスト的に安い）
                let row_nums = 12;
                let pmax = 1.0 * ((row_nums + 1) as f64);
                let pink_scalar = pmax.recip();

                if rows.is_empty() {
                    rows.resize(row_nums, 0.0);
                }

                // 更新するpink_iから0の数を数えることで更新するrowsの番地を探す。
                // もしかして0なら、何もしないのがお決まり。
                *pink_i = (*pink_i + 1) & ((1 << row_nums) - 1);
                if *pink_i != 0 {
                    let row_i = pink_i.trailing_zeros() as usize;

                    // running_sumから前の値を抜いて、新しい正規乱数を入れる。
                    *running_sum -= rows[row_i];

                    // [-1, 1]にする。
                    let rng_v: f64 = self.rng.sample(rand::distributions::Standard);
                    let value = (rng_v * 2.0) - 1.0;

                    // 新しい正規乱数を足して再指定する。
                    *running_sum += value;
                    rows[row_i] = value;
                }

                // 段階が低くてもPinkNoise感を出すために（またランダム性をもたせるために）
                // 正規乱数を入れてサンプル値にする。[-1, 1]にする。
                let rng_v: f64 = self.rng.sample(rand::distributions::Standard);
                let value = (rng_v * 2.0) - 1.0;
                let sum = *running_sum + value;
                let sample_value = pink_scalar * sum;

                UniformedSample::from_f64((sample_value * self.intensity).clamp(-1.0, 1.0))
            }
        }
    }

    /// `length`分のサンプルを取得する。
    pub fn next_samples(&mut self, length: usize) -> Vec<UniformedSample> {
        if length == 0 {
            vec![]
        } else {
            (0..length).map(|_| self.next_sample()).collect_vec()
        }
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
