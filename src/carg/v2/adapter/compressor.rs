use itertools::Itertools;
use serde::{Deserialize, Serialize};
use crate::carg::v2::meta::setting::Setting;
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::meta::input::EInputContainerCategoryFlag;
use crate::carg::v2::{EProcessOutput, ProcessControlItem, ProcessOutputBuffer, ProcessProcessorInput, SItemSPtr, TProcess, TProcessItemPtr};
use crate::carg::v2::meta::node::ENode;
use crate::carg::v2::meta::output::EProcessOutputContainer;
use crate::carg::v2::meta::system::TSystemCategory;
use crate::carg::v2::node::common::EProcessState;
use crate::wave::EBitDepth;
use crate::wave::sample::UniformedSample;

/// Compressorノードの設定入力情報
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaCompressorInfo {
    /// Compressor動作の基準dB
    pub threshold_db: f64,
    /// 遷移帯域幅の総周波数範囲
    pub makeup_gain_db: f64,
    /// `threshold_db`前後の和らげさのdB範囲
    pub knee_width_db: f64,
    /// `threshold_db`からのボリュームの増加比率の分母
    pub ratio: f64,
    /// 基準Depth
    pub bit_depth: EBitDepth,
}

#[derive(Debug)]
pub struct AdapterCompressorProcessData {
    setting: Setting,
    common: ProcessControlItem,
    info: MetaCompressorInfo,
}

const INPUT_IN: &'static str = "in";
const OUTPUT_OUT: &'static str = "out";

impl TPinCategory for AdapterCompressorProcessData {
    fn get_input_pin_names() -> Vec<&'static str> {
        vec![INPUT_IN]
    }

    fn get_output_pin_names() -> Vec<&'static str> {
        vec![OUTPUT_OUT]
    }

    fn get_pin_categories(pin_name: &str) -> Option<EPinCategoryFlag> {
        match pin_name {
            INPUT_IN => Some(pin_category::BUFFER_MONO),
            OUTPUT_OUT => Some(pin_category::BUFFER_MONO),
            _ => None,
        }
    }

    fn get_input_container_flag(pin_name: &str) -> Option<EInputContainerCategoryFlag> {
        match pin_name {
            INPUT_IN => Some(input::container_category::BUFFER_MONO_PHANTOM),
            _ => None,
        }
    }
}

impl TSystemCategory for AdapterCompressorProcessData {}

impl TProcess for AdapterCompressorProcessData {
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

        match self.common.state {
            EProcessState::Stopped | EProcessState::Playing => self.update_state(input),
            _ => (),
        }
    }
}

impl AdapterCompressorProcessData {
    pub fn create_from(node: &ENode, setting: &Setting) -> TProcessItemPtr {
        if let ENode::AdapterCompressor(v) = node {
            let item= Self {
                setting: setting.clone(),
                common: ProcessControlItem::new(ENodeSpecifier::AdapterCompressor),
                info: v.clone(),
            };

            return SItemSPtr::new(item);
        }

        unreachable!("Unexpected branch");
    }

    pub fn update_state(&mut self, in_input: &ProcessProcessorInput) {
        // Inputがなきゃ何もできぬ。
        // これなに…
        let linked_output_pin = self
            .common
            .get_input_pin(INPUT_IN)
            .unwrap()
            .upgrade()
            .unwrap()
            .borrow()
            .linked_pins
            .first()
            .unwrap()
            .upgrade()
            .unwrap();

        let borrowed = linked_output_pin.borrow();
        let input = match &borrowed.output {
            EProcessOutputContainer::BufferMono(v) => v,
            _ => unreachable!("Unexpected branch"),
        };

        // 処理
        // TODO : Cubic-hermite spline補完の両端のタンジェントがおかしいかも。
        let bit_depth = self.info.bit_depth;
        let interp_min = self.info.threshold_db - self.info.knee_width_db;
        let interp_max = self.info.threshold_db + self.info.knee_width_db;
        let interp_range = 2.0 * self.info.knee_width_db;
        let output_buffer = input.buffer.iter().map(|v| {
            let is_plus = v.to_f64().is_sign_positive();
            let aligned_db = match v.apply_bit_depth(bit_depth) {
                v if v < interp_min => v,
                v if v >= interp_max => {
                    (v - self.info.threshold_db) * self.info.ratio.recip() + self.info.threshold_db
                },
                v => {
                    // cubic-hermite splineで何とかする。
                    let f = (v - interp_min) / interp_range;
                    let fpow3 = f.powf(3.0);
                    let fpow2 = f.powf(2.0);

                    let l = (2.0 * fpow3) - (3.0 * fpow2) + 1.0;
                    let m = fpow3 - (2.0 * fpow2) + f;
                    let n = (-2.0 * fpow3) + (3.0 * fpow2);
                    let o = fpow3 - fpow2;

                    let a = v;
                    let b = (v - self.info.threshold_db) * self.info.ratio.recip() + self.info.threshold_db;

                    (l * a) + m + (n * b) + (o * self.info.ratio.recip())
                }
            };

            //dbg!(v, aligned_db);
            UniformedSample::from_db(aligned_db + self.info.makeup_gain_db, bit_depth, is_plus)
        }).collect_vec();

        // outputのどこかに保持する。
        self.common
            .insert_to_output_pin(
                OUTPUT_OUT,
                EProcessOutput::BufferMono(ProcessOutputBuffer::new(output_buffer, input.setting.clone())),
            )
            .unwrap();

        if in_input.is_children_all_finished() {
            self.common.state = EProcessState::Finished;
            return;
        } else {
            self.common.state = EProcessState::Playing;
            return;
        }
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
