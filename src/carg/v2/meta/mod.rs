pub mod input;
pub mod node;
pub mod output;
pub mod relation;
pub mod setting;

use crate::carg::v2::adapter::envelope_ad::AdapterEnvelopeAdProcessData;
use crate::carg::v2::adapter::envelope_adsr::AdapterEnvelopeAdsrProcessData;
use crate::carg::v2::adapter::wave_sum::AdapterWaveSumProcessData;
use crate::carg::v2::analyzer::dft::AnalyzerDFTProcessData;
use crate::carg::v2::analyzer::fft::AnalyzerFFTProcessData;
use crate::carg::v2::emitter::idft::IDFTEmitterProcessData;
use crate::carg::v2::emitter::ifft::IFFTEmitterProcessData;
use crate::carg::v2::emitter::oscilo::SineWaveEmitterProcessData;
use crate::carg::v2::filter::fir::FIRProcessData;
use crate::carg::v2::filter::iir::IIRProcessData;
use crate::carg::v2::meta::input::EInputContainerCategoryFlag;
use crate::carg::v2::mix::stereo::MixStereoProcessData;
use crate::carg::v2::output::output_file::OutputFileProcessData;
use crate::carg::v2::output::output_log::OutputLogProcessData;
use crate::carg::v2::special::dummy::DummyProcessData;
use crate::carg::v2::special::start::StartProcessData;
use crate::carg::v2::{ENode, NodePinItem, NodePinItemList};
use num_traits::Zero;
use crate::carg::v2::adapter::compressor::AdapterCompressorProcessData;
use crate::carg::v2::adapter::limiter::AdapterLimiterProcessData;
use crate::carg::v2::analyzer::lufs::AnalyzeLUFSProcessData;
use crate::carg::v2::emitter::wav_mono::EmitterWavMonoProcessData;
use crate::carg::v2::filter::irconv::IRConvolutionProcessData;
use crate::carg::v2::output::output_device::OutputDeviceProcessData;

/// ピンのカテゴリのビットフラグ
pub mod pin_category {
    /// グラフのスタートピン
    pub const START: u32 = 1 << 0;

    /// 音波バッファが保持できる
    pub const BUFFER_MONO: u32 = 1 << 2;

    /// ステレオの音波バッファが保持できる
    pub const BUFFER_STEREO: u32 = 1 << 3;

    /// ただのテキストが保持できる
    pub const TEXT: u32 = 1 << 4;

    /// 周波数情報を保持する。
    pub const FREQUENCY: u32 = 1 << 5;

    /// ダミー
    pub const DUMMY: u32 = BUFFER_MONO | BUFFER_STEREO | TEXT | FREQUENCY;
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
    EmitterIFFT,
    EmitterWavMono,
    AnalyzerDFT,
    AnalyzerFFT,
    AnalyzerLUFS,
    AdapterEnvelopeAd,
    AdapterEnvelopeAdsr,
    AdapterWaveSum,
    AdapterCompressor,
    AdapterLimiter,
    FilterFIR,
    FilterIIRLPF,
    FilterIIRHPF,
    FilterIIRBandPass,
    FilterIIRBandStop,
    FilterIRConvolution,
    MixStereo,
    OutputFile,
    OutputLog,
    OutputDevice,
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
            ENode::EmitterIFFT { .. } => Self::EmitterIFFT,
            ENode::EmitterWavMono(_) => Self::EmitterWavMono,
            ENode::AnalyzerDFT { .. } => Self::AnalyzerDFT,
            ENode::AnalyzerFFT { .. } => Self::AnalyzerFFT,
            ENode::AnalyzerLUFS(_) => Self::AnalyzerLUFS,
            ENode::AdapterEnvelopeAd { .. } => Self::AdapterEnvelopeAd,
            ENode::AdapterEnvelopeAdsr { .. } => Self::AdapterEnvelopeAdsr,
            ENode::AdapterCompressor(_) => Self::AdapterCompressor,
            ENode::OutputFile { .. } => Self::OutputFile,
            ENode::OutputLog { .. } => Self::OutputLog,
            ENode::OutputDevice(_) => Self::OutputDevice,
            ENode::AdapterWaveSum => Self::AdapterWaveSum,
            ENode::MixStereo { .. } => Self::MixStereo,
            ENode::FilterFIR(_) => Self::FilterFIR,
            ENode::FilterIIRLPF(_) => Self::FilterIIRLPF,
            ENode::FilterIIRHPF(_) => Self::FilterIIRHPF,
            ENode::FilterIIRBandPass(_) => Self::FilterIIRBandPass,
            ENode::FilterIIRBandStop(_) => Self::FilterIIRBandStop,
            ENode::FilterIRConvolution(_) => Self::FilterIRConvolution,
            ENode::AdapterLimiter(_) => Self::AdapterLimiter,
        }
    }

    /// 入力ピンの名前のリストを取得する。
    pub fn get_input_pin_names(&self) -> Vec<&'static str> {
        match self {
            Self::InternalStartPin => StartProcessData::get_input_pin_names(),
            Self::InternalDummy => DummyProcessData::get_input_pin_names(),
            Self::AdapterEnvelopeAd => AdapterEnvelopeAdProcessData::get_input_pin_names(),
            Self::AdapterEnvelopeAdsr => AdapterEnvelopeAdsrProcessData::get_input_pin_names(),
            Self::AdapterWaveSum => AdapterWaveSumProcessData::get_input_pin_names(),
            Self::AdapterCompressor => AdapterCompressorProcessData::get_input_pin_names(),
            Self::AdapterLimiter => AdapterLimiterProcessData::get_input_pin_names(),
            Self::EmitterPinkNoise
            | Self::EmitterSawtooth
            | Self::EmitterSquare
            | Self::EmitterTriangle
            | Self::EmitterWhiteNoise
            | Self::EmitterSineWave => SineWaveEmitterProcessData::get_input_pin_names(),
            Self::EmitterWavMono => EmitterWavMonoProcessData::get_input_pin_names(),
            Self::AnalyzerDFT => AnalyzerDFTProcessData::get_input_pin_names(),
            Self::AnalyzerFFT => AnalyzerFFTProcessData::get_input_pin_names(),
            Self::AnalyzerLUFS => AnalyzeLUFSProcessData::get_input_pin_names(),
            Self::OutputFile => OutputFileProcessData::get_input_pin_names(),
            Self::OutputLog => OutputLogProcessData::get_input_pin_names(),
            Self::OutputDevice => OutputDeviceProcessData::get_input_pin_names(),
            Self::EmitterIDFT => IDFTEmitterProcessData::get_input_pin_names(),
            Self::EmitterIFFT => IFFTEmitterProcessData::get_input_pin_names(),
            Self::MixStereo => MixStereoProcessData::get_input_pin_names(),
            Self::FilterFIR => FIRProcessData::get_input_pin_names(),
            Self::FilterIIRLPF | Self::FilterIIRHPF | Self::FilterIIRBandPass | Self::FilterIIRBandStop => {
                IIRProcessData::get_input_pin_names()
            }
            Self::FilterIRConvolution => IRConvolutionProcessData::get_input_pin_names(),
        }
    }

    /// 出力ピンの名前リストを取得する。
    pub fn get_output_pin_names(&self) -> Vec<&'static str> {
        match self {
            Self::InternalStartPin => StartProcessData::get_output_pin_names(),
            Self::InternalDummy => DummyProcessData::get_output_pin_names(),
            Self::AdapterEnvelopeAd => AdapterEnvelopeAdProcessData::get_output_pin_names(),
            Self::AdapterEnvelopeAdsr => AdapterEnvelopeAdsrProcessData::get_output_pin_names(),
            Self::AdapterWaveSum => AdapterWaveSumProcessData::get_output_pin_names(),
            Self::AdapterCompressor => AdapterCompressorProcessData::get_output_pin_names(),
            Self::AdapterLimiter => AdapterLimiterProcessData::get_output_pin_names(),
            Self::EmitterPinkNoise
            | Self::EmitterSawtooth
            | Self::EmitterSquare
            | Self::EmitterTriangle
            | Self::EmitterWhiteNoise
            | Self::EmitterSineWave => SineWaveEmitterProcessData::get_output_pin_names(),
            Self::EmitterWavMono => EmitterWavMonoProcessData::get_output_pin_names(),
            Self::AnalyzerDFT => AnalyzerDFTProcessData::get_output_pin_names(),
            Self::AnalyzerFFT => AnalyzerFFTProcessData::get_output_pin_names(),
            Self::AnalyzerLUFS => AnalyzeLUFSProcessData::get_output_pin_names(),
            Self::OutputFile => OutputFileProcessData::get_output_pin_names(),
            Self::OutputLog => OutputLogProcessData::get_output_pin_names(),
            Self::OutputDevice => OutputDeviceProcessData::get_output_pin_names(),
            Self::EmitterIDFT => IDFTEmitterProcessData::get_output_pin_names(),
            Self::EmitterIFFT => IFFTEmitterProcessData::get_output_pin_names(),
            Self::MixStereo => MixStereoProcessData::get_output_pin_names(),
            Self::FilterFIR => FIRProcessData::get_output_pin_names(),
            Self::FilterIIRLPF | Self::FilterIIRHPF | Self::FilterIIRBandPass | Self::FilterIIRBandStop => {
                IIRProcessData::get_output_pin_names()
            }
            Self::FilterIRConvolution => IRConvolutionProcessData::get_output_pin_names(),
        }
    }

    /// 処理ノード（[`ProcessControlItem`]）に必要な、ノードの入力側のピンのリストを生成して返す。
    pub fn create_input_pins(&self) -> NodePinItemList {
        let names = self.get_input_pin_names();
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
        let names = self.get_output_pin_names();
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
        let names = self.get_input_pin_names();
        if names.is_empty() {
            return false;
        }
        names.contains(&pin_name)
    }

    /// 自分のoutputピンに`pin_name`と一致する名前のピンがあるか？
    pub fn is_valid_output_pin(&self, pin_name: &str) -> bool {
        let names = self.get_output_pin_names();
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
            Self::AdapterWaveSum => AdapterWaveSumProcessData::get_pin_categories(pin_name),
            Self::AdapterCompressor => AdapterCompressorProcessData::get_pin_categories(pin_name),
            Self::AdapterLimiter => AdapterLimiterProcessData::get_pin_categories(pin_name),
            Self::EmitterPinkNoise
            | Self::EmitterSawtooth
            | Self::EmitterSquare
            | Self::EmitterTriangle
            | Self::EmitterWhiteNoise
            | Self::EmitterSineWave => SineWaveEmitterProcessData::get_pin_categories(pin_name),
            Self::EmitterWavMono => EmitterWavMonoProcessData::get_pin_categories(pin_name),
            Self::AnalyzerDFT => AnalyzerDFTProcessData::get_pin_categories(pin_name),
            Self::AnalyzerFFT => AnalyzerFFTProcessData::get_pin_categories(pin_name),
            Self::AnalyzerLUFS => AnalyzeLUFSProcessData::get_pin_categories(pin_name),
            Self::OutputFile => OutputFileProcessData::get_pin_categories(pin_name),
            Self::OutputLog => OutputLogProcessData::get_pin_categories(pin_name),
            Self::OutputDevice => OutputDeviceProcessData::get_pin_categories(pin_name),
            Self::EmitterIDFT => IDFTEmitterProcessData::get_pin_categories(pin_name),
            Self::EmitterIFFT => IFFTEmitterProcessData::get_pin_categories(pin_name),
            Self::MixStereo => MixStereoProcessData::get_pin_categories(pin_name),
            Self::FilterFIR => FIRProcessData::get_pin_categories(pin_name),
            Self::FilterIIRLPF | Self::FilterIIRHPF | Self::FilterIIRBandPass | Self::FilterIIRBandStop => {
                IIRProcessData::get_pin_categories(pin_name)
            },
            Self::FilterIRConvolution => IRConvolutionProcessData::get_pin_categories(pin_name),
        }
    }

    /// Inputピンである場合、そのInputピンのフラグを返す。
    pub fn get_input_flag(&self, pin_name: &str) -> EInputContainerCategoryFlag {
        match self {
            Self::InternalStartPin => StartProcessData::get_input_container_flag(pin_name),
            Self::InternalDummy => DummyProcessData::get_input_container_flag(pin_name),
            Self::AdapterEnvelopeAd => AdapterEnvelopeAdProcessData::get_input_container_flag(pin_name),
            Self::AdapterEnvelopeAdsr => AdapterEnvelopeAdsrProcessData::get_input_container_flag(pin_name),
            Self::AdapterWaveSum => AdapterWaveSumProcessData::get_input_container_flag(pin_name),
            Self::AdapterCompressor => AdapterCompressorProcessData::get_input_container_flag(pin_name),
            Self::AdapterLimiter => AdapterLimiterProcessData::get_input_container_flag(pin_name),
            Self::EmitterPinkNoise
            | Self::EmitterSawtooth
            | Self::EmitterSquare
            | Self::EmitterTriangle
            | Self::EmitterWhiteNoise
            | Self::EmitterSineWave => SineWaveEmitterProcessData::get_input_container_flag(pin_name),
            Self::EmitterWavMono => EmitterWavMonoProcessData::get_input_container_flag(pin_name),
            Self::AnalyzerDFT => AnalyzerDFTProcessData::get_input_container_flag(pin_name),
            Self::AnalyzerFFT => AnalyzerFFTProcessData::get_input_container_flag(pin_name),
            Self::AnalyzerLUFS => AnalyzeLUFSProcessData::get_input_container_flag(pin_name),
            Self::OutputFile => OutputFileProcessData::get_input_container_flag(pin_name),
            Self::OutputLog => OutputLogProcessData::get_input_container_flag(pin_name),
            Self::OutputDevice => OutputDeviceProcessData::get_input_container_flag(pin_name),
            Self::EmitterIDFT => IDFTEmitterProcessData::get_input_container_flag(pin_name),
            Self::EmitterIFFT => IFFTEmitterProcessData::get_input_container_flag(pin_name),
            Self::MixStereo => MixStereoProcessData::get_input_container_flag(pin_name),
            Self::FilterFIR => FIRProcessData::get_input_container_flag(pin_name),
            Self::FilterIIRLPF | Self::FilterIIRHPF | Self::FilterIIRBandPass | Self::FilterIIRBandStop => {
                IIRProcessData::get_input_container_flag(pin_name)
            },
            Self::FilterIRConvolution => IRConvolutionProcessData::get_input_container_flag(pin_name),
        }
        .unwrap()
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
