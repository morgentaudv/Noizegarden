use crate::carg::v2::meta::input::EInputContainerCategoryFlag;
use crate::carg::v2::meta::node::ENode;
use crate::carg::v2::meta::setting::Setting;
use crate::carg::v2::meta::system::{InitializeSystemAccessor, TSystemCategory};
use crate::carg::v2::meta::tick::TTimeTickCategory;
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::node::common::{EProcessState, ProcessControlItem, ProcessControlItemSetting};
use crate::carg::v2::{
    EProcessOutput, ProcessItemCreateSetting, ProcessOutputBufferStereo, ProcessProcessorInput, SItemSPtr, TProcess,
    TProcessItem, TProcessItemPtr,
};
use crate::nz_define_time_tick_for;
use crate::wave::container::WaveContainer;
use crate::wave::sample::UniformedSample;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::BufReader;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaWavStereoInfo {
    /// ファイルのパス
    pub path: String,
}

#[derive(Debug)]
pub struct EmitterWavStereoProcessData {
    setting: Setting,
    common: ProcessControlItem,
    info: MetaWavStereoInfo,
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

impl TPinCategory for EmitterWavStereoProcessData {
    fn get_input_pin_names() -> Vec<&'static str> {
        vec![INPUT_IN]
    }

    fn get_output_pin_names() -> Vec<&'static str> {
        vec![OUTPUT_OUT]
    }

    fn get_pin_categories(pin_name: &str) -> Option<EPinCategoryFlag> {
        match pin_name {
            INPUT_IN => Some(pin_category::START),
            OUTPUT_OUT => Some(pin_category::BUFFER_STEREO),
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

impl TSystemCategory for EmitterWavStereoProcessData {}
nz_define_time_tick_for!(EmitterWavStereoProcessData, true, true);

impl TProcess for EmitterWavStereoProcessData {
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
        self.common.process_input_pins_deprecated();

        if self.common.state == EProcessState::Finished {
            return;
        }
        if self.common.state == EProcessState::Stopped {
            // 初期化する。
            self.initialize();
            assert!(self.internal.container.is_some());
        }

        let result = self.next_samples(input);
        if result.is_empty() {
            return;
        }

        self.common
            .insert_to_output_pin(
                OUTPUT_OUT,
                EProcessOutput::BufferStereo(ProcessOutputBufferStereo {
                    ch_left: result.left,
                    ch_right: result.right,
                    sample_rate: self.internal.sample_rate,
                }),
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

impl TProcessItem for EmitterWavStereoProcessData {
    fn can_create_item(_setting: &ProcessItemCreateSetting) -> anyhow::Result<()> {
        Ok(())
    }

    fn create_item(
        setting: &ProcessItemCreateSetting,
        system_setting: &InitializeSystemAccessor,
    ) -> anyhow::Result<TProcessItemPtr> {
        if let ENode::EmitterWavStereo(v) = setting.node {
            let item = Self {
                setting: setting.setting.clone(),
                common: ProcessControlItem::new(ProcessControlItemSetting {
                    specifier: ENodeSpecifier::EmitterWavStereo,
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

impl EmitterWavStereoProcessData {
    fn initialize(&mut self) {
        let container = {
            //let this_pointer = UnsafeCell::new(self as *mut Self);
            //self.common.systems.access_file_io_fn(move |system| {
            //    // これ絶対よくない。
            //    let this = unsafe { &mut **this_pointer.get() };

            //    // 書き込み。
            //    let file_setting = EFileAccessSetting::Read { path: this.info.path.clone() };
            //    let file_handle = system.create_handle(file_setting);

            //    let setting = FileReaderSetting {
            //        seek_to_first_when_drop: true,
            //    };
            //    let mut reader = file_handle.try_read(setting).unwrap();
            //    let container = WaveContainer::from_bufread(&mut reader).expect("Could not create WaveContainer.");

            //});

            let file = fs::File::open(&self.info.path).expect(&format!("Could not find {}.", &self.info.path));
            let mut reader = BufReader::new(file);
            WaveContainer::from_bufread(&mut reader).expect("Could not create WaveContainer.")
        };

        // 移動して終わり。
        self.internal.sample_rate = container.samples_per_second() as usize;
        self.internal.container = Some(container);
    }

    /// 初期化した情報から設定分のOutputを更新する。
    fn next_samples(&mut self, input: &ProcessProcessorInput) -> NextSamplesResult {
        assert!(self.internal.container.is_some());

        let required_sample_count = input.get_realtime_required_samples(self.internal.sample_rate);
        if required_sample_count <= 0 {
            return NextSamplesResult::default();
        }
        let buffer = self.internal.container.as_ref().unwrap().uniformed_sample_buffer();

        let (each_ch_sample_count, end) = {
            let next_start_i = self.internal.next_start_i + (required_sample_count * 2);
            if next_start_i >= buffer.len() {
                let raw_samples = buffer.len() - self.internal.next_start_i;
                (raw_samples >> 1, true)
            } else {
                (required_sample_count >> 1, false)
            }
        };

        // 汲み取る
        let mut result = NextSamplesResult::default();
        result.left.reserve(each_ch_sample_count);
        result.right.reserve(each_ch_sample_count);
        buffer
            .iter()
            .skip(self.internal.next_start_i)
            .take(each_ch_sample_count << 1)
            .tuples::<(_, _)>()
            .for_each(|(l, r)| {
                result.left.push(*l);
                result.right.push(*r);
            });

        // もし最後まで到達したら、containerを破棄する。
        if end {
            self.internal.container = None;
        }

        // インデックス更新
        self.internal.next_start_i += required_sample_count * 2;

        result
    }
}

#[derive(Default)]
struct NextSamplesResult {
    left: Vec<UniformedSample>,
    right: Vec<UniformedSample>,
}

impl NextSamplesResult {
    fn is_empty(&self) -> bool {
        self.left.is_empty() && self.right.is_empty()
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
