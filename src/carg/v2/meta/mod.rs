pub mod input;
pub mod output;
pub mod relation;

use crate::carg::v2::meta::input::EInputContainerCategoryFlag;
use crate::carg::v2::{ENode, NodePinItem, NodePinItemList};
use num_traits::Zero;

/// ピンのカテゴリのビットフラグ
pub mod pin_category {
    /// グラフのスタートピン
    pub const START: u32 = 1 << 0;
    /// 音波バッファが保持できる
    pub const WAVE_BUFFER: u32 = 1 << 1;
    /// ただのテキストが保持できる
    pub const TEXT: u32 = 1 << 2;

    /// 周波数情報を保持する。
    pub const FREQUENCY: u32 = 1 << 3;
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

/// 内部識別処理に使うEnum。
#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub enum ENodeSpecifier {
    InternalStartPin,
    EmitterPinkNoise,
    EmitterWhiteNoise,
    EmitterSineWave,
    EmitterSawtooth,
    EmitterTriangle,
    EmitterSquare,
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
            ENode::EmitterPinkNoise { .. } => Self::EmitterPinkNoise,
            ENode::EmitterWhiteNoise { .. } => Self::EmitterWhiteNoise,
            ENode::EmitterSineWave { .. } => Self::EmitterSineWave,
            ENode::EmitterSawtooth { .. } => Self::EmitterSawtooth,
            ENode::EmitterTriangle { .. } => Self::EmitterTriangle,
            ENode::EmitterSquare { .. } => Self::EmitterSquare,
            ENode::AnalyzerDFT { .. } => Self::AnalyzerDFT,
            ENode::AdapterEnvlopeAd { .. } => Self::AdapterEnvelopeAd,
            ENode::AdapterEnvlopeAdsr { .. } => Self::AdapterEnvelopeAdsr,
            ENode::OutputFile { .. } => Self::OutputFile,
            ENode::OutputLog { .. } => Self::OutputLog,
        }
    }

    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの入力側のピンのリストを生成して返す。
    pub fn create_input_pins(&self) -> NodePinItemList {
        let mut map = NodePinItemList::new();

        match self {
            Self::InternalStartPin => (),
            Self::EmitterPinkNoise
            | Self::EmitterSawtooth
            | Self::EmitterSquare
            | Self::EmitterTriangle
            | Self::EmitterWhiteNoise
            | Self::EmitterSineWave
            | Self::AdapterEnvelopeAd
            | Self::AdapterEnvelopeAdsr
            | Self::AnalyzerDFT
            | Self::OutputFile
            | Self::OutputLog => {
                map.insert(
                    "in".to_owned(),
                    NodePinItem::new_item(
                        "in",
                        self.get_pin_categories("in").unwrap(),
                        false,
                        self.get_input_flag("in"),
                    ),
                );
            }
        }

        map
    }

    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの出力側のピンのリストを生成して返す。
    pub fn create_output_pins(&self) -> NodePinItemList {
        let mut map = NodePinItemList::new();

        match self {
            Self::InternalStartPin
            | Self::EmitterPinkNoise
            | Self::EmitterSawtooth
            | Self::EmitterSquare
            | Self::EmitterTriangle
            | Self::EmitterWhiteNoise
            | Self::EmitterSineWave
            | Self::AdapterEnvelopeAd
            | Self::AdapterEnvelopeAdsr => {
                map.insert(
                    "out".to_owned(),
                    NodePinItem::new_item(
                        "out",
                        self.get_pin_categories("out").unwrap(),
                        true,
                        input::container_category::UNINITIALIZED,
                    ),
                );
            }
            Self::AnalyzerDFT => {
                map.insert(
                    "out_info".to_owned(),
                    NodePinItem::new_item(
                        "out_info",
                        self.get_pin_categories("out_info").unwrap(),
                        true,
                        input::container_category::UNINITIALIZED,
                    ),
                );
                map.insert(
                    "out_freq".to_owned(),
                    NodePinItem::new_item(
                        "out_freq",
                        self.get_pin_categories("out_freq").unwrap(),
                        true,
                        input::container_category::UNINITIALIZED,
                    ),
                );
            }
            _ => {}
        }

        map
    }

    /// 自分のinputピンに`pin_name`と一致する名前のピンがあるか？
    pub fn is_valid_input_pin(&self, pin_name: &str) -> bool {
        match self {
            Self::EmitterPinkNoise
            | Self::EmitterSawtooth
            | Self::EmitterSquare
            | Self::EmitterTriangle
            | Self::EmitterWhiteNoise
            | Self::EmitterSineWave
            | Self::AdapterEnvelopeAd
            | Self::AdapterEnvelopeAdsr
            | Self::AnalyzerDFT
            | Self::OutputFile
            | Self::OutputLog => match pin_name {
                "in" => true,
                &_ => false,
            },
            _ => false,
        }
    }

    /// 自分のoutputピンに`pin_name`と一致する名前のピンがあるか？
    pub fn is_valid_output_pin(&self, pin_name: &str) -> bool {
        match self {
            Self::InternalStartPin
            | Self::EmitterPinkNoise
            | Self::EmitterSawtooth
            | Self::EmitterSquare
            | Self::EmitterTriangle
            | Self::EmitterWhiteNoise
            | Self::EmitterSineWave
            | Self::AdapterEnvelopeAd
            | Self::AdapterEnvelopeAdsr => match pin_name {
                "out" => true,
                &_ => false,
            },
            Self::AnalyzerDFT => match pin_name {
                "out_freq" => true,
                "out_info" => true,
                &_ => false,
            },
            _ => false,
        }
    }

    /// 関係ノードに書いているピンのカテゴリ（複数可）を返す。
    pub fn get_pin_categories(&self, pin_name: &str) -> Option<EPinCategoryFlag> {
        match self {
            Self::InternalStartPin => match pin_name {
                "out" => Some(pin_category::START),
                &_ => None,
            },
            Self::AdapterEnvelopeAd | Self::AdapterEnvelopeAdsr => match pin_name {
                "in" => Some(pin_category::WAVE_BUFFER),
                "out" => Some(pin_category::WAVE_BUFFER),
                &_ => None,
            },
            Self::EmitterPinkNoise
            | Self::EmitterSawtooth
            | Self::EmitterSquare
            | Self::EmitterTriangle
            | Self::EmitterWhiteNoise
            | Self::EmitterSineWave => match pin_name {
                "in" => Some(pin_category::START),
                "out" => Some(pin_category::WAVE_BUFFER),
                &_ => None,
            },
            Self::AnalyzerDFT => match pin_name {
                "in" => Some(pin_category::WAVE_BUFFER),
                "out_freq" => Some(pin_category::FREQUENCY),
                "out_info" => Some(pin_category::TEXT),
                &_ => None,
            },
            Self::OutputFile => match pin_name {
                "in" => Some(pin_category::WAVE_BUFFER),
                &_ => None,
            },
            Self::OutputLog => match pin_name {
                "in" => Some(pin_category::TEXT | pin_category::WAVE_BUFFER),
                &_ => None,
            },
        }
    }

    /// Inputピンである場合、そのInputピンのフラグを返す。
    pub fn get_input_flag(&self, _pin_name: &str) -> EInputContainerCategoryFlag {
        match self {
            ENodeSpecifier::InternalStartPin
            | ENodeSpecifier::EmitterPinkNoise
            | ENodeSpecifier::EmitterWhiteNoise
            | ENodeSpecifier::EmitterSineWave
            | ENodeSpecifier::EmitterSawtooth
            | ENodeSpecifier::EmitterTriangle
            | ENodeSpecifier::EmitterSquare => input::container_category::EMPTY,
            ENodeSpecifier::AdapterEnvelopeAd | ENodeSpecifier::AdapterEnvelopeAdsr => input::container_category::WAVE_BUFFER_PHANTOM,
            ENodeSpecifier::AnalyzerDFT => input::container_category::WAVE_BUFFERS_DYNAMIC,
            ENodeSpecifier::OutputFile => input::container_category::WAVE_BUFFERS_DYNAMIC,
            ENodeSpecifier::OutputLog => input::container_category::OUTPUT_LOG,
        }
    }
}
