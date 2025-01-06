use crate::carg::v2::adapter::compressor::{AdapterCompressorProcessData, MetaCompressorInfo};
use crate::carg::v2::adapter::envelope_ad::AdapterEnvelopeAdProcessData;
use crate::carg::v2::adapter::envelope_adsr::AdapterEnvelopeAdsrProcessData;
use crate::carg::v2::adapter::limiter::{AdapterLimiterProcessData, MetaLimiterInfo};
use crate::carg::v2::adapter::resample::{MetaResampleInfo, ResampleProcessData};
use crate::carg::v2::adapter::wave_sum::AdapterWaveSumProcessData;
use crate::carg::v2::analyzer::dft::AnalyzerDFTProcessData;
use crate::carg::v2::analyzer::fft::AnalyzerFFTProcessData;
use crate::carg::v2::analyzer::lufs::{AnalyzeLUFSProcessData, MetaLufsInfo};
use crate::carg::v2::emitter::idft::IDFTEmitterProcessData;
use crate::carg::v2::emitter::ifft::IFFTEmitterProcessData;
use crate::carg::v2::emitter::oscilo::{MetaSineEmitterInfo, MetaSineNoiseInfo, MetaSineSquareInfo, SineWaveEmitterProcessData};
use crate::carg::v2::emitter::wav_mono::{EmitterWavMonoProcessData, MetaWavInfo};
use crate::carg::v2::filter::fir::{FIRProcessData, MetaFIRInfo};
use crate::carg::v2::filter::iir::{IIRProcessData, MetaIIRInfo};
use crate::carg::v2::filter::irconv::{IRConvolutionProcessData, MetaIRConvInfo};
use crate::carg::v2::meta::process::{process_category, EProcessCategoryFlag};
use crate::carg::v2::meta::relation::{Relation, RelationItemPin};
use crate::carg::v2::meta::system::{system_category, ESystemCategoryFlag, InitializeSystemAccessor};
use crate::carg::v2::meta::{ENodeSpecifier, EPinCategoryFlag, SPinCategory};
use crate::carg::v2::mix::stereo::MixStereoProcessData;
use crate::carg::v2::output::output_device::{MetaOutputDeviceInfo, OutputDeviceProcessData};
use crate::carg::v2::output::output_file::{MetaOutputFileInfo, OutputFileProcessData};
use crate::carg::v2::output::output_log::OutputLogProcessData;
use crate::carg::v2::special::dummy::DummyProcessData;
use crate::carg::v2::special::start::StartProcessData;
use crate::carg::v2::{
    EParsedOutputLogMode, ProcessItemCreateSetting, Setting,
    TProcessItem, TProcessItemPtr,
};
use crate::math::float::EFloatCommonPin;
use crate::math::window::EWindowFunction;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::carg::v2::adapter::delay::{AdapterDelayProcessData, MetaDelayInfo};
use crate::carg::v2::emitter::sine_sweep::{MetaSineSweepInfo, SineSweepEmitterProcessData};
// ----------------------------------------------------------------------------
// ENode
// ----------------------------------------------------------------------------

///
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum ENode {
    /// 内部制御用。
    #[serde(rename = "_start_pin")]
    InternalStartPin,
    #[serde(rename = "_dummy")]
    InternalDummy,
    /// ピンクノイズを出力する。
    #[serde(rename = "emitter-pinknoise")]
    EmitterPinkNoise(MetaSineNoiseInfo),
    /// ホワイトノイズを出力する。
    #[serde(rename = "emitter-whitenoise")]
    EmitterWhiteNoise(MetaSineNoiseInfo),
    /// サイン波形（正弦波）を出力する。
    #[serde(rename = "emitter-sine")]
    EmitterSineWave(MetaSineEmitterInfo),
    /// ノコギリ波を出力する。
    #[serde(rename = "emitter-saw")]
    EmitterSawtooth(MetaSineEmitterInfo),
    /// 三角波を出力する。
    #[serde(rename = "emitter-triangle")]
    EmitterTriangle(MetaSineEmitterInfo),
    /// 矩形波を出力する。
    #[serde(rename = "emitter-square")]
    EmitterSquare(MetaSineSquareInfo),
    /// 周波数情報から音波バッファを生成する。
    #[serde(rename = "emitter-idft")]
    EmitterIDFT {
        sample_length: usize,
        /// 半分ずつ重ねるか
        overlap: bool,
    },
    /// 周波数情報から音波バッファを生成する。
    #[serde(rename = "emitter-ifft")]
    EmitterIFFT {
        sample_length: usize,
        /// 半分ずつ重ねるか
        overlap: bool,
    },
    /// パスからサポートできるWavを読み込み、サンプルをバッファで出力する。
    #[serde(rename = "emitter-wav-mono")]
    EmitterWavMono(MetaWavInfo),
    #[serde(rename = "emitter-sinesweep")]
    EmitterSineSweep(MetaSineSweepInfo),
    /// DFTで音波を分析する。
    #[serde(rename = "analyze-dft")]
    AnalyzerDFT {
        level: usize,
        window_function: EWindowFunction,
        /// 半分ずつ重ねるか
        overlap: bool,
    },
    /// FFTで音波を分析する。
    #[serde(rename = "analyze-fft")]
    AnalyzerFFT {
        level: usize,
        window_function: EWindowFunction,
        /// 半分ずつ重ねるか
        overlap: bool,
    },
    /// 音を分析しLUFSを測定する。
    #[serde(rename = "analyze-lufs")]
    AnalyzerLUFS(MetaLufsInfo),
    /// 振幅をAD(Attack-Delay)Envelopeを使って調整する。
    #[serde(rename = "adapter-envelope-ad")]
    AdapterEnvelopeAd {
        attack_time: f64,
        decay_time: f64,
        attack_curve: f64,
        decay_curve: f64,
    },
    /// 振幅をADSR(Attack-Delay-Sustain-Release)Envelopeを使って調整する。
    #[serde(rename = "adapter-envelope-adsr")]
    AdapterEnvelopeAdsr {
        attack_time: f64,
        decay_time: f64,
        sustain_time: f64,
        release_time: f64,
        attack_curve: f64,
        decay_curve: f64,
        release_curve: f64,
        /// sustainで維持する振幅`[0, 1]`の値。
        sustain_value: f64,
    },
    /// バッファを全部合わせる。
    #[serde(rename = "adapter-wave-sum")]
    AdapterWaveSum,
    #[serde(rename = "adapter-compressor")]
    AdapterCompressor(MetaCompressorInfo),
    #[serde(rename = "adapter-limiter")]
    AdapterLimiter(MetaLimiterInfo),
    #[serde(rename = "adapter-resample")]
    AdapterResample(MetaResampleInfo),
    #[serde(rename = "adapter-delay")]
    AdapterDelay(MetaDelayInfo),
    /// 昔に作っておいたFIRのLPFフィルター（2次FIR）
    #[serde(rename = "filter-fir")]
    FilterFIR(MetaFIRInfo),
    /// 昔に作っておいたIIRのLPFフィルター（2次IIR）
    #[serde(rename = "filter-iir-lpf")]
    FilterIIRLPF(MetaIIRInfo),
    /// 昔に作っておいたIIRのHPFフィルター（2次IIR）
    #[serde(rename = "filter-iir-hpf")]
    FilterIIRHPF(MetaIIRInfo),
    /// 昔に作っておいたIIRのバンドパスフィルター（2次IIR）
    #[serde(rename = "filter-iir-bpf")]
    FilterIIRBandPass(MetaIIRInfo),
    /// 昔に作っておいたIIRのバンドストップフィルター（2次IIR）
    #[serde(rename = "filter-iir-bsf")]
    FilterIIRBandStop(MetaIIRInfo),
    #[serde(rename = "filter-irconv")]
    FilterIRConvolution(MetaIRConvInfo),
    #[serde(rename = "mix-stereo")]
    MixStereo {
        gain_0: EFloatCommonPin,
        gain_1: EFloatCommonPin,
    },
    /// 何かからファイルを出力する
    #[serde(rename = "output-file")]
    OutputFile(MetaOutputFileInfo),
    #[serde(rename = "output-log")]
    OutputLog { mode: EParsedOutputLogMode },
    #[serde(rename = "output-device")]
    OutputDevice(MetaOutputDeviceInfo),
}

impl ENode {
    /// ノードから処理アイテムを生成する。
    pub fn create_from(&self, setting: &Setting, system_setting: &InitializeSystemAccessor) -> TProcessItemPtr {
        let setting = ProcessItemCreateSetting { node: &self, setting };

        match self {
            ENode::EmitterPinkNoise { .. }
            | ENode::EmitterWhiteNoise { .. }
            | ENode::EmitterSineWave { .. }
            | ENode::EmitterTriangle { .. }
            | ENode::EmitterSquare { .. }
            | ENode::EmitterSawtooth { .. } => {
                SineWaveEmitterProcessData::create_item(&setting, &system_setting).expect("Failed to create item")
            }
            ENode::AdapterEnvelopeAd { .. } => {
                AdapterEnvelopeAdProcessData::create_item(&setting, &system_setting).expect("Failed to create item")
            },
            ENode::AdapterEnvelopeAdsr { .. } => {
                AdapterEnvelopeAdsrProcessData::create_item(&setting, &system_setting).expect("Failed to create item")
            },
            ENode::AdapterCompressor(_) => {
                AdapterCompressorProcessData::create_item(&setting, &system_setting).expect("Failed to create item")
            },
            ENode::AdapterLimiter(_) => {
                AdapterLimiterProcessData::create_item(&setting, &system_setting).expect("Failed to create item")
            },
            ENode::AdapterWaveSum => {
                AdapterWaveSumProcessData::create_item(&setting, &system_setting).expect("Failed to create item")
            },
            ENode::AdapterResample(_) => {
                ResampleProcessData::create_item(&setting, &system_setting).expect("Failed to create item")
            }
            ENode::AdapterDelay(_) => {
                AdapterDelayProcessData::create_item(&setting, &system_setting).expect("Failed to create item")
            }
            ENode::AnalyzerDFT { .. } => {
                AnalyzerDFTProcessData::create_item(&setting, &system_setting).expect("Failed to create item")
            },
            ENode::AnalyzerFFT { .. } => {
                AnalyzerFFTProcessData::create_item(&setting, &system_setting).expect("Failed to create item")
            },
            ENode::InternalStartPin => {
                StartProcessData::create_item(&setting, &system_setting).expect("Failed to create item")
            },
            ENode::EmitterIDFT { .. } => {
                IDFTEmitterProcessData::create_item(&setting, &system_setting).expect("Failed to create item")
            },
            ENode::EmitterIFFT { .. } => {
                IFFTEmitterProcessData::create_item(&setting, &system_setting).expect("Failed to create item")
            }
            ENode::EmitterWavMono(_) => {
                EmitterWavMonoProcessData::create_item(&setting, &system_setting).expect("Failed to create item")
            }
            ENode::InternalDummy => {
                DummyProcessData::create_item(&setting, &system_setting).expect("Failed to create item")
            }
            ENode::MixStereo { .. } => {
                MixStereoProcessData::create_item(&setting, &system_setting).expect("Failed to create item")
            },
            ENode::FilterFIR(_) => {
                FIRProcessData::create_item(&setting, &system_setting).expect("Failed to create item")
            }
            ENode::FilterIIRHPF(_) |
            ENode::FilterIIRBandPass(_) |
            ENode::FilterIIRBandStop(_) |
            ENode::FilterIIRLPF(_) => {
                IIRProcessData::create_item(&setting, &system_setting).expect("Failed to create item")
            }
            ENode::FilterIRConvolution(_) => {
                IRConvolutionProcessData::create_item(&setting, &system_setting).expect("Failed to create item")
            }
            ENode::OutputLog { .. } => {
                OutputLogProcessData::create_item(&setting, &system_setting).expect("Failed to create item")
            }
            ENode::OutputFile(_) => {
                OutputFileProcessData::create_item(&setting, &system_setting).expect("Failed to create item")
            }
            ENode::AnalyzerLUFS(_) => {
                AnalyzeLUFSProcessData::create_item(&setting, &system_setting).expect("Failed to create item")
            }
            ENode::OutputDevice(_) => {
                OutputDeviceProcessData::create_item(&setting, &system_setting).expect("Failed to create item")
            }
            ENode::EmitterSineSweep(_) => {
                SineSweepEmitterProcessData::create_item(&setting, &system_setting).expect("Failed to create item")
            }
        }
    }
}

// ----------------------------------------------------------------------------
// MetaNodeContainer
// ----------------------------------------------------------------------------

/// パーサーから取得したメター情報を持つノードのコンテナ。
pub struct MetaNodeContainer {
    pub map: HashMap<String, ENode>,
}

impl MetaNodeContainer {
    /// ノード間の関係図で前ノードとして処理が行えるか。
    pub fn is_valid_prev_node_pin(&self, item: &RelationItemPin) -> bool {
        // メタノードに接近して、pinが存在しているか？
        match self.map.get(&item.node) {
            // メタノードマップにあるか？
            None => false,
            Some(v) => ENodeSpecifier::from_node(v).is_valid_output_pin(&item.pin),
        }
    }

    /// ノード間の関係図で次ノードとして処理が行えるか。
    pub fn is_valid_next_node_pin(&self, item: &RelationItemPin) -> bool {
        // メタノードに接近して、pinが存在しているか？
        match self.map.get(&item.node) {
            // メタノードマップにあるか？
            None => false,
            Some(v) => ENodeSpecifier::from_node(v).is_valid_input_pin(&item.pin),
        }
    }

    /// 関係ノードに書いているピンのカテゴリ（複数可）を返す。
    pub fn get_pin_categories(&self, item: &RelationItemPin) -> Option<EPinCategoryFlag> {
        match self.map.get(&item.node) {
            // メタノードマップにあるか？
            None => None,
            Some(v) => ENodeSpecifier::from_node(v).get_pin_categories(&item.pin),
        }
    }

    /// `relation`が有効か？
    pub fn is_valid_relation(&self, relation: &Relation) -> bool {
        if !self.is_valid_prev_node_pin(&relation.prev) {
            println!("self.is_valid_prev_node_pin(&relation.prev) failed.");
            return false;
        }
        if !self.is_valid_next_node_pin(&relation.next) {
            println!("self.is_valid_next_node_pin(&relation.next) failed.");
            return false;
        }

        // 24-12-13 prev/next間の処理順が逆になっていないかを確認する。
        let prev_orders = ENodeSpecifier::from_node(self.map.get(&relation.prev.node).unwrap()).get_process_category();
        let next_orders = ENodeSpecifier::from_node(self.map.get(&relation.next.node).unwrap()).get_process_category();
        if next_orders < prev_orders {
            println!(
                "Given relation prev : ({:?}) and next : ({:?}) are not in the valid order.",
                relation.prev, relation.next
            );
            return false;
        }

        // お互いにチェック。
        // pinの種類を見て判定する。
        let output_pin = self
            .get_pin_categories(&relation.prev)
            .expect(&format!("({:?}) must be have pin categories", relation.prev));
        let input_pin = self
            .get_pin_categories(&relation.next)
            .expect(&format!("({:?}) must be have pin categories", relation.next));
        SPinCategory::can_support(output_pin, input_pin)
    }

    /// このマップで必要となるシステムのカテゴリ全体フラグを返す。
    pub fn get_dependent_system_categories(&self) -> ESystemCategoryFlag {
        let mut categories = system_category::NONE;
        for (_, v) in &self.map {
            categories |= ENodeSpecifier::from_node(v).get_dependent_system_categories();
        }

        categories
    }

    /// このマップで必要となる処理順カテゴリ全体を返す。
    pub fn get_using_process_categories(&self) -> EProcessCategoryFlag {
        let mut categories = process_category::NORMAL;
        for (_, v) in &self.map {
            categories |= ENodeSpecifier::from_node(v).get_process_category();
        }

        categories
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
