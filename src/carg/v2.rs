use std::{
    cell::RefCell,
    collections::{HashMap, HashSet, VecDeque},
    fs,
    io::{self, Write},
    rc::Rc,
};

use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{
    math::frequency::EFrequency,
    wave::{
        analyze::window::EWindowFunction,
        container::WaveBuilder,
        sample::UniformedSample,
        setting::{EFrequencyItem, WaveFormatSetting, WaveSound, WaveSoundSettingBuilder},
        stretch::pitch::{PitchShifterBufferSetting, PitchShifterBuilder},
    },
};

use super::{container::ENodeContainer, v1::EOutputFileFormat};

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
    OutputFile,
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
            ENode::OutputFile { .. } => Self::OutputFile,
        }
    }

    pub fn is_input(&self) -> bool {
        match self {
            Self::EmitterPinkNoise => true,
            Self::EmitterWhiteNoise => true,
            Self::EmitterSineWave => true,
            Self::EmitterSawtooth => true,
            Self::EmitterTriangle => true,
            Self::OutputFile => false,
        }
    }

    pub fn is_output(&self) -> bool {
        match self {
            Self::EmitterPinkNoise => false,
            Self::EmitterWhiteNoise => false,
            Self::EmitterSineWave => false,
            Self::EmitterSawtooth => false,
            Self::EmitterTriangle => false,
            Self::OutputFile => true,
        }
    }

    /// 自分が`output`と互換性のあるノードなのか？
    pub fn is_supported_by(&self, output: &Self) -> bool {
        match *output {
            // falseしかできない。
            Self::EmitterPinkNoise => false,
            Self::EmitterWhiteNoise => false,
            Self::EmitterSineWave => false,
            Self::EmitterSawtooth => false,
            Self::EmitterTriangle => false,
            // trueになれる。
            Self::OutputFile => match self {
                Self::EmitterPinkNoise => true,
                Self::EmitterWhiteNoise => true,
                Self::EmitterSineWave => true,
                Self::EmitterSawtooth => true,
                Self::EmitterTriangle => true,
                Self::OutputFile => false,
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
    /// 何かからファイルを出力する
    #[serde(rename = "output-file")]
    OutputFile {
        format: EOutputFileFormat,
        file_name: String,
    },
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
}

impl ENodeProcessData {
    /// ノードから処理アイテムを生成する。
    pub fn create_from(node: &ENode, setting: &Setting) -> Self {
        match node {
            ENode::EmitterPinkNoise { .. }
            | ENode::EmitterWhiteNoise { .. }
            | ENode::EmitterSineWave { .. }
            | ENode::EmitterTriangle { .. }
            | ENode::EmitterSawtooth { .. } => {
                ENodeProcessData::InputNoneOutputBuffer(SInputNoneOutputBuffer::create_from(node, setting))
            }
            ENode::OutputFile { .. } => {
                ENodeProcessData::InputBufferOutputNone(SInputBufferOutputNone::create_from(node, setting))
            }
        }
    }

    /// データアイテムの処理が終わったか？
    pub fn is_finished(&self) -> bool {
        match self {
            ENodeProcessData::InputNoneOutputBuffer(v) => v.borrow().is_finished(),
            ENodeProcessData::InputBufferOutputNone(v) => v.borrow().is_finished(),
        }
    }

    /// 自分のタイムスタンプを返す。
    pub fn get_timestamp(&self) -> i64 {
        match self {
            ENodeProcessData::InputNoneOutputBuffer(v) => v.borrow().get_timestamp(),
            ENodeProcessData::InputBufferOutputNone(v) => v.borrow().get_timestamp(),
        }
    }

    /// 処理してみる。
    pub fn try_process(&mut self) -> EProcessResult {
        match self {
            ENodeProcessData::InputNoneOutputBuffer(v) => v.borrow_mut().try_process(),
            ENodeProcessData::InputBufferOutputNone(v) => v.borrow_mut().try_process(),
        }
    }

    /// 中に`output`を更新する。
    pub fn update_input(&mut self, index: usize, output: EProcessOutput) {
        match self {
            ENodeProcessData::InputNoneOutputBuffer(_) => unreachable!("Unexpected branch"),
            ENodeProcessData::InputBufferOutputNone(v) => v.borrow_mut().update_input(index, output),
        }
    }

    /// 処理した後の出力を返す。
    pub fn get_output(&self) -> EProcessOutput {
        match self {
            ENodeProcessData::InputNoneOutputBuffer(v) => EProcessOutput::Buffer(v.borrow().get_output()),
            ENodeProcessData::InputBufferOutputNone(_) => EProcessOutput::None,
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
                let item = SineWaveEmitterProcessData {
                    common: ProcessControlItem::new(),
                    emitter_type: ESineWaveEmitterType::PinkNoise,
                    intensity: *intensity,
                    frequency: 0.0,
                    range: *range,
                    setting: setting.clone(),
                    output: None,
                };

                Rc::new(RefCell::new(item))
            }
            ENode::EmitterWhiteNoise { intensity, range } => {
                let item = SineWaveEmitterProcessData {
                    common: ProcessControlItem::new(),
                    emitter_type: ESineWaveEmitterType::WhiteNoise,
                    intensity: *intensity,
                    frequency: 0.0,
                    range: *range,
                    setting: setting.clone(),
                    output: None,
                };

                Rc::new(RefCell::new(item))
            }
            ENode::EmitterSineWave {
                frequency,
                intensity,
                range,
            } => {
                let item = SineWaveEmitterProcessData {
                    common: ProcessControlItem::new(),
                    emitter_type: ESineWaveEmitterType::Sine,
                    intensity: *intensity,
                    frequency: frequency.to_frequency(),
                    range: *range,
                    setting: setting.clone(),
                    output: None,
                };

                Rc::new(RefCell::new(item))
            }
            ENode::EmitterSawtooth {
                frequency,
                intensity,
                range,
            } => {
                let item = SineWaveEmitterProcessData {
                    common: ProcessControlItem::new(),
                    emitter_type: ESineWaveEmitterType::Saw,
                    intensity: *intensity,
                    frequency: frequency.to_frequency(),
                    range: *range,
                    setting: setting.clone(),
                    output: None,
                };

                Rc::new(RefCell::new(item))
            }
            ENode::EmitterTriangle {
                frequency,
                intensity,
                range,
            } => {
                let item = SineWaveEmitterProcessData {
                    common: ProcessControlItem::new(),
                    emitter_type: ESineWaveEmitterType::Triangle,
                    intensity: *intensity,
                    frequency: frequency.to_frequency(),
                    range: *range,
                    setting: setting.clone(),
                    output: None,
                };

                Rc::new(RefCell::new(item))
            }
            _ => unreachable!("Unexpected branch."),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ESineWaveEmitterType {
    PinkNoise,
    WhiteNoise,
    Sine,
    Saw,
    Triangle,
}

/// 正弦波を使って波形のバッファを作るための構造体
#[derive(Debug)]
pub struct SineWaveEmitterProcessData {
    common: ProcessControlItem,
    emitter_type: ESineWaveEmitterType,
    intensity: f64,
    frequency: f64,
    range: EmitterRange,
    setting: Setting,
    /// 処理後に出力情報が保存されるところ。
    output: Option<ProcessOutputBuffer>,
}

impl TInputNoneOutputBuffer for SineWaveEmitterProcessData {
    fn is_finished(&self) -> bool {
        self.common.state == EProcessState::Finished
    }

    /// 自分のタイムスタンプを返す。
    fn get_timestamp(&self) -> i64 {
        self.common.process_timestamp
    }

    fn get_output(&self) -> ProcessOutputBuffer {
        assert!(self.output.is_some());
        self.output.as_ref().unwrap().clone()
    }

    fn set_child_count(&mut self, count: usize) {
        self.common.child_count = count;
    }

    fn try_process(&mut self) -> EProcessResult {
        if self.common.state == EProcessState::Finished {
            return EProcessResult::Finished;
        }

        let frequency = match self.emitter_type {
            ESineWaveEmitterType::PinkNoise => EFrequencyItem::PinkNoise,
            ESineWaveEmitterType::WhiteNoise => EFrequencyItem::WhiteNoise,
            ESineWaveEmitterType::Sine => EFrequencyItem::Constant {
                frequency: self.frequency,
            },
            ESineWaveEmitterType::Saw => EFrequencyItem::Constant {
                frequency: self.frequency,
            },
            ESineWaveEmitterType::Triangle => EFrequencyItem::Triangle {
                frequency: self.frequency,
            },
        };
        let sound_setting = WaveSoundSettingBuilder::default()
            .frequency(frequency)
            .length_sec(self.range.length as f32)
            .intensity(self.intensity)
            .build()
            .unwrap();

        let format = WaveFormatSetting {
            samples_per_sec: self.setting.sample_rate as u32,
            bits_per_sample: crate::wave::setting::EBitsPerSample::Bits16,
        };
        let sound = WaveSound::from_setting(&format, &sound_setting);

        let mut buffer: Vec<UniformedSample> = vec![];
        for fragment in sound.sound_fragments {
            buffer.extend(&fragment.buffer);
        }

        // outputのどこかに保持する。
        self.output = Some(ProcessOutputBuffer {
            buffer,
            setting: self.setting.clone(),
            range: self.range,
        });

        // 状態変更。
        self.common.state = EProcessState::Finished;
        self.common.process_timestamp += 1;
        return EProcessResult::Finished;
    }
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

#[derive(Debug)]
pub struct OutputFileProcessData {
    common: ProcessControlItem,
    format: EOutputFileFormat,
    file_name: String,
    inputs: HashMap<usize, ProcessOutputBuffer>,
}

impl TInputBufferOutputNone for OutputFileProcessData {
    /// データアイテムの処理が終わったか？
    fn is_finished(&self) -> bool {
        self.common.state == EProcessState::Finished
    }

    /// 自分のタイムスタンプを返す。
    fn get_timestamp(&self) -> i64 {
        self.common.process_timestamp
    }

    fn set_child_count(&mut self, count: usize) {
        self.common.child_count = count;
    }

    fn update_input(&mut self, index: usize, output: EProcessOutput) {
        match output {
            EProcessOutput::None => unimplemented!("Unexpected branch."),
            EProcessOutput::Buffer(v) => {
                self.inputs.insert(index, v);
            }
        }
    }

    fn try_process(&mut self) -> EProcessResult {
        // Childrenが全部送信完了したら処理が行える。
        // commonで初期Childrenの数を比較するだけでいいかも。
        if self.inputs.len() < self.common.child_count {
            return EProcessResult::Pending;
        }
        assert!(self.common.child_count > 0);

        // inputsのサンプルレートが同じかを確認する。
        let source_sample_rate = self.inputs.get(&0).unwrap().setting.sample_rate;
        for (_, input) in self.inputs.iter().skip(1) {
            assert!(input.setting.sample_rate == source_sample_rate);
        }

        // ここで各bufferを組み合わせて、一つにしてから書き込む。
        let mut final_buffer_length = 0usize;
        let ref_vec = self
            .inputs
            .iter()
            .map(|(_, info)| {
                let buffer_length = info.buffer.len();
                let start_index = (info.range.start * (info.setting.sample_rate as f64)).floor() as usize;
                let exclusive_end_index = start_index + buffer_length;

                final_buffer_length = exclusive_end_index.max(final_buffer_length);

                (info, start_index)
            })
            .collect_vec();

        // 書き込み
        let mut new_buffer = vec![];
        new_buffer.resize(final_buffer_length, UniformedSample::default());
        for (ref_buffer, start_index) in ref_vec {
            for src_i in 0..ref_buffer.buffer.len() {
                let dest_i = start_index + src_i;
                new_buffer[dest_i] += ref_buffer.buffer[src_i];
            }
        }

        let container = match self.format {
            EOutputFileFormat::WavLPCM16 { sample_rate } => {
                // もしsettingのsampling_rateがoutputのsampling_rateと違ったら、
                // リサンプリングをしなきゃならない。
                let source_sample_rate = source_sample_rate as f64;
                let dest_sample_rate = sample_rate as f64;

                let processed_container = {
                    let pitch_rate = source_sample_rate / dest_sample_rate;
                    if pitch_rate == 1.0 {
                        new_buffer
                    } else {
                        PitchShifterBuilder::default()
                            .pitch_rate(pitch_rate)
                            .window_size(128)
                            .window_function(EWindowFunction::None)
                            .build()
                            .unwrap()
                            .process_with_buffer(&PitchShifterBufferSetting { buffer: &new_buffer })
                            .unwrap()
                    }
                };

                WaveBuilder {
                    samples_per_sec: sample_rate as u32,
                    bits_per_sample: 16,
                }
                .build_container(processed_container)
                .unwrap()
            }
        };

        // 書き込み。
        {
            let dest_file = fs::File::create(&self.file_name).expect("Could not create 500hz.wav.");
            let mut writer = io::BufWriter::new(dest_file);
            container.write(&mut writer);
            writer.flush().expect("Failed to flush writer.")
        }

        // 状態変更。
        self.common.state = EProcessState::Finished;
        self.common.process_timestamp += 1;
        return EProcessResult::Finished;
    }
}

struct SInputBufferOutputNone;
impl SInputBufferOutputNone {
    fn create_from(node: &ENode, _: &Setting) -> InputBufferOutputNonePtr {
        match node {
            ENode::OutputFile { format, file_name } => {
                let item = OutputFileProcessData {
                    common: ProcessControlItem::new(),
                    format: format.clone(),
                    file_name: file_name.clone(),
                    inputs: HashMap::new(),
                };

                Rc::new(RefCell::new(item))
            }
            _ => unreachable!("Unexpected branch."),
        }
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
