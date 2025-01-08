use std::fs;
use std::io::BufReader;
use serde::{Deserialize, Serialize};
use crate::carg::v2::{EProcessOutput, ProcessControlItem, ProcessItemCreateSetting, ProcessOutputBuffer, ProcessProcessorInput, SItemSPtr, TProcess, TProcessItem, TProcessItemPtr};
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::meta::input::EInputContainerCategoryFlag;
use crate::carg::v2::meta::node::ENode;
use crate::carg::v2::meta::sample_timer::SampleTimer;
use crate::carg::v2::meta::setting::Setting;
use crate::carg::v2::meta::system::{InitializeSystemAccessor, TSystemCategory};
use crate::carg::v2::meta::tick::TTimeTickCategory;
use crate::carg::v2::node::common::{EProcessState, ProcessControlItemSetting};
use crate::nz_define_time_tick_for;
use crate::wave::container::WaveContainer;
use crate::wave::sample::UniformedSample;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaIRConvInfo {
    /// IR.wavファイルのパス
    pub path: String,
}

#[derive(Debug)]
pub struct IRConvolutionProcessData {
    setting: Setting,
    common: ProcessControlItem,
    info: MetaIRConvInfo,
    internal: InternalInfo,
    timer: SampleTimer,
}

#[derive(Default, Debug)]
struct InternalInfo {
    container: Option<WaveContainer>,
    /// PCMのサンプルレート
    sample_rate: usize,
    /// 現在のIRの最初にあたる入力サンプルのインデックスカーソル。
    /// デフォ値は0。
    input_now_index: usize,
}

const INPUT_SOURCE: &'static str = "in";
const OUTPUT_OUT: &'static str = "out";

impl TPinCategory for IRConvolutionProcessData {
    fn get_input_pin_names() -> Vec<&'static str> { vec![INPUT_SOURCE] }

    fn get_output_pin_names() -> Vec<&'static str> {
        vec![OUTPUT_OUT]
    }

    fn get_pin_categories(pin_name: &str) -> Option<EPinCategoryFlag> {
        match pin_name {
            INPUT_SOURCE => Some(pin_category::BUFFER_MONO),
            OUTPUT_OUT => Some(pin_category::BUFFER_MONO),
            _ => None,
        }
    }

    fn get_input_container_flag(pin_name: &str) -> Option<EInputContainerCategoryFlag> {
        match pin_name {
            INPUT_SOURCE => Some(input::container_category::BUFFER_MONO_DYNAMIC),
            _ => None,
        }
    }
}

impl TSystemCategory for IRConvolutionProcessData {}
nz_define_time_tick_for!(IRConvolutionProcessData, true, true);

impl TProcess for IRConvolutionProcessData {
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

        match self.common.state {
            EProcessState::Stopped | EProcessState::Playing => self.update_state(input),
            _ => (),
        }
    }
}

impl TProcessItem for IRConvolutionProcessData {
    fn can_create_item(_setting: &ProcessItemCreateSetting) -> anyhow::Result<()> {
        Ok(())
    }

    fn create_item(setting: &ProcessItemCreateSetting, system_setting: &InitializeSystemAccessor) -> anyhow::Result<TProcessItemPtr> {
        if let ENode::FilterIRConvolution(v) = setting.node {
            let item = Self {
                setting: setting.setting.clone(),
                common: ProcessControlItem::new(ProcessControlItemSetting {
                    specifier: ENodeSpecifier::FilterIRConvolution,
                    systems: &system_setting,
                }),
                info: v.clone(),
                internal: Default::default(),
                timer: SampleTimer::new(0.0),
            };

            return Ok(SItemSPtr::new(item));
        }

        unreachable!("Unexpected branch");
    }
}

impl IRConvolutionProcessData {
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

    fn update_state(&mut self, input: &ProcessProcessorInput) {
        let sample_rate = self.internal.sample_rate;
        let time_result = self.timer.process_time(input.common.frame_time, sample_rate);
        if time_result.required_sample_count <= 0 {
            return;
        }

        // バッファを出力する。
        let result = self.next_samples(input, time_result.required_sample_count);
        self.common
            .insert_to_output_pin(
                OUTPUT_OUT,
                EProcessOutput::BufferMono(ProcessOutputBuffer::new(result.buffer, self.internal.sample_rate)),
            )
            .unwrap();

        if result.is_finished && input.is_children_all_finished() {
            self.common.state = EProcessState::Finished;
            return;
        } else {
            self.common.state = EProcessState::Playing;
            return;
        }
    }

    fn next_samples(&mut self, in_input: &ProcessProcessorInput, sample_count: usize) -> DrainBufferResult {
        debug_assert!(sample_count > 0);

        let mut input_internal = self.common.get_input_internal_mut(INPUT_SOURCE).unwrap();
        let input = input_internal.buffer_mono_dynamic_mut().unwrap();
        if !input.can_process() {
            // からっぽを供給する。
            return DrainBufferResult {
                buffer: vec![UniformedSample::MIN; sample_count],
                is_finished: false,
            };
        }

        // ここから問題。
        // input_start_iがあればいい。
        let input_start_i = self.internal.input_now_index;
        let mut result = vec![UniformedSample::MIN; sample_count];

        // 前分
        let ir = self.internal.container.as_ref().unwrap().uniformed_sample_buffer();
        let ir_end_i = ir.len();
        let mut remove_count = 0;
        for sample_i in 0..input_start_i {
            // もしsample_iにかけるIRフィルターの範囲がOOBなら、何もしない。(startだけみる）
            // ので、`input_start_i`に近づけば近づくほど、irの最初スタートはDirectに戻る。
            let ir_start_i = input_start_i - sample_i;
            if ir_start_i >= ir_end_i {
                remove_count += 1;
                continue;
            }

            let sample = input.buffer[sample_i];
            let ir_length = (ir_end_i - ir_start_i).min(sample_count);

            // すすめさせる。
            // sample_count分（ir尺が足りる限り）埋めつくす。
            // resultスタートは必ず0から。
            for target_i in 0..ir_length {
                let ir_i = ir_start_i + target_i;
                result[target_i] += UniformedSample::from_f64(sample.to_f64() * ir[ir_i].to_f64());
            }
        }

        // ここからはir_lengthが縮む。resultスタートも1こずつ前にすすむ。
        for sample_i in 0..sample_count {
            let input_i = input_start_i + sample_i;
            let sample = input.buffer[input_i];

            let ir_length = sample_count - sample_i;
            for target_i in 0..ir_length {
                let ir_i = target_i;
                let result_i = target_i + sample_i;

                result[result_i] += UniformedSample::from_f64(sample.to_f64() * ir[ir_i].to_f64());
            }
        }
        self.internal.input_now_index += sample_count;

        // IR影響外になったsampleは除去する。
        if remove_count > 0 {
            input.buffer.drain(..remove_count);
            self.internal.input_now_index -= remove_count;
        }
        let is_input_empty = input.buffer.is_empty();

        // 空白はいれない。
        DrainBufferResult {
            buffer: result,
            is_finished: is_input_empty && in_input.is_children_all_finished(),
        }
    }
}

#[derive(Default)]
struct DrainBufferResult {
    buffer: Vec<UniformedSample>,
    is_finished: bool,
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
