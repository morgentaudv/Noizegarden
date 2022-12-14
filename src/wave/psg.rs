use super::{
    setting::{EFrequencyItem, WaveSoundSetting, WaveSoundSettingBuilder},
    Second,
};

pub enum EPSGSignal {
    Sawtooth {
        length_time: Second,
        frequency: f64,
        order: usize,
        intensity: f64,
    },
}

impl EPSGSignal {
    pub fn apply(&self) -> Option<Vec<WaveSoundSetting>> {
        match &self {
            EPSGSignal::Sawtooth {
                length_time,
                frequency,
                order,
                intensity,
            } => {
                if length_time.0 <= 0.0 {
                    return None;
                }

                let mut results = vec![];
                let mut setting = WaveSoundSettingBuilder::default();

                // 基本音を入れる。
                setting
                    .frequency(EFrequencyItem::Constant { frequency: *frequency })
                    .length_sec(length_time.0 as f32)
                    .intensity(*intensity);
                results.push(setting.build().unwrap());

                // 倍音を入れる。
                for order_i in 2..*order {
                    let overtone_frequency = *frequency * (order_i as f64);
                    let overtone_intensity = intensity * (order_i as f64).recip();
                    results.push(
                        setting
                            .frequency(EFrequencyItem::Constant {
                                frequency: overtone_frequency,
                            })
                            .length_sec(length_time.0 as f32)
                            .intensity(overtone_intensity)
                            .build()
                            .unwrap(),
                    );
                }

                Some(results)
            }
        }
    }
}
