use super::{
    setting::{WaveSoundSetting, WaveSoundSettingBuilder},
    Second,
};

pub enum EPSGSignal {
    Sawtooth {
        length_time: Second,
        frequency: f64,
        order: usize,
    },
}

impl EPSGSignal {
    pub fn apply(&self) -> Option<Vec<WaveSoundSetting>> {
        match &self {
            EPSGSignal::Sawtooth {
                length_time,
                frequency,
                order,
            } => {
                if length_time.0 <= 0.0 {
                    return None;
                }

                let mut results = vec![];
                let mut setting = WaveSoundSettingBuilder::default();

                // 基本音を入れる。
                setting
                    .frequency(*frequency as f32)
                    .length_sec(length_time.0 as f32)
                    .intensity(0.4f64);
                results.push(setting.build().unwrap());

                // 倍音を入れる。
                for order_i in 2..*order {
                    let overtone_frequency = *frequency * (order_i as f64);
                    let intensity = 0.4f64 * (order_i as f64).recip();
                    results.push(
                        setting
                            .frequency(overtone_frequency as f32)
                            .intensity(intensity)
                            .build()
                            .unwrap(),
                    );
                }

                Some(results)
            }
        }
    }
}
