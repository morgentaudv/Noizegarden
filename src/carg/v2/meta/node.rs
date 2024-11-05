use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::carg::v1::EOutputFileFormat;
use crate::carg::v2::{EParsedOutputLogMode, EmitterRange, Setting, TProcessItemPtr};
use crate::carg::v2::adapter::envelope_ad::AdapterEnvelopeAdProcessData;
use crate::carg::v2::adapter::envelope_adsr::AdapterEnvelopeAdsrProcessData;
use crate::carg::v2::adapter::wave_sum::AdapterWaveSumProcessData;
use crate::carg::v2::analyzer::dft::AnalyzerDFTProcessData;
use crate::carg::v2::analyzer::fft::AnalyzerFFTProcessData;
use crate::carg::v2::emitter::idft::IDFTEmitterProcessData;
use crate::carg::v2::emitter::ifft::IFFTEmitterProcessData;
use crate::carg::v2::emitter::oscilo::SineWaveEmitterProcessData;
use crate::carg::v2::filter::fir_lpf::{FIRLPFProcessData, MetaFIRLPFInfo};
use crate::carg::v2::meta::{ENodeSpecifier, EPinCategoryFlag, SPinCategory};
use crate::carg::v2::meta::relation::{Relation, RelationItemPin};
use crate::carg::v2::mix::stereo::MixStereoProcessData;
use crate::carg::v2::output::output_file::OutputFileProcessData;
use crate::carg::v2::output::output_log::OutputLogProcessData;
use crate::carg::v2::special::dummy::DummyProcessData;
use crate::carg::v2::special::start::StartProcessData;
use crate::math::float::EFloatCommonPin;
use crate::math::frequency::EFrequency;
use crate::math::window::EWindowFunction;
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
    EmitterPinkNoise { intensity: f64, range: EmitterRange },
    /// ホワイトノイズを出力する。
    #[serde(rename = "emitter-whitenoise")]
    EmitterWhiteNoise { intensity: f64, range: EmitterRange },
    /// サイン波形（正弦波）を出力する。
    #[serde(rename = "emitter-sine")]
    EmitterSineWave {
        frequency: EFrequency,
        intensity: f64,
        range: EmitterRange,
    },
    /// ノコギリ波を出力する。
    #[serde(rename = "emitter-saw")]
    EmitterSawtooth {
        frequency: EFrequency,
        intensity: f64,
        range: EmitterRange,
    },
    /// 三角波を出力する。
    #[serde(rename = "emitter-triangle")]
    EmitterTriangle {
        frequency: EFrequency,
        intensity: f64,
        range: EmitterRange,
    },
    /// 矩形波を出力する。
    #[serde(rename = "emitter-square")]
    EmitterSquare {
        frequency: EFrequency,
        duty_rate: f64,
        intensity: f64,
        range: EmitterRange,
    },
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
    /// 昔に作っておいたFIRのLPFフィルター（2次FIR）
    #[serde(rename = "filter-fir-lpf")]
    FilterFIRLPF(MetaFIRLPFInfo),
    #[serde(rename = "mix-stereo")]
    MixStereo {
        gain_0: EFloatCommonPin,
        gain_1: EFloatCommonPin,
    },
    /// 何かからファイルを出力する
    #[serde(rename = "output-file")]
    OutputFile {
        format: EOutputFileFormat,
        file_name: String,
    },
    #[serde(rename = "output-log")]
    OutputLog { mode: EParsedOutputLogMode },
}

impl ENode {
    /// ノードから処理アイテムを生成する。
    pub fn create_from(&self, setting: &Setting) -> TProcessItemPtr {
        match self {
            ENode::EmitterPinkNoise { .. }
            | ENode::EmitterWhiteNoise { .. }
            | ENode::EmitterSineWave { .. }
            | ENode::EmitterTriangle { .. }
            | ENode::EmitterSquare { .. }
            | ENode::EmitterSawtooth { .. } => SineWaveEmitterProcessData::create_from(self, setting),
            ENode::AdapterEnvelopeAd { .. } => AdapterEnvelopeAdProcessData::create_from(self, setting),
            ENode::AdapterEnvelopeAdsr { .. } => AdapterEnvelopeAdsrProcessData::create_from(self, setting),
            ENode::OutputLog { .. } => OutputLogProcessData::create_from(self, setting),
            ENode::OutputFile { .. } => OutputFileProcessData::create_from(self, setting),
            ENode::AnalyzerDFT { .. } => AnalyzerDFTProcessData::create_from(self, setting),
            ENode::AnalyzerFFT { .. } => AnalyzerFFTProcessData::create_from(self, setting),
            ENode::InternalStartPin => StartProcessData::create_from(self, setting),
            ENode::EmitterIDFT { .. } => IDFTEmitterProcessData::create_from(self, setting),
            ENode::EmitterIFFT { .. } => IFFTEmitterProcessData::create_from(self, setting),
            ENode::InternalDummy => DummyProcessData::create_from(self, setting),
            ENode::AdapterWaveSum => AdapterWaveSumProcessData::create_from(self, setting),
            ENode::MixStereo { .. } => MixStereoProcessData::create_from(self, setting),
            ENode::FilterFIRLPF(_) => FIRLPFProcessData::create_from(self, setting),
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
}