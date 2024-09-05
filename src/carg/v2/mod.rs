use std::{
    cell::RefCell,
    collections::{HashMap, HashSet, VecDeque},
    os::windows::process,
    rc::Rc,
};

use adapter::{envelope_ad::AdapterEnvelopeAdProcessData, envelope_adsr::AdapterEnvelopeAdsrProcessData};
use emitter::SineWaveEmitterProcessData;
use itertools::Itertools;
use output::{output_file::OutputFileProcessData, output_log::OutputLogProcessData};
use serde::{Deserialize, Serialize};

use crate::{
    math::{frequency::EFrequency, timer::Timer},
    wave::sample::UniformedSample,
};

use super::{container::ENodeContainer, v1::EOutputFileFormat};

pub mod adapter;
pub mod emitter;
pub mod output;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Setting {
    /// 更新時の推奨されるサンプル数。
    /// たとえば48kHzだと約21ms弱ぐらいになる。
    /// この値は必ず2のべき乗数でなければならない。
    sample_count_frame: usize,
    /// 音生成のために使うサンプルレートを指す。0より上であること。
    sample_rate: u64,
}

/// 内部識別処理に使うEnum。
#[derive(Debug, Clone, Copy)]
enum ENodeSpecifier {
    InternalStartPin,
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
            ENode::InternalStartPin => Self::InternalStartPin,
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

    /// Inputノードとして入れられるか
    pub fn is_input(&self) -> bool {
        match self {
            Self::InternalStartPin
            | Self::EmitterPinkNoise
            | Self::EmitterWhiteNoise
            | Self::EmitterSineWave
            | Self::EmitterSawtooth
            | Self::EmitterTriangle
            | Self::EmitterSquare
            | Self::AdapterEnvlopeAd
            | Self::AdapterEnvlopeAdsr => true,
            Self::OutputFile | Self::OutputLog => false,
        }
    }

    /// 自分が`output`と互換性のあるノードなのか？
    pub fn can_connect_to(&self, output: &Self) -> bool {
        match *output {
            Self::InternalStartPin => false,
            // falseしかできない。
            Self::EmitterPinkNoise
            | Self::EmitterWhiteNoise
            | Self::EmitterSineWave
            | Self::EmitterSawtooth
            | Self::EmitterTriangle
            | Self::EmitterSquare => match self {
                Self::InternalStartPin => true,
                _ => false,
            },
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
                _ => false,
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
                _ => false,
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
                _ => false,
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
                _ => false,
            },
        }
    }

    /// 処理が可能なノードが？
    pub fn can_process(&self) -> bool {
        match self {
            _ => true,
        }
    }
}

///
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum ENode {
    /// 内部制御用。
    #[serde(rename = "_start_pin")]
    InternalStartPin,
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

    pub fn can_connect_to(&self, output: &ENode) -> bool {
        ENodeSpecifier::from_node(self).can_connect_to(&ENodeSpecifier::from_node(output))
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
    pub input: String,
    /// 出力側
    pub output: String,
}

/// v2バージョンにパーシングする。
pub fn parse_v2(info: &serde_json::Value) -> anyhow::Result<ENodeContainer> {
    // Input, Setting, Outputがちゃんとあるとみなして吐き出す。
    let setting: Setting = serde_json::from_value(info["setting"].clone())?;
    if !setting.sample_count_frame.is_power_of_two() {
        return Err(anyhow::anyhow!(
            "Given `sample_count_frame` is not power of two. (256, 512, 1024...)"
        ));
    }

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

/// 特殊なインプットノード名なのかを確認する。
pub fn is_special_input_node(input: &str) -> bool {
    if input == "_start_pin" {
        return true;
    }

    return false;
}

/// 次のことを検査する。
///
/// * inputとoutputが空白なものがあるかを確認する。
/// * それぞれのノードに対してCycleになっていないかを確認する。
pub fn validate_node_relations(nodes: &HashMap<String, ENode>, relations: &[Relation]) -> anyhow::Result<()> {
    let mut is_start_node_exist = false;
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
        // もしくは特殊ノードなのかも確認。
        if !is_special_input_node(&relation.input) {
            let node_name = &relation.input;
            if !nodes.contains_key(node_name) {
                return Err(anyhow::anyhow!("Given node {} is not exist in node map.", node_name));
            }
            if !nodes[node_name].is_input() {
                return Err(anyhow::anyhow!("Given node {} is not for input.", node_name));
            }
        } else {
            is_start_node_exist = true;
        }

        // そしてinputとoutputの互換性を確認する。
        // 具体的にはoutputに対してinputの組み方とタイプを検証する。
        {
            let output_node_name = &relation.output;
            let output = &nodes[output_node_name];

            if !nodes[&relation.input].can_connect_to(output) {
                return Err(anyhow::anyhow!(
                    "Input node {} does not support output node {}.",
                    &relation.input,
                    output_node_name
                ));
            }
        }
    }

    if !is_start_node_exist {
        return Err(anyhow::anyhow!("There is no start pin node. '_start_pin'."));
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

pub type ItemSPtr<T: ?Sized> = Rc<RefCell<T>>;

pub struct SItemSPtr;
impl SItemSPtr {
    #[inline]
    pub fn new<T>(value: T) -> ItemSPtr<T> {
        Rc::new(RefCell::new(value))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProcessCommonInput {
    /// 前のフレーム処理から何秒経ったか
    pub elapsed_time: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessProcessorInput {
    pub common: ProcessCommonInput,
    pub children_states: Vec<bool>,
}

impl ProcessProcessorInput {
    /// 子供がないかそれとも子供が全部処理が終わったか？
    pub fn is_children_all_finished(&self) -> bool {
        if self.children_states.is_empty() {
            return true;
        }

        self.children_states.iter().all(|v| *v == true)
    }
}

pub type RelationTreeNodePtr = ItemSPtr<RelationTreeNode>;

/// 木のノードアイテム。
#[derive(Debug)]
pub struct RelationTreeNode {
    /// ノードの名前
    pub name: String,
    /// ノード識別タイプ
    specifier: ENodeSpecifier,
    /// 前からこのノードを依存する前ノードのマップ
    prev_nodes: HashMap<String, RelationTreeNodePtr>,
    /// 次に伝播するノードのマップ
    next_nodes: HashMap<String, RelationTreeNodePtr>,
    /// 子ノードのインプットを取得するためのタイムスタンプのリスト。[`Self::children`]と同じ数であること。
    sync_timestamps: Vec<i64>,
    /// このノードから処理するアイテム
    processor: ENodeProcessData,
}

impl RelationTreeNode {
    /// ノードアイテムを作る。
    pub fn new_item(name: &str, specifier: ENodeSpecifier, processor: &ENodeProcessData) -> RelationTreeNodePtr {
        SItemSPtr::new(RelationTreeNode {
            name: name.to_owned(),
            specifier,
            prev_nodes: HashMap::new(),
            next_nodes: HashMap::new(),
            sync_timestamps: vec![],
            processor: processor.clone(),
        })
    }

    /// 前からこのノードを依存する前ノードを登録する。
    pub fn append_prev_node(&mut self, node_name: String, prev: RelationTreeNodePtr) {
        self.prev_nodes.insert(node_name, prev);
    }

    /// 次に処理を伝播するノードを登録する。
    pub fn append_next_node(&mut self, node_name: String, next: RelationTreeNodePtr) {
        self.next_nodes.insert(node_name, next);
    }

    /// ノード自分の処理が可能か？
    pub fn can_process(&self) -> bool {
        if !self.specifier.can_process() {
            return false;
        }

        // 自分のノードも視野に入れる。
        match &self.processor {
            ENodeProcessData::InputNoneOutputBuffer(v) => v.borrow().can_process(),
            ENodeProcessData::InputBufferOutputNone(v) => v.borrow().can_process(),
            ENodeProcessData::InputBufferOutputBuffer(v) => v.borrow().can_process(),
            ENodeProcessData::InternalStartNode => true,
        }
    }

    pub fn process(&mut self, input: &ProcessCommonInput) {
        let input = ProcessProcessorInput {
            common: *input,
            children_states: self.prev_nodes.iter().map(|(_, v)| v.borrow().is_finished()).collect_vec(),
        };

        // 自分のノードも視野に入れる。
        let _state = match &self.processor {
            ENodeProcessData::InputNoneOutputBuffer(v) => v.borrow_mut().try_process(&input),
            ENodeProcessData::InputBufferOutputNone(v) => v.borrow_mut().try_process(&input),
            ENodeProcessData::InputBufferOutputBuffer(v) => v.borrow_mut().try_process(&input),
            ENodeProcessData::InternalStartNode => EProcessResult::Finished,
        };

        // inputを入れる。inputをどう扱うかはnextノード各自でお任せする。
        let output = self.processor.get_output();
        for (_, node) in &mut self.next_nodes {
            node.borrow_mut().update_input(&self.name, &output);
        }
    }

    /// 次につながっているノードのリストを返す。
    pub fn get_next_nodes(&self) -> Vec<RelationTreeNodePtr> {
        self.next_nodes.iter().map(|(_, v)| v.clone()).collect_vec()
    }

    /// 処理した後の出力を返す。
    pub fn get_output(&self) -> EProcessOutput {
        self.processor.get_output()
    }

    /// 自分のノードに[`input`]を入れるか判定して適切に処理する。
    pub fn update_input(&mut self, node_name: &str, input: &EProcessOutput) {
        // 自分のノードも視野に入れる。
        match &self.processor {
            ENodeProcessData::InputBufferOutputNone(v) => v.borrow_mut().update_input(node_name, input),
            ENodeProcessData::InputBufferOutputBuffer(v) => v.borrow_mut().update_input(node_name, input),
            _ => (),
        };
    }

    /// ノードの処理活動が終わったか？
    pub fn is_finished(&self) -> bool {
        // 前のノードが全部Finishedであること。
        if self.prev_nodes.iter().any(|(_, node)| !node.borrow().is_finished()) {
            return false;
        }

        // 自分のノードも視野に入れる。
        let is_finished = match &self.processor {
            ENodeProcessData::InputNoneOutputBuffer(v) => v.borrow().is_finished(),
            ENodeProcessData::InputBufferOutputNone(v) => v.borrow().is_finished(),
            ENodeProcessData::InputBufferOutputBuffer(v) => v.borrow().is_finished(),
            ENodeProcessData::InternalStartNode => true,
        };
        is_finished
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
    /// 経過した時間（秒単位）
    pub elapsed_time: f64,
}

impl ProcessControlItem {
    pub fn new() -> Self {
        Self {
            state: EProcessState::Stopped,
            process_timestamp: 0i64,
            elapsed_time: 0.0,
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
    InternalStartNode,
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
                Self::InputNoneOutputBuffer(SInputNoneOutputBuffer::create_from(node, setting))
            }
            ENode::AdapterEnvlopeAd { .. } | ENode::AdapterEnvlopeAdsr { .. } => {
                Self::InputBufferOutputBuffer(SInputBufferOutputBuffer::create_from(node, setting))
            }
            ENode::OutputLog { .. } | ENode::OutputFile { .. } => {
                Self::InputBufferOutputNone(SInputBufferOutputNone::create_from(node, setting))
            }
            ENode::InternalStartPin => Self::InternalStartNode,
        }
    }

    /// データアイテムの処理が終わったか？
    pub fn is_finished(&self) -> bool {
        match self {
            ENodeProcessData::InputNoneOutputBuffer(v) => v.borrow().is_finished(),
            ENodeProcessData::InputBufferOutputNone(v) => v.borrow().is_finished(),
            ENodeProcessData::InputBufferOutputBuffer(v) => v.borrow().is_finished(),
            ENodeProcessData::InternalStartNode => true,
        }
    }

    /// 処理してみる。
    pub fn try_process(&mut self, input: &ProcessProcessorInput) -> EProcessResult {
        match self {
            ENodeProcessData::InputNoneOutputBuffer(v) => v.borrow_mut().try_process(input),
            ENodeProcessData::InputBufferOutputNone(v) => v.borrow_mut().try_process(input),
            ENodeProcessData::InputBufferOutputBuffer(v) => v.borrow_mut().try_process(input),
            ENodeProcessData::InternalStartNode => EProcessResult::Finished,
        }
    }

    /// 処理した後の出力を返す。
    pub fn get_output(&self) -> EProcessOutput {
        match self {
            ENodeProcessData::InputNoneOutputBuffer(v) => EProcessOutput::Buffer(v.borrow().get_output()),
            ENodeProcessData::InputBufferOutputBuffer(v) => EProcessOutput::Buffer(v.borrow().get_output()),
            _ => EProcessOutput::None,
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

pub trait TProcess {
    /// データアイテムの処理が終わったか？
    fn is_finished(&self) -> bool;

    /// 自分が処理可能なノードなのかを確認する。
    fn can_process(&self) -> bool;

    fn try_process(&mut self, input: &ProcessProcessorInput) -> EProcessResult;
}

// ----------------------------------------------------------------------------
// TInputNoneOutputBuffer
// ----------------------------------------------------------------------------

/// [`TInputNoneOutputBuffer`]を実装しているアイテムの外部表示タイプ
pub type InputNoneOutputBufferPtr = ItemSPtr<dyn TInputNoneOutputBuffer>;

/// 処理からOutputでバッファを返すためのTrait。
pub trait TInputNoneOutputBuffer: std::fmt::Debug + TProcess {
    /// 処理結果を返す。
    fn get_output(&self) -> ProcessOutputBuffer;
}

/// [`TInputNoneOutputBuffer`]のインスタンス生成ファクトリー。
struct SInputNoneOutputBuffer;
impl SInputNoneOutputBuffer {
    fn create_from(node: &ENode, setting: &Setting) -> InputNoneOutputBufferPtr {
        match node {
            ENode::EmitterPinkNoise { intensity, range } => {
                let item = SineWaveEmitterProcessData::new_pink(*intensity, *range, setting.clone());
                SItemSPtr::new(item)
            }
            ENode::EmitterWhiteNoise { intensity, range } => {
                let item = SineWaveEmitterProcessData::new_white(*intensity, *range, setting.clone());
                SItemSPtr::new(item)
            }
            ENode::EmitterSineWave {
                frequency,
                intensity,
                range,
            } => {
                let item = SineWaveEmitterProcessData::new_sine(*frequency, *intensity, *range, setting.clone());
                SItemSPtr::new(item)
            }
            ENode::EmitterSawtooth {
                frequency,
                intensity,
                range,
            } => {
                let item = SineWaveEmitterProcessData::new_saw(*frequency, *intensity, *range, setting.clone());
                SItemSPtr::new(item)
            }
            ENode::EmitterTriangle {
                frequency,
                intensity,
                range,
            } => {
                let item = SineWaveEmitterProcessData::new_triangle(*frequency, *intensity, *range, setting.clone());
                SItemSPtr::new(item)
            }
            ENode::EmitterSquare {
                frequency,
                duty_rate,
                intensity,
                range,
            } => {
                let item =
                    SineWaveEmitterProcessData::new_square(*frequency, *duty_rate, *intensity, *range, setting.clone());
                SItemSPtr::new(item)
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
pub type InputBufferOutputNonePtr = ItemSPtr<dyn TInputBufferOutputNone>;

/// インプットでバッファーを受け取り、自分の処理の中で消費して完結するためのTrait。
pub trait TInputBufferOutputNone: std::fmt::Debug + TProcess {
    /// 自分のノードに[`input`]を入れるか判定して適切に処理する。
    fn update_input(&mut self, node_name: &str, input: &EProcessOutput);
}

struct SInputBufferOutputNone;
impl SInputBufferOutputNone {
    fn create_from(node: &ENode, _: &Setting) -> InputBufferOutputNonePtr {
        match node {
            ENode::OutputFile { format, file_name } => {
                SItemSPtr::new(OutputFileProcessData::new(format.clone(), file_name.clone()))
            }
            ENode::OutputLog { mode } => SItemSPtr::new(OutputLogProcessData::new(*mode)),
            _ => unreachable!("Unexpected branch."),
        }
    }
}

// ----------------------------------------------------------------------------
// TInputBufferOutputBuffer
// ----------------------------------------------------------------------------

/// [`TInputBufferOutputBuffer`]を実装しているアイテムの外部表示タイプ
pub type InputBufferOutputBufferPtr = ItemSPtr<dyn TInputBufferOutputBuffer>;

/// インプットでバッファーを受け取り、自分の処理の中で消費して完結するためのTrait。
pub trait TInputBufferOutputBuffer: std::fmt::Debug + TProcess {
    /// 処理結果を返す。
    fn get_output(&self) -> ProcessOutputBuffer;

    /// 自分のノードに[`input`]を入れるか判定して適切に処理する。
    fn update_input(&mut self, node_name: &str, input: &EProcessOutput);
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
            } => SItemSPtr::new(AdapterEnvelopeAdProcessData::new(
                *attack_time,
                *decay_time,
                *attack_curve,
                *decay_curve,
            )),
            ENode::AdapterEnvlopeAdsr {
                attack_time,
                decay_time,
                sustain_time,
                release_time,
                attack_curve,
                decay_curve,
                release_curve,
                sustain_value,
            } => SItemSPtr::new(AdapterEnvelopeAdsrProcessData::new(
                *attack_time,
                *decay_time,
                *sustain_time,
                *release_time,
                *attack_curve,
                *decay_curve,
                *release_curve,
                *sustain_value,
            )),
            _ => unreachable!("Unexpected branch."),
        }
    }
}

pub fn process_v2(setting: &Setting, nodes: &HashMap<String, ENode>, relations: &[Relation]) -> anyhow::Result<()> {
    // 下で`_start_pin`のチェックもやってくれる。
    validate_node_relations(nodes, &relations)?;

    // チェックができたので(validation)、relationを元にGraphを生成する。
    // ただしそれぞれの独立したoutputをルートにして必要となるinputを子としてツリーを構成する。
    // outputがinputとして使われているものは一つの独立したツリーとして構成しない。
    let node_map = {
        let mut map = HashMap::new();
        // 各ノードから処理に使うためのアイテムを全部生成しておく。
        for (node_name, node) in nodes {
            let processor = ENodeProcessData::create_from(node, setting);
            let specifier = ENodeSpecifier::from_node(node);
            let node = RelationTreeNode::new_item(&node_name, specifier, &processor);
            map.insert(node_name.clone(), node);
        }

        map
    };

    // relationsからnext_nodeを入れる。
    for relation in relations {
        let input = &relation.input;
        let output = &relation.output;

        // prev → next
        {
            let output_node = node_map.get(output).unwrap().clone();
            node_map[input].borrow_mut().append_next_node(output.clone(), output_node);
        }
        // next → prev
        {
            let input_node = node_map.get(input).unwrap().clone();
            node_map[output].borrow_mut().append_prev_node(input.clone(), input_node);
        }
    }

    // そしてcontrol_itemsとnodes、output_treeを使って処理をする。+ setting.
    // VecDequeをStackのように扱って、DFSをコールスタックを使わずに実装することができそう。
    let start_node = node_map.get("_start_pin").unwrap().clone();
    let tick_threshold = (setting.sample_count_frame as f64) / (setting.sample_rate as f64);
    let mut tick_timer = Timer::from_second(tick_threshold);

    // 終了条件は、すべてのノードが終わった時。
    let mut node_queue = VecDeque::new();
    loop {
        let duration = tick_timer.tick();
        let second = duration.as_secs_f64();

        // 共通で使う処理時の入力。
        let input = ProcessCommonInput { elapsed_time: second };
        node_queue.push_back(start_node.clone());

        //
        let mut is_all_finished = true;
        while !node_queue.is_empty() {
            let process_node = node_queue.pop_front().unwrap();
            process_node.borrow_mut().process(&input);

            // 次ノードをQueueに入れる。
            let next_nodes = process_node.borrow().get_next_nodes();
            let is_node_end = next_nodes.is_empty();
            for next_node in next_nodes.into_iter().filter(|v| v.borrow().can_process()) {
                node_queue.push_back(next_node);
            }

            // もしnextがなければ、自分をfinalだとみなしてstartから自分までの処理が終わってるかを確認する。
            if is_node_end {
                is_all_finished &= process_node.borrow().is_finished();
            }
        }

        if is_all_finished {
            break;
        }
    }

    let duration = tick_timer.tick();
    println!("{:?}", duration);

    Ok(())
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
