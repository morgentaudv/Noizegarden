use crate::carg::v2::meta::input::EInputContainerCategoryFlag;
use crate::carg::v2::meta::node::ENode;
use crate::carg::v2::meta::setting::Setting;
use crate::carg::v2::meta::system::{InitializeSystemAccessor, TSystemCategory};
use crate::carg::v2::meta::tick::TTimeTickCategory;
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::node::common::{EProcessState, ProcessControlItemSetting};
use crate::carg::v2::{EProcessOutput, ProcessControlItem, ProcessItemCreateSetting, ProcessOutputBuffer, ProcessProcessorInput, SItemSPtr, TProcess, TProcessItem, TProcessItemPtr};
use crate::nz_define_time_tick_for;
use crate::wave::container::WaveContainer;
use crate::wave::sample::UniformedSample;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::BufReader;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaWavInfo {
    /// ファイルのパス
    pub path: String,
}

#[derive(Debug)]
pub struct EmitterWavMonoProcessData {
    setting: Setting,
    common: ProcessControlItem,
    info: MetaWavInfo,
    internal: InternalInfo,
}

#[derive(Default, Debug)]
struct InternalInfo {
    container: Option<WaveContainer>,
    /// 次のサンプル出力でバッファのスタート地点インデックス
    next_start_i: usize,
    /// PCMのサンプルレート
    sample_rate: usize,
}

const INPUT_IN: &'static str = "in";
const OUTPUT_OUT: &'static str = "out";

impl TPinCategory for EmitterWavMonoProcessData {
    fn get_input_pin_names() -> Vec<&'static str> {
        vec![INPUT_IN]
    }

    fn get_output_pin_names() -> Vec<&'static str> {
        vec![OUTPUT_OUT]
    }

    fn get_pin_categories(pin_name: &str) -> Option<EPinCategoryFlag> {
        match pin_name {
            INPUT_IN => Some(pin_category::START),
            OUTPUT_OUT => Some(pin_category::BUFFER_MONO),
            _ => None,
        }
    }

    fn get_input_container_flag(pin_name: &str) -> Option<EInputContainerCategoryFlag> {
        match pin_name {
            INPUT_IN => Some(input::container_category::EMPTY),
            _ => None,
        }
    }
}

impl TSystemCategory for EmitterWavMonoProcessData {}
nz_define_time_tick_for!(EmitterWavMonoProcessData, true, true);

impl TProcess for EmitterWavMonoProcessData {
    fn is_finished(&self) -> bool {
        self.common.state == EProcessState::Finished
    }
    fn can_process(&self) -> bool {
        true
    }

    fn get_common_ref(&self) -> &ProcessControlItem {
        &self.common
    }

    fn get_common_mut(&mut self) -> &mut ProcessControlItem {
        &mut self.common
    }

    fn try_process(&mut self, input: &ProcessProcessorInput) {
        // 時間更新。またInputピンのリソース更新はしなくてもいい。
        self.common.elapsed_time = input.common.elapsed_time;
        self.common.process_input_pins();

        if self.common.state == EProcessState::Finished {
            return;
        }
        if self.common.state == EProcessState::Stopped {
            // 初期化する。
            self.initialize();
            assert!(self.internal.container.is_some());
        }

        // バッファを出力する。
        let buffer = self.next_samples(input);
        self.common
            .insert_to_output_pin(
                OUTPUT_OUT,
                EProcessOutput::BufferMono(ProcessOutputBuffer::new(buffer, self.internal.sample_rate)),
            )
            .unwrap();

        // 状態確認
        if self.internal.container.is_some() {
            self.common.state = EProcessState::Playing;
        } else {
            self.common.state = EProcessState::Finished;
        }
    }
}

impl TProcessItem for EmitterWavMonoProcessData {
    fn can_create_item(_setting: &ProcessItemCreateSetting) -> anyhow::Result<()> {
        Ok(())
    }

    fn create_item(setting: &ProcessItemCreateSetting, system_setting: &InitializeSystemAccessor) -> anyhow::Result<TProcessItemPtr> {
        if let ENode::EmitterWavMono(v) = setting.node {
            let item = Self {
                setting: setting.setting.clone(),
                common: ProcessControlItem::new(ProcessControlItemSetting{
                    specifier: ENodeSpecifier::EmitterWavMono,
                    systems: &system_setting,
                }),
                info: v.clone(),
                internal: InternalInfo::default(),
            };
            return Ok(SItemSPtr::new(item));
        }

        unreachable!("Unexpected branch");
    }
}

impl EmitterWavMonoProcessData {
    fn initialize(&mut self) {
        let container = {
            let file = fs::File::open(&self.info.path).expect(&format!("Could not find {}.", &self.info.path));
            let mut reader = BufReader::new(file);
            WaveContainer::from_bufread(&mut reader).expect("Could not create WaveContainer.")
        };
        assert_eq!(container.bits_per_sample(), 16);

        // 移動して終わり。
        self.internal.sample_rate = container.samples_per_second() as usize;
        self.internal.container = Some(container);
    }

    /// 初期化した情報から設定分のOutputを更新する。
    fn next_samples(&mut self, _input: &ProcessProcessorInput) -> Vec<UniformedSample> {
        assert!(self.internal.container.is_some());

        // 24-11-06
        // TODO リアルタイムのDeltaTimeに対応する。
        // 今は固定サンプルを汲み取る。
        let (sample_counts, end) = {
            let ideal_count = self.setting.sample_count_frame;
            let next_start_i = self.internal.next_start_i + ideal_count;

            let buffer = self.internal.container.as_ref().unwrap().uniformed_sample_buffer();
            if next_start_i >= buffer.len() {
                (buffer.len() - self.internal.next_start_i, true)
            } else {
                (ideal_count, false)
            }
        };

        // 汲み取る
        let buffer = self
            .internal
            .container
            .as_ref()
            .unwrap()
            .uniformed_sample_buffer()
            .iter()
            .skip(self.internal.next_start_i)
            .take(sample_counts)
            .cloned()
            .collect();

        // もし最後まで到達したら、containerを破棄する。
        if end {
            self.internal.container = None;
        }

        // インデックス更新
        self.internal.next_start_i += sample_counts;

        buffer
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
