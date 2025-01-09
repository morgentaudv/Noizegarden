use crate::carg::v2::meta::input::EInputContainerCategoryFlag;
use crate::carg::v2::meta::node::ENode;
use crate::carg::v2::meta::sample_timer::SampleTimer;
use crate::carg::v2::meta::setting::Setting;
use crate::carg::v2::meta::system::{InitializeSystemAccessor, TSystemCategory};
use crate::carg::v2::meta::tick::TTimeTickCategory;
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::node::common::{EProcessState, ProcessControlItemSetting};
use crate::carg::v2::{
    EProcessOutput, ProcessControlItem, ProcessItemCreateSetting, ProcessOutputBuffer, ProcessProcessorInput,
    SItemSPtr, TProcess, TProcessItem, TProcessItemPtr,
};
use crate::nz_define_time_tick_for;
use crate::wave::container::WaveContainer;
use crate::wave::sample::UniformedSample;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::BufReader;

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
    /// このノードが停止できる時点での`input_now_index`。
    /// これに値があれば、ずっと処理を続けさせて-1以下に達したらノードの終了ができる。
    last_before_finish_index: Option<isize>,
}

const INPUT_SOURCE: &'static str = "in";
const OUTPUT_OUT: &'static str = "out";

impl TPinCategory for IRConvolutionProcessData {
    fn get_input_pin_names() -> Vec<&'static str> {
        vec![INPUT_SOURCE]
    }

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

    fn create_item(
        setting: &ProcessItemCreateSetting,
        system_setting: &InitializeSystemAccessor,
    ) -> anyhow::Result<TProcessItemPtr> {
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

        // 移動して終わり。
        self.internal.sample_rate = container.samples_per_second() as usize;
        self.internal.container = Some(container);
    }

    fn update_state(&mut self, input: &ProcessProcessorInput) {
        let sample_rate = self.internal.sample_rate;
        let time_result = self.timer.process_time(input.common.frame_time, sample_rate);
        if time_result.required_sample_count <= 0 {
            self.common.state = EProcessState::Playing;
            return;
        }

        // バッファを出力する。
        let result = self.next_samples(input, time_result.required_sample_count.min(self.setting.sample_count_frame));
        self.common
            .insert_to_output_pin(
                OUTPUT_OUT,
                EProcessOutput::BufferMono(ProcessOutputBuffer::new(result.buffer, self.internal.sample_rate)),
            )
            .unwrap();

        if result.is_finished {
            self.common.state = EProcessState::Finished;
        } else {
            self.common.state = EProcessState::Playing;
        }
    }

    fn next_samples(&mut self, in_input: &ProcessProcessorInput, sample_count: usize) -> DrainBufferResult {
        let all_finished = in_input.is_children_all_finished();

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
        let ir_length = ir.len();

        let mut remove_count: usize = 0;
        for sample_i in 0..input_start_i {
            // もしsample_iにかけるIRフィルターの範囲がOOBなら、何もしない。(startだけみる）
            // ので、`input_start_i`に近づけば近づくほど、irの最初スタートはDirectに戻る。
            let ir_start_i = input_start_i - sample_i;
            if ir_start_i >= ir_length {
                remove_count += 1;
                continue;
            }

            let sample = input.buffer[sample_i];
            if sample.to_f64() == 0.0 {
                // 0なら処理しない。
                continue;
            }

            // すすめさせる。
            // sample_count分（ir尺が足りる限り）埋めつくす。
            // resultスタートは必ず0から。
            let ir_length = (ir_length - ir_start_i).min(sample_count);
            for target_i in 0..ir_length {
                let ir_i = ir_start_i + target_i;
                let new_sample = sample.to_f64() * ir[ir_i].to_f64() * 0.2;
                result[target_i] += UniformedSample::from_f64(new_sample);
            }
        }

        // 25-01-09 バッファの0埋め処理。
        {
            let old_input_buffer_len = input.buffer.len();
            let processable_len = old_input_buffer_len - input_start_i;
            if sample_count > processable_len {
                let offset_len = sample_count - processable_len;
                let total_buffer_length = input_start_i + processable_len + offset_len;

                input.buffer.resize(total_buffer_length, UniformedSample::MIN);
            }
        }

        // ここからはir_lengthが縮む。resultスタートも1こずつ前にすすむ。
        for sample_i in 0..sample_count {
            let input_i = input_start_i + sample_i;
            let sample = input.buffer[input_i];
            if sample.to_f64() == 0.0 {
                // 0なら処理しない。
                continue;
            }

            let ir_length = (sample_count - sample_i).min(ir_length);
            for target_i in 0..ir_length {
                let ir_i = target_i;
                let result_i = target_i + sample_i;

                let new_sample = sample.to_f64() * ir[ir_i].to_f64() * 0.2;
                result[result_i] += UniformedSample::from_f64(new_sample);
            }
        }
        self.internal.input_now_index += sample_count;

        // IR影響外になったsampleは除去する。
        if remove_count > 0 {
            input.buffer.drain(..remove_count);
            self.internal.input_now_index -= remove_count;
        }

        // もし`all_finished == false`なら、サンプルカウントに対し足りない分は0埋めする。
        // ちょっと窮屈だけど、今の仕組みではしようがないか。
        let mut is_finished = false;
        if all_finished {
            if self.internal.last_before_finish_index.is_none() {
                self.internal.last_before_finish_index = Some(input.buffer.len() as isize);
            } else {
                let mut remaining_index = self.internal.last_before_finish_index.as_mut().unwrap();
                *remaining_index -= remove_count as isize;

                if *remaining_index < 0 {
                    is_finished = true;
                }
            }
        }

        // 空白はいれない。
        DrainBufferResult {
            buffer: result,
            is_finished,
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
