pub mod input;
pub mod node;
pub mod output;
pub mod relation;

use crate::carg::v2::adapter::envelope_ad::AdapterEnvelopeAdProcessData;
use crate::carg::v2::adapter::envelope_adsr::AdapterEnvelopeAdsrProcessData;
use crate::carg::v2::analyzer::AnalyzerDFSProcessData;
use crate::carg::v2::emitter::idft::IDFTEmitterProcessData;
use crate::carg::v2::emitter::oscilo::SineWaveEmitterProcessData;
use crate::carg::v2::meta::input::EInputContainerCategoryFlag;
use crate::carg::v2::output::output_file::OutputFileProcessData;
use crate::carg::v2::output::output_log::OutputLogProcessData;
use crate::carg::v2::special::dummy::DummyProcessData;
use crate::carg::v2::special::start::StartProcessData;
use crate::carg::v2::{ENode, NodePinItem, NodePinItemList};
use num_traits::Zero;

/// ピンのカテゴリのビットフラグ
pub mod pin_category {
    /// グラフのスタートピン
    pub const START: u32 = 1 << 0;
    /// 音波バッファが保持できる
    pub const WAVE_BUFFER: u32 = 1 << 2;
    /// ただのテキストが保持できる
    pub const TEXT: u32 = 1 << 3;

    /// 周波数情報を保持する。
    pub const FREQUENCY: u32 = 1 << 4;
    /// ダミー
    pub const DUMMY: u32 = WAVE_BUFFER | TEXT | FREQUENCY;
}

/// [`pin_category`]のフラグ制御の補助タイプ
pub type EPinCategoryFlag = u32;

/// [`pin_category`]関連の関数を提供する。
pub struct SPinCategory;
impl SPinCategory {
    /// `input`が`output`をサポートするか？
    pub fn can_support(output: EPinCategoryFlag, input: EPinCategoryFlag) -> bool {
        !(input & output).is_zero()
    }
}

/// `EPinCategoryFlag`関連のtrait。
pub trait TPinCategory {
    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの入力側のピンの名前を返す。
    fn get_input_pin_names() -> Vec<&'static str>;

    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの出力側のピンの名前を返す。
    fn get_output_pin_names() -> Vec<&'static str>;

    /// 関係ノードに書いているピンのカテゴリ（複数可）を返す。
    fn get_pin_categories(pin_name: &str) -> Option<EPinCategoryFlag>;

    /// Inputピンのコンテナフラグ
    fn get_input_container_flag(pin_name: &str) -> Option<EInputContainerCategoryFlag>;
}

/// 内部識別処理に使うEnum。
#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub enum ENodeSpecifier {
    InternalStartPin,
    InternalDummy,
    EmitterPinkNoise,
    EmitterWhiteNoise,
    EmitterSineWave,
    EmitterSawtooth,
    EmitterTriangle,
    EmitterSquare,
    EmitterIDFT,
    AnalyzerDFT,
    AdapterEnvelopeAd,
    AdapterEnvelopeAdsr,
    OutputFile,
    OutputLog,
}

impl ENodeSpecifier {
    /// 変換する
    pub fn from_node(node: &ENode) -> Self {
        match node {
            ENode::InternalStartPin => Self::InternalStartPin,
            ENode::InternalDummy => Self::InternalDummy,
            ENode::EmitterPinkNoise { .. } => Self::EmitterPinkNoise,
            ENode::EmitterWhiteNoise { .. } => Self::EmitterWhiteNoise,
            ENode::EmitterSineWave { .. } => Self::EmitterSineWave,
            ENode::EmitterSawtooth { .. } => Self::EmitterSawtooth,
            ENode::EmitterTriangle { .. } => Self::EmitterTriangle,
            ENode::EmitterSquare { .. } => Self::EmitterSquare,
            ENode::EmitterIDFT { .. } => Self::EmitterIDFT,
            ENode::AnalyzerDFT { .. } => Self::AnalyzerDFT,
            ENode::AdapterEnvlopeAd { .. } => Self::AdapterEnvelopeAd,
            ENode::AdapterEnvlopeAdsr { .. } => Self::AdapterEnvelopeAdsr,
            ENode::OutputFile { .. } => Self::OutputFile,
            ENode::OutputLog { .. } => Self::OutputLog,
        }
    }

    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの入力側のピンのリストを生成して返す。
    pub fn create_input_pins(&self) -> NodePinItemList {
        let names = match self {
            Self::InternalStartPin => StartProcessData::get_input_pin_names(),
            Self::InternalDummy => DummyProcessData::get_input_pin_names(),
            Self::AdapterEnvelopeAd => AdapterEnvelopeAdProcessData::get_input_pin_names(),
            Self::AdapterEnvelopeAdsr => AdapterEnvelopeAdsrProcessData::get_input_pin_names(),
            Self::EmitterPinkNoise
            | Self::EmitterSawtooth
            | Self::EmitterSquare
            | Self::EmitterTriangle
            | Self::EmitterWhiteNoise
            | Self::EmitterSineWave => SineWaveEmitterProcessData::get_input_pin_names(),
            Self::AnalyzerDFT => AnalyzerDFSProcessData::get_input_pin_names(),
            Self::OutputFile => OutputFileProcessData::get_input_pin_names(),
            Self::OutputLog => OutputLogProcessData::get_input_pin_names(),
            Self::EmitterIDFT => IDFTEmitterProcessData::get_input_pin_names(),
        };

        let mut map = NodePinItemList::new();
        for name in names {
            map.insert(
                name.to_owned(),
                NodePinItem::new_item(name, self.get_pin_categories(name).unwrap(), false, self.get_input_flag(name)),
            );
        }
        map
    }

    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの出力側のピンのリストを生成して返す。
    pub fn create_output_pins(&self) -> NodePinItemList {
        let names = match self {
            Self::InternalStartPin => StartProcessData::get_output_pin_names(),
            Self::InternalDummy => DummyProcessData::get_output_pin_names(),
            Self::AdapterEnvelopeAd => AdapterEnvelopeAdProcessData::get_output_pin_names(),
            Self::AdapterEnvelopeAdsr => AdapterEnvelopeAdsrProcessData::get_output_pin_names(),
            Self::EmitterPinkNoise
            | Self::EmitterSawtooth
            | Self::EmitterSquare
            | Self::EmitterTriangle
            | Self::EmitterWhiteNoise
            | Self::EmitterSineWave => SineWaveEmitterProcessData::get_output_pin_names(),
            Self::AnalyzerDFT => AnalyzerDFSProcessData::get_output_pin_names(),
            Self::OutputFile => OutputFileProcessData::get_output_pin_names(),
            Self::OutputLog => OutputLogProcessData::get_output_pin_names(),
            Self::EmitterIDFT => IDFTEmitterProcessData::get_output_pin_names(),
        };

        let mut map = NodePinItemList::new();
        for name in names {
            map.insert(
                name.to_owned(),
                NodePinItem::new_item(
                    name,
                    self.get_pin_categories(name).expect(&format!("{}", name)),
                    true,
                    input::container_category::UNINITIALIZED,
                ),
            );
        }
        map
    }

    /// 自分のinputピンに`pin_name`と一致する名前のピンがあるか？
    pub fn is_valid_input_pin(&self, pin_name: &str) -> bool {
        let names = match self {
            Self::InternalStartPin => StartProcessData::get_input_pin_names(),
            Self::InternalDummy => DummyProcessData::get_input_pin_names(),
            Self::AdapterEnvelopeAd => AdapterEnvelopeAdProcessData::get_input_pin_names(),
            Self::AdapterEnvelopeAdsr => AdapterEnvelopeAdsrProcessData::get_input_pin_names(),
            Self::EmitterPinkNoise
            | Self::EmitterSawtooth
            | Self::EmitterSquare
            | Self::EmitterTriangle
            | Self::EmitterWhiteNoise
            | Self::EmitterSineWave => SineWaveEmitterProcessData::get_input_pin_names(),
            Self::AnalyzerDFT => AnalyzerDFSProcessData::get_input_pin_names(),
            Self::OutputFile => OutputFileProcessData::get_input_pin_names(),
            Self::OutputLog => OutputLogProcessData::get_input_pin_names(),
            Self::EmitterIDFT => IDFTEmitterProcessData::get_input_pin_names(),
        };
        if names.is_empty() {
            return false;
        }
        names.contains(&pin_name)
    }

    /// 自分のoutputピンに`pin_name`と一致する名前のピンがあるか？
    pub fn is_valid_output_pin(&self, pin_name: &str) -> bool {
        let names = match self {
            Self::InternalStartPin => StartProcessData::get_output_pin_names(),
            Self::InternalDummy => DummyProcessData::get_output_pin_names(),
            Self::AdapterEnvelopeAd => AdapterEnvelopeAdProcessData::get_output_pin_names(),
            Self::AdapterEnvelopeAdsr => AdapterEnvelopeAdsrProcessData::get_output_pin_names(),
            Self::EmitterPinkNoise
            | Self::EmitterSawtooth
            | Self::EmitterSquare
            | Self::EmitterTriangle
            | Self::EmitterWhiteNoise
            | Self::EmitterSineWave => SineWaveEmitterProcessData::get_output_pin_names(),
            Self::AnalyzerDFT => AnalyzerDFSProcessData::get_output_pin_names(),
            Self::OutputFile => OutputFileProcessData::get_output_pin_names(),
            Self::OutputLog => OutputLogProcessData::get_output_pin_names(),
            Self::EmitterIDFT => IDFTEmitterProcessData::get_output_pin_names(),
        };
        if names.is_empty() {
            return false;
        }
        names.contains(&pin_name)
    }

    /// 関係ノードに書いているピンのカテゴリ（複数可）を返す。
    pub fn get_pin_categories(&self, pin_name: &str) -> Option<EPinCategoryFlag> {
        match self {
            Self::InternalStartPin => StartProcessData::get_pin_categories(pin_name),
            Self::InternalDummy => DummyProcessData::get_pin_categories(pin_name),
            Self::AdapterEnvelopeAd => AdapterEnvelopeAdProcessData::get_pin_categories(pin_name),
            Self::AdapterEnvelopeAdsr => AdapterEnvelopeAdsrProcessData::get_pin_categories(pin_name),
            Self::EmitterPinkNoise
            | Self::EmitterSawtooth
            | Self::EmitterSquare
            | Self::EmitterTriangle
            | Self::EmitterWhiteNoise
            | Self::EmitterSineWave => SineWaveEmitterProcessData::get_pin_categories(pin_name),
            Self::AnalyzerDFT => AnalyzerDFSProcessData::get_pin_categories(pin_name),
            Self::OutputFile => OutputFileProcessData::get_pin_categories(pin_name),
            Self::OutputLog => OutputLogProcessData::get_pin_categories(pin_name),
            Self::EmitterIDFT => IDFTEmitterProcessData::get_pin_categories(pin_name),
        }
    }

    /// Inputピンである場合、そのInputピンのフラグを返す。
    pub fn get_input_flag(&self, pin_name: &str) -> EInputContainerCategoryFlag {
        match self {
            Self::InternalStartPin => StartProcessData::get_input_container_flag(pin_name),
            Self::InternalDummy => DummyProcessData::get_input_container_flag(pin_name),
            Self::AdapterEnvelopeAd => AdapterEnvelopeAdProcessData::get_input_container_flag(pin_name),
            Self::AdapterEnvelopeAdsr => AdapterEnvelopeAdsrProcessData::get_input_container_flag(pin_name),
            Self::EmitterPinkNoise
            | Self::EmitterSawtooth
            | Self::EmitterSquare
            | Self::EmitterTriangle
            | Self::EmitterWhiteNoise
            | Self::EmitterSineWave => SineWaveEmitterProcessData::get_input_container_flag(pin_name),
            Self::AnalyzerDFT => AnalyzerDFSProcessData::get_input_container_flag(pin_name),
            Self::OutputFile => OutputFileProcessData::get_input_container_flag(pin_name),
            Self::OutputLog => OutputLogProcessData::get_input_container_flag(pin_name),
            Self::EmitterIDFT => IDFTEmitterProcessData::get_input_container_flag(pin_name),
        }
        .unwrap()
    }
}
