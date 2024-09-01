use std::{
    cell::RefCell,
    collections::{HashMap, HashSet, VecDeque},
    rc::Rc,
};

use adapter::{envelope_ad::AdapterEnvelopeAdProcessData, envelope_adsr::AdapterEnvelopeAdsrProcessData};
use emitter::SineWaveEmitterProcessData;
use itertools::Itertools;
use output::{output_file::OutputFileProcessData, output_log::OutputLogProcessData};
use serde::{Deserialize, Serialize};

use crate::{math::frequency::EFrequency, wave::sample::UniformedSample};

use super::{container::ENodeContainer, v1::EOutputFileFormat};

pub mod adapter;
pub mod emitter;
pub mod output;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Setting {
    /// 音生成のために使うサンプルレートを指す。0より上であること。
    sample_rate: u64,
}

/// 内部識別処理に使うEnum。
enum ENodeSpecifier {
    EmitterPinkNoise,
    EmitterWhiteNoise,
    EmitterSineWave,
    EmitterSawtooth,
    EmitterTriangle,
    EmitterSquare,
    AdapterEnvlopeAd,
    AdapterEnvlopeAdsr,
    OutputFile,
    OutputLog,
}

impl ENodeSpecifier {
    /// 変換する
    pub fn from_node(node: &ENode) -> Self {
        match node {
            ENode::EmitterPinkNoise { .. } => Self::EmitterPinkNoise,
            ENode::EmitterWhiteNoise { .. } => Self::EmitterWhiteNoise,
            ENode::EmitterSineWave { .. } => Self::EmitterSineWave,
            ENode::EmitterSawtooth { .. } => Self::EmitterSawtooth,
            ENode::EmitterTriangle { .. } => Self::EmitterTriangle,
            ENode::EmitterSquare { .. } => Self::EmitterSquare,
            ENode::AdapterEnvlopeAd { .. } => Self::AdapterEnvlopeAd,
            ENode::AdapterEnvlopeAdsr { .. } => Self::AdapterEnvlopeAdsr,
            ENode::OutputFile { .. } => Self::OutputFile,
            ENode::OutputLog { .. } => Self::OutputLog,
        }
    }

    pub fn is_input(&self) -> bool {
        match self {
            Self::EmitterPinkNoise => true,
            Self::EmitterWhiteNoise => true,
            Self::EmitterSineWave => true,
            Self::EmitterSawtooth => true,
            Self::EmitterTriangle => true,
            Self::EmitterSquare | Self::AdapterEnvlopeAd | Self::AdapterEnvlopeAdsr => true,
            Self::OutputFile => false,
            Self::OutputLog => false,
        }
    }

    pub fn is_output(&self) -> bool {
        match self {
            Self::EmitterPinkNoise => false,
            Self::EmitterWhiteNoise => false,
            Self::EmitterSineWave => false,
            Self::EmitterSawtooth => false,
            Self::EmitterTriangle => false,
            Self::EmitterSquare => false,
            Self::OutputFile | Self::OutputLog | Self::AdapterEnvlopeAd | Self::AdapterEnvlopeAdsr => true,
        }
    }

    /// 自分が`output`と互換性のあるノードなのか？
    pub fn is_supported_by(&self, output: &Self) -> bool {
        match *output {
            // falseしかできない。
            Self::EmitterPinkNoise
            | Self::EmitterWhiteNoise
            | Self::EmitterSineWave
            | Self::EmitterSawtooth
            | Self::EmitterTriangle
            | Self::EmitterSquare => false,
            // trueになれる。
            Self::OutputFile => match self {
                Self::EmitterPinkNoise
                | Self::EmitterWhiteNoise
                | Self::EmitterSineWave
                | Self::EmitterSawtooth
                | Self::EmitterTriangle
                | Self::AdapterEnvlopeAd
                | Self::AdapterEnvlopeAdsr
                | Self::EmitterSquare => true,
                Self::OutputFile | Self::OutputLog => false,
            },
            Self::OutputLog => match self {
                Self::EmitterPinkNoise
                | Self::EmitterWhiteNoise
                | Self::EmitterSineWave
                | Self::EmitterSawtooth
                | Self::EmitterTriangle
                | Self::AdapterEnvlopeAd
                | Self::AdapterEnvlopeAdsr
                | Self::EmitterSquare => true,
                Self::OutputFile | Self::OutputLog => false,
            },
            Self::AdapterEnvlopeAd => match self {
                Self::EmitterPinkNoise
                | Self::EmitterWhiteNoise
                | Self::EmitterSineWave
                | Self::EmitterSawtooth
                | Self::EmitterTriangle
                | Self::AdapterEnvlopeAd
                | Self::AdapterEnvlopeAdsr
                | Self::EmitterSquare => true,
                Self::OutputFile | Self::OutputLog => false,
            },
            Self::AdapterEnvlopeAdsr => match self {
                Self::EmitterPinkNoise
                | Self::EmitterWhiteNoise
                | Self::EmitterSineWave
                | Self::EmitterSawtooth
                | Self::EmitterTriangle
                | Self::AdapterEnvlopeAd
                | Self::AdapterEnvlopeAdsr
                | Self::EmitterSquare => true,
                Self::OutputFile | Self::OutputLog => false,
            },
        }
    }
}

///
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum ENode {
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
    /// 振幅をAD(Attack-Delay)Envelopeを使って調整する。
    #[serde(rename = "adapter-envelope-ad")]
    AdapterEnvlopeAd {
        attack_time: f64,
        decay_time: f64,
        attack_curve: f64,
        decay_curve: f64,
    },
    /// 振幅をADSR(Attack-Delay-Sustain-Release)Envelopeを使って調整する。
    #[serde(rename = "adapter-envelope-adsr")]
    AdapterEnvlopeAdsr {
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
    ///
    pub fn is_input(&self) -> bool {
        ENodeSpecifier::from_node(self).is_input()
    }

    pub fn is_output(&self) -> bool {
        ENodeSpecifier::from_node(self).is_output()
    }

    pub fn is_supported_by(&self, output: &ENode) -> bool {
        ENodeSpecifier::from_node(self).is_supported_by(&ENodeSpecifier::from_node(output))
    }
}

/// 発動条件を示す。
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum ETriggerCondition {
    #[serde(rename = "time")]
    Time { start: f64 },
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum EParsedOutputLogMode {
    #[serde(rename = "print")]
    Print,
}

///
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct EmitterRange {
    start: f64,
    length: f64,
}

/// [`ENode`]間の関係性を記述する。
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Relation {
    /// 入力側
    pub input: Vec<String>,
    /// 出力側
    pub output: String,
}

/// v2バージョンにパーシングする。
pub fn parse_v2(info: &serde_json::Value) -> anyhow::Result<ENodeContainer> {
    // Input, Setting, Outputがちゃんとあるとみなして吐き出す。
    let setting: Setting = serde_json::from_value(info["setting"].clone())?;
    let nodes: HashMap<String, ENode> = serde_json::from_value(info["node"].clone())?;
    let relations: Vec<Relation> = serde_json::from_value(info["relation"].clone())?;

    // まとめて出力。
    let container = ENodeContainer::V2 {
        setting,
        nodes,
        relations,
    };
    return Ok(container);
}

/// 次のことを検査する。
///
/// * inputとoutputが空白なものがあるかを確認する。
/// * それぞれのノードに対してCycleになっていないかを確認する。
pub fn validate_node_relations(nodes: &HashMap<String, ENode>, relations: &[Relation]) -> anyhow::Result<()> {
    for relation in relations {
        // inputとoutputが空白なものがあるかを確認する。
        if relation.input.is_empty() {
            return Err(anyhow::anyhow!("input node list is empty."));
        }
        if relation.output.is_empty() {
            return Err(anyhow::anyhow!("output node list is empty."));
        }

        // まずrelationsからnodesに当てはまらないノード文字列があるかを確認する。
        // input/output指定の文字列のノードが本当にinput/outputとして動作できるかを確認。
        for node_name in &relation.input {
            if !nodes.contains_key(node_name) {
                return Err(anyhow::anyhow!("Given node {} is not exist in node map.", node_name));
            }
            if !nodes[node_name].is_input() {
                return Err(anyhow::anyhow!("Given node {} is not for input.", node_name));
            }
        }
        {
            let output_node_name = &relation.output;
            if !nodes.contains_key(output_node_name) {
                return Err(anyhow::anyhow!("Given node {} is not exist in node map.", output_node_name));
            }
            if !nodes[output_node_name].is_output() {
                return Err(anyhow::anyhow!("Given node {} is not for output.", output_node_name));
            }
        }

        // そしてinputとoutputの互換性を確認する。
        // 具体的にはoutputに対してinputの組み方とタイプを検証する。
        {
            let output_node_name = &relation.output;
            let output = &nodes[output_node_name];

            for input_node_name in &relation.input {
                if !nodes[input_node_name].is_supported_by(output) {
                    return Err(anyhow::anyhow!(
                        "Input node {} does not support output node {}.",
                        input_node_name,
                        output_node_name
                    ));
                }
            }
        }
    }

    // それぞれのノードに対してCycleになっていないかを確認する。
    // 一番簡単な方法？ではinputとして使っているノードだけを検査し、
    // ノードからの経路をチェックして2回目以上通ることがあればCycle判定にする。
    for (node_name, _) in nodes {
        let mut name_queue: VecDeque<&String> = VecDeque::new();
        name_queue.push_back(node_name);

        let mut route_set: HashSet<GraphNodeRoute> = HashSet::new();

        while !name_queue.is_empty() {
            let search_name = name_queue.pop_front().unwrap();

            for relation in relations {
                let is_input = relation.input.contains(search_name);
                if !is_input {
                    continue;
                }

                // input-outputの経路を作って、今持っている経路リストにすでにあるかを確認する。
                // もしあれば、Cycleになっていると判断する。
                let route_item = GraphNodeRoute {
                    from: (*search_name).clone(),
                    to: relation.output.clone(),
                };
                if route_set.contains(&route_item) {
                    return Err(anyhow::anyhow!("Node {} is cycled.", node_name));
                }

                // 入れる。
                route_set.insert(route_item);
            }
        }
    }

    Ok(())
}

/// [`validate_node_relations`]の関数だけでしか使わないもの。経路を表す。
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct GraphNodeRoute {
    from: String,
    to: String,
}

/// Outputなノードの名前だけをリスト化して返す。
pub fn get_end_node_names(nodes: &HashMap<String, ENode>, relations: &[Relation]) -> Vec<String> {
    let mut end_nodes = vec![];
    for (node_name, _) in nodes {
        let mut is_node_only_output = true; // inputにはなくて、outputにはあったか。

        for relation in relations {
            if relation.input.contains(node_name) {
                // outputではなくなった。
                is_node_only_output = false;
                break;
            }
        }

        // 実はここでnode_nameが本当にoutputにあるかを確認すべきだけど、
        if is_node_only_output {
            end_nodes.push(node_name.clone());
        }
    }

    end_nodes
}

pub type RelationTreeNodePtr = Rc<RefCell<RelationTreeNode>>;

/// 木のノードアイテム。
#[derive(Debug)]
pub struct RelationTreeNode {
    /// ノードの名前
    pub name: String,
    /// 子ノード
    children: Vec<RelationTreeNodePtr>,
    /// 子ノードのインプットを取得するためのタイムスタンプのリスト。[`Self::children`]と同じ数であること。
    sync_timestamps: Vec<i64>,
    /// このノードから処理するアイテム
    processor: ENodeProcessData,
}

impl RelationTreeNode {
    /// 木のアイテムを作る。
    pub fn new_item(name: &str, processor: &ENodeProcessData) -> RelationTreeNodePtr {
        Rc::new(RefCell::new(RelationTreeNode {
            name: name.to_owned(),
            children: vec![],
            sync_timestamps: vec![],
            processor: processor.clone(),
        }))
    }

    /// 子ノードのリストを追加する。
    pub fn append_children(&mut self, mut children: Vec<RelationTreeNodePtr>) {
        if children.is_empty() {
            return;
        }

        self.children.append(&mut children);
        let new_child_count = self.children.len();
        self.sync_timestamps.resize(new_child_count, 0i64);

        // child_countを更新する。
        match &mut self.processor {
            ENodeProcessData::InputNoneOutputBuffer(v) => v.borrow_mut().set_child_count(new_child_count),
            ENodeProcessData::InputBufferOutputNone(v) => v.borrow_mut().set_child_count(new_child_count),
            ENodeProcessData::InputBufferOutputBuffer(v) => v.borrow_mut().set_child_count(new_child_count),
        }
    }

    pub fn try_process(&mut self) -> EProcessResult {
        assert_eq!(self.children.len(), self.sync_timestamps.len());

        // まずinputの形式が何かを取得する。
        // というかそもそもTreeの中ではinput/outputがValidationが決まっているので、正直いらないかも。
        //
        // 子ノードの終わる時間または更新時間はそれぞれ異なるはずなので、
        // それぞれのタイムスタンプが更新されていて、自分のタイムスタンプが遅れていれば子ノードからinputを取得する。
        for (child_i, (child, sync_timestamp)) in self.children.iter().zip(&mut self.sync_timestamps).enumerate() {
            let borrowed_child = child.borrow();
            if !borrowed_child.processor.is_finished() {
                return EProcessResult::Pending;
            }

            let child_time_stamp = borrowed_child.get_timestamp();
            if *sync_timestamp >= child_time_stamp {
                continue;
            }

            // 更新する必要があれば、取得する。
            self.processor.update_input(child_i, borrowed_child.get_output());
            *sync_timestamp = child_time_stamp;
        }

        self.processor.try_process()
    }

    /// 処理が終わっていない子ノードだけを名前としてリストに返す。
    pub fn try_get_unfinished_children_names(&self) -> Vec<String> {
        if self.children.is_empty() {
            return vec![];
        }

        self.children
            .iter()
            .filter(|v| !v.borrow().processor.is_finished())
            .map(|v| v.borrow().name.clone())
            .collect_vec()
    }

    /// 自分のタイムスタンプを返す。
    pub fn get_timestamp(&self) -> i64 {
        self.processor.get_timestamp()
    }

    /// 処理した後の出力を返す。
    pub fn get_output(&self) -> EProcessOutput {
        self.processor.get_output()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]

pub enum EProcessResult {
    Finished,
    Pending,
}

// ----------------------------------------------------------------------------
// ProcessControlItem
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProcessControlItem {
    /// アイテムの状態を表す。
    pub state: EProcessState,
    /// 最後の処理時間を示す。-1ならまだ処理していないことを表す。
    pub process_timestamp: i64,
    /// アイテムを持つノードからつながっている子アイテムの数
    pub child_count: usize,
}

impl ProcessControlItem {
    pub fn new() -> Self {
        Self {
            state: EProcessState::Stopped,
            process_timestamp: 0i64,
            child_count: 0usize,
        }
    }
}

/// 処理状態
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EProcessState {
    Stopped,
    Playing,
    Finished,
}

// ----------------------------------------------------------------------------
// ProcessBuffer
// ----------------------------------------------------------------------------

/// 各ノードが自分の情報と周りの情報から処理を行うときの処理アイテム
#[derive(Debug, Clone)]
pub enum ENodeProcessData {
    InputNoneOutputBuffer(InputNoneOutputBufferPtr),
    InputBufferOutputNone(InputBufferOutputNonePtr),
    InputBufferOutputBuffer(InputBufferOutputBufferPtr),
}

impl ENodeProcessData {
    /// ノードから処理アイテムを生成する。
    pub fn create_from(node: &ENode, setting: &Setting) -> Self {
        match node {
            ENode::EmitterPinkNoise { .. }
            | ENode::EmitterWhiteNoise { .. }
            | ENode::EmitterSineWave { .. }
            | ENode::EmitterTriangle { .. }
            | ENode::EmitterSquare { .. }
            | ENode::EmitterSawtooth { .. } => {
                ENodeProcessData::InputNoneOutputBuffer(SInputNoneOutputBuffer::create_from(node, setting))
            }
            ENode::AdapterEnvlopeAd { .. } | ENode::AdapterEnvlopeAdsr { .. } => {
                ENodeProcessData::InputBufferOutputBuffer(SInputBufferOutputBuffer::create_from(node, setting))
            }
            ENode::OutputLog { .. } | ENode::OutputFile { .. } => {
                ENodeProcessData::InputBufferOutputNone(SInputBufferOutputNone::create_from(node, setting))
            }
        }
    }

    /// データアイテムの処理が終わったか？
    pub fn is_finished(&self) -> bool {
        match self {
            ENodeProcessData::InputNoneOutputBuffer(v) => v.borrow().is_finished(),
            ENodeProcessData::InputBufferOutputNone(v) => v.borrow().is_finished(),
            ENodeProcessData::InputBufferOutputBuffer(v) => v.borrow().is_finished(),
        }
    }

    /// 自分のタイムスタンプを返す。
    pub fn get_timestamp(&self) -> i64 {
        match self {
            ENodeProcessData::InputNoneOutputBuffer(v) => v.borrow().get_timestamp(),
            ENodeProcessData::InputBufferOutputNone(v) => v.borrow().get_timestamp(),
            ENodeProcessData::InputBufferOutputBuffer(v) => v.borrow().get_timestamp(),
        }
    }

    /// 処理してみる。
    pub fn try_process(&mut self) -> EProcessResult {
        match self {
            ENodeProcessData::InputNoneOutputBuffer(v) => v.borrow_mut().try_process(),
            ENodeProcessData::InputBufferOutputNone(v) => v.borrow_mut().try_process(),
            ENodeProcessData::InputBufferOutputBuffer(v) => v.borrow_mut().try_process(),
        }
    }

    /// 中に`output`を更新する。
    pub fn update_input(&mut self, index: usize, output: EProcessOutput) {
        match self {
            ENodeProcessData::InputNoneOutputBuffer(_) => unreachable!("Unexpected branch"),
            ENodeProcessData::InputBufferOutputNone(v) => v.borrow_mut().update_input(index, output),
            ENodeProcessData::InputBufferOutputBuffer(v) => v.borrow_mut().update_input(index, output),
        }
    }

    /// 処理した後の出力を返す。
    pub fn get_output(&self) -> EProcessOutput {
        match self {
            ENodeProcessData::InputNoneOutputBuffer(v) => EProcessOutput::Buffer(v.borrow().get_output()),
            ENodeProcessData::InputBufferOutputNone(_) => EProcessOutput::None,
            ENodeProcessData::InputBufferOutputBuffer(v) => EProcessOutput::Buffer(v.borrow().get_output()),
        }
    }
}

// ----------------------------------------------------------------------------
// EProcessOutput
// ----------------------------------------------------------------------------

#[derive(Debug)]
pub enum EProcessOutput {
    None,
    Buffer(ProcessOutputBuffer),
}

#[derive(Debug, Clone)]
pub struct ProcessOutputBuffer {
    buffer: Vec<UniformedSample>,
    range: EmitterRange,
    setting: Setting,
}

// ----------------------------------------------------------------------------
// TInputNoneOutputBuffer
// ----------------------------------------------------------------------------

/// [`TInputNoneOutputBuffer`]を実装しているアイテムの外部表示タイプ
pub type InputNoneOutputBufferPtr = Rc<RefCell<dyn TInputNoneOutputBuffer>>;

/// 処理からOutputでバッファを返すためのTrait。
pub trait TInputNoneOutputBuffer: std::fmt::Debug {
    /// データアイテムの処理が終わったか？
    fn is_finished(&self) -> bool;

    /// 自分のタイムスタンプを返す。
    fn get_timestamp(&self) -> i64;

    /// 処理結果を返す。
    fn get_output(&self) -> ProcessOutputBuffer;

    fn try_process(&mut self) -> EProcessResult;

    fn set_child_count(&mut self, count: usize);
}

/// [`TInputNoneOutputBuffer`]のインスタンス生成ファクトリー。
struct SInputNoneOutputBuffer;
impl SInputNoneOutputBuffer {
    fn create_from(node: &ENode, setting: &Setting) -> InputNoneOutputBufferPtr {
        match node {
            ENode::EmitterPinkNoise { intensity, range } => {
                let item = SineWaveEmitterProcessData::new_pink(*intensity, *range, setting.clone());
                Rc::new(RefCell::new(item))
            }
            ENode::EmitterWhiteNoise { intensity, range } => {
                let item = SineWaveEmitterProcessData::new_white(*intensity, *range, setting.clone());
                Rc::new(RefCell::new(item))
            }
            ENode::EmitterSineWave {
                frequency,
                intensity,
                range,
            } => {
                let item = SineWaveEmitterProcessData::new_sine(*frequency, *intensity, *range, setting.clone());
                Rc::new(RefCell::new(item))
            }
            ENode::EmitterSawtooth {
                frequency,
                intensity,
                range,
            } => {
                let item = SineWaveEmitterProcessData::new_saw(*frequency, *intensity, *range, setting.clone());
                Rc::new(RefCell::new(item))
            }
            ENode::EmitterTriangle {
                frequency,
                intensity,
                range,
            } => {
                let item = SineWaveEmitterProcessData::new_triangle(*frequency, *intensity, *range, setting.clone());
                Rc::new(RefCell::new(item))
            }
            ENode::EmitterSquare {
                frequency,
                duty_rate,
                intensity,
                range,
            } => {
                let item =
                    SineWaveEmitterProcessData::new_square(*frequency, *duty_rate, *intensity, *range, setting.clone());
                Rc::new(RefCell::new(item))
            }
            _ => unreachable!("Unexpected branch."),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ESineWaveEmitterType {
    PinkNoise,
    WhiteNoise,
    Sine,
    Saw,
    Triangle,
    Square { duty_rate: f64 },
}

// ----------------------------------------------------------------------------
// TInputBufferOutputNone
// ----------------------------------------------------------------------------

/// [`TInputBufferOutputNone`]を実装しているアイテムの外部表示タイプ
pub type InputBufferOutputNonePtr = Rc<RefCell<dyn TInputBufferOutputNone>>;

/// インプットでバッファーを受け取り、自分の処理の中で消費して完結するためのTrait。
pub trait TInputBufferOutputNone: std::fmt::Debug {
    /// データアイテムの処理が終わったか？
    fn is_finished(&self) -> bool;

    /// 自分のタイムスタンプを返す。
    fn get_timestamp(&self) -> i64;

    /// 処理してみる。
    fn try_process(&mut self) -> EProcessResult;

    /// 中に`output`を更新する。
    fn update_input(&mut self, index: usize, output: EProcessOutput);

    fn set_child_count(&mut self, count: usize);
}

struct SInputBufferOutputNone;
impl SInputBufferOutputNone {
    fn create_from(node: &ENode, _: &Setting) -> InputBufferOutputNonePtr {
        match node {
            ENode::OutputFile { format, file_name } => {
                Rc::new(RefCell::new(OutputFileProcessData::new(format.clone(), file_name.clone())))
            }
            ENode::OutputLog { mode } => Rc::new(RefCell::new(OutputLogProcessData::new(*mode))),
            _ => unreachable!("Unexpected branch."),
        }
    }
}

// ----------------------------------------------------------------------------
// TInputBufferOutputBuffer
// ----------------------------------------------------------------------------

/// [`TInputBufferOutputBuffer`]を実装しているアイテムの外部表示タイプ
pub type InputBufferOutputBufferPtr = Rc<RefCell<dyn TInputBufferOutputBuffer>>;

/// インプットでバッファーを受け取り、自分の処理の中で消費して完結するためのTrait。
pub trait TInputBufferOutputBuffer: std::fmt::Debug {
    /// データアイテムの処理が終わったか？
    fn is_finished(&self) -> bool;

    /// 自分のタイムスタンプを返す。
    fn get_timestamp(&self) -> i64;

    /// 処理結果を返す。
    fn get_output(&self) -> ProcessOutputBuffer;

    /// 中に`output`を更新する。
    fn update_input(&mut self, index: usize, output: EProcessOutput);

    fn set_child_count(&mut self, count: usize);

    /// 処理してみる。
    fn try_process(&mut self) -> EProcessResult;
}

struct SInputBufferOutputBuffer;
impl SInputBufferOutputBuffer {
    fn create_from(node: &ENode, _: &Setting) -> InputBufferOutputBufferPtr {
        match node {
            ENode::AdapterEnvlopeAd {
                attack_time,
                decay_time,
                attack_curve,
                decay_curve,
            } => Rc::new(RefCell::new(AdapterEnvelopeAdProcessData::new(
                *attack_time,
                *decay_time,
                *attack_curve,
                *decay_curve,
            ))),
            ENode::AdapterEnvlopeAdsr {
                attack_time,
                decay_time,
                sustain_time,
                release_time,
                attack_curve,
                decay_curve,
                release_curve,
                sustain_value,
            } => Rc::new(RefCell::new(AdapterEnvelopeAdsrProcessData::new(
                *attack_time,
                *decay_time,
                *sustain_time,
                *release_time,
                *attack_curve,
                *decay_curve,
                *release_curve,
                *sustain_value,
            ))),
            _ => unreachable!("Unexpected branch."),
        }
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
