use crate::carg::v2::meta::input::EInputContainerCategoryFlag;
use crate::carg::v2::meta::node::ENode;
use crate::carg::v2::meta::setting::Setting;
use crate::carg::v2::meta::system::{system_category, ESystemCategoryFlag, TSystemCategory};
use crate::carg::v2::meta::tick::TTimeTickCategory;
use crate::carg::v2::meta::{input, pin_category, ENodeSpecifier, EPinCategoryFlag, TPinCategory};
use crate::carg::v2::node::common::{EProcessState, ProcessControlItem};
use crate::carg::v2::{
    EProcessOutput, ProcessItemCreateSetting, ProcessItemCreateSettingSystem, ProcessOutputBuffer,
    ProcessProcessorInput, SItemSPtr, TProcess, TProcessItem, TProcessItemPtr,
};
use crate::nz_define_time_tick_for;
use crate::resample::{
    ProcessSamplingSetting, ProcessSourceResult, ResampleHeaderSetting,
    ResampleSystemProxyWeakPtr,
};
use crate::wave::sample::UniformedSample;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetaResampleInfo {
    /// サンプルレートに変換
    pub to_sample_rate: usize,
    ///
    pub high_quality: bool,
}

#[derive(Debug)]
pub struct ResampleProcessData {
    setting: Setting,
    common: ProcessControlItem,
    resample_proxy: ResampleSystemProxyWeakPtr,
    info: MetaResampleInfo,
    /// 内部用データ
    #[allow(dead_code)]
    internal: InternalInfo,
}

const INPUT_IN: &'static str = "in";
const OUTPUT_OUT: &'static str = "out";

impl TPinCategory for ResampleProcessData {
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
            INPUT_IN => Some(input::container_category::BUFFER_MONO_DYNAMIC),
            _ => None,
        }
    }
}

impl TSystemCategory for ResampleProcessData {
    fn get_dependent_system_categories() -> ESystemCategoryFlag {
        system_category::RESAMPLE_SYSTEM
    }
}
nz_define_time_tick_for!(ResampleProcessData, true, true);

impl TProcess for ResampleProcessData {
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

        if self.common.is_state(EProcessState::Finished) {
            return;
        }

        if self.common.is_state(EProcessState::Stopped) {
            // inputが入っているときだけ初期化できる。
            // @todo ピンからインプットに接近して、インプットが出すFsを取得する。
            let setting = {
                let input_internal = self.common.get_input_internal(INPUT_IN).unwrap();
                let input = input_internal.buffer_mono_dynamic().unwrap();
                // もしインプットがきてなくて、Fsがセットされたなきゃなんもしない。
                if input.sample_rate == 0 {
                    return;
                }

                InitializeSetting {
                    from_fs: input.sample_rate,
                    to_fs: self.info.to_sample_rate,
                    is_high_quality: self.info.high_quality,
                }
            };

            // 初期化する。
            self.initialize(&setting);
            self.common.set_state(EProcessState::Playing);
        }

        // 24-12-22 まずoverlappingなしでやってみる。
        let to_fs = self.info.to_sample_rate;
        let (input_buffer, is_last) = self.drain_buffer(&input);
        if input_buffer.is_empty() {
            return;
        }

        let result = self.process_resample(&input_buffer, is_last, self.internal.next_phase_time);
        self.internal.next_phase_time = result.next_phase_time;

        self.common
            .insert_to_output_pin(
                OUTPUT_OUT,
                EProcessOutput::BufferMono(ProcessOutputBuffer::new(result.outputs, to_fs)),
            )
            .unwrap();

        // 状態確認
        if is_last && input.is_children_all_finished() {
            self.common.state = EProcessState::Finished;
        } else {
            self.common.state = EProcessState::Playing;
        }
    }
}

impl TProcessItem for ResampleProcessData {
    fn can_create_item(_setting: &ProcessItemCreateSetting) -> anyhow::Result<()> {
        Ok(())
    }

    fn create_item(
        setting: &ProcessItemCreateSetting,
        system_setting: &ProcessItemCreateSettingSystem,
    ) -> anyhow::Result<TProcessItemPtr> {
        if let ENode::AdapterResample(v) = setting.node {
            let item = Self {
                setting: setting.setting.clone(),
                common: ProcessControlItem::new(ENodeSpecifier::AdapterResample),
                resample_proxy: system_setting.resample_system.unwrap().clone(),
                info: v.clone(),
                internal: Default::default(),
            };

            return Ok(SItemSPtr::new(item));
        }
        unreachable!("Unexpected branch");
    }
}

impl ResampleProcessData {
    /// ノードの最初の初期化を行う。
    fn initialize(&mut self, setting: &InitializeSetting) {
        self.internal.from_fs = Some(setting.from_fs);

        let system = self.resample_proxy.upgrade();
        assert!(system.is_some());

        // あとで特定のIRに接近するために保持する。
        let setting = ResampleHeaderSetting {
            from_fs: setting.from_fs,
            to_fs: setting.to_fs,
            is_high_quality: setting.is_high_quality,
        };
        self.internal.ir_setting = Some(setting.clone());

        // IRの生成。
        {
            let system = system.unwrap();
            let mut system = system.lock().unwrap();
            system.create_response(&setting);
        }
    }

    fn drain_buffer(&mut self, in_input: &ProcessProcessorInput) -> (Vec<UniformedSample>, bool) {
        // 24-12-22 まずソースのサンプルサイズ（1チャンネル）は固定にしてみる。
        const SRC_SAMPLE_LEN: usize = 4096;

        let mut input_internal = self.common.get_input_internal_mut(INPUT_IN).unwrap();
        let input = input_internal.buffer_mono_dynamic_mut().unwrap();

        // バッファ0補充分岐
        let is_buffer_enough = input.buffer.len() >= SRC_SAMPLE_LEN;
        if !is_buffer_enough && in_input.is_children_all_finished() {
            let mut buffer = input.buffer.drain(..).collect_vec();
            buffer.resize(SRC_SAMPLE_LEN, UniformedSample::MIN);
            return (buffer, true);
        }
        if !is_buffer_enough {
            return (vec![], false);
        }

        // 普通。
        (input.buffer.drain(..SRC_SAMPLE_LEN).collect_vec(), false)
    }

    fn process_resample(
        &self,
        src_buffer: &[UniformedSample],
        _is_last: bool,
        start_phase_time: f64,
    ) -> ProcessSourceResult {
        let setting = ProcessSamplingSetting {
            src_buffer,
            use_interp: false,
            start_phase_time,
        };

        {
            let system = self.resample_proxy.upgrade();
            assert!(system.is_some());

            let system = system.unwrap();
            let system = system.lock().unwrap();

            // Account for increased filter gain when using factors less than 1.
            // Decimationするなら、ゲインを減らす必要があるっぽい？
            system
                .process_response(self.internal.ir_setting.as_ref().unwrap(), &setting)
                .unwrap()
        }
    }
}

/// 初期化の設定
#[derive(Debug, Clone)]
struct InitializeSetting {
    /// 変換のための前音波のサンプリング周波数。
    from_fs: usize,
    /// 変換後の音波のサンプリング周波数。
    to_fs: usize,
    /// 処理のクォリティーがいいか
    is_high_quality: bool,
}

// ----------------------------------------------------------------------------
// InternalInfo
// ----------------------------------------------------------------------------

#[derive(Debug)]
struct InternalInfo {
    /// サンプリング周波数は固定にする必要ある。
    /// 変動するものはサポートできない。(ADPCM?）
    from_fs: Option<usize>,
    ir_setting: Option<ResampleHeaderSetting>,
    next_phase_time: f64,
}

impl Default for InternalInfo {
    fn default() -> Self {
        Self {
            from_fs: None,
            ir_setting: None,
            next_phase_time: 0.0,
        }
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
