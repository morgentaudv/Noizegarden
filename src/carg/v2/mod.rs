use super::container::ENodeContainer;
use crate::carg::v2::meta::node::{ENode, MetaNodeContainer};
use crate::carg::v2::meta::process::{process_category, EProcessCategoryFlag, StartItemGroup};
use crate::carg::v2::meta::setting::{ETimeTickMode, Setting};
use crate::carg::v2::meta::system::{system_category, TSystemCategory};
use crate::carg::v2::meta::{pin_category, EPinCategoryFlag};
use crate::carg::v2::node::common::ProcessControlItem;
use crate::carg::v2::node::{process_result, RelationTreeNode};
use crate::carg::v2::utility::{update_process_graph_connection, validate_node_relations};
use crate::device::{AudioDevice, AudioDeviceConfig, AudioDeviceProxyWeakPtr};
use crate::wave::analyze::sine_freq::SineFrequency;
use crate::{math::timer::Timer, wave::sample::UniformedSample};
use itertools::Itertools;
use meta::relation::Relation;
use num_traits::{One, Zero};
use serde::{Deserialize, Serialize};
use std::rc::Weak;
use std::thread::sleep;
use std::time::Duration;
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    rc::Rc,
};

pub mod adapter;
pub mod analyzer;
pub mod base;
pub mod emitter;
pub mod filter;
pub mod meta;
pub mod mix;
pub mod node;
pub mod output;
mod special;
mod utility;

/// シングルスレッド、通常参照
pub type ItemSPtr<T> = Rc<RefCell<T>>;

/// シングルスレッド、弱参照
pub type ItemWPtr<T> = Weak<RefCell<T>>;

pub struct SItemSPtr;
impl SItemSPtr {
    #[inline]
    pub fn new<T>(value: T) -> ItemSPtr<T> {
        Rc::new(RefCell::new(value))
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

/// v2バージョンにパーシングする。
pub fn parse_v2(info: &serde_json::Value) -> anyhow::Result<ENodeContainer> {
    // Input, Setting, Outputがちゃんとあるとみなして吐き出す。
    let setting = Setting::from_serde_value(info["setting"].clone())?;
    let nodes: HashMap<String, ENode> = serde_json::from_value(info["node"].clone())?;
    let relations: Vec<Relation> = serde_json::from_value(info["relation"].clone())?;

    // まとめて出力。
    let container = ENodeContainer::V2 {
        setting,
        nodes,
        relations,
    };
    Ok(container)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProcessCommonInput {
    /// `elapsed_time`の解釈方法
    pub time_tick_mode: ETimeTickMode,
    /// スタートから何秒経ったか
    pub elapsed_time: f64,
    /// 前のフレーム処理から何秒経ったか
    pub frame_time: f64,
    /// 処理カテゴリ
    pub category: EProcessCategoryFlag,
    /// 各チャンネル別にとればよさそうなサンプルの数。
    /// `time_tick_mode`が[`ETimeTickMode::Realtime`]な時だけ有効。
    pub required_channel_samples: usize,
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

// ----------------------------------------------------------------------------
// EProcessOutput
// ----------------------------------------------------------------------------

#[derive(Debug)]
pub enum EProcessOutput {
    None,
    BufferMono(ProcessOutputBuffer),
    BufferStereo(ProcessOutputBufferStereo),
    Text(ProcessOutputText),
    Frequency(ProcessOutputFrequency),
}

impl EProcessOutput {
    /// 自分が`categories`の範疇に入れるかを確認する。
    pub fn check(&self, categories: EPinCategoryFlag) -> bool {
        let self_flag = self.as_pin_category_flag();
        categories & self_flag == self_flag
    }

    pub fn as_pin_category_flag(&self) -> EPinCategoryFlag {
        match self {
            Self::None => pin_category::START,
            Self::BufferMono(_) => pin_category::BUFFER_MONO,
            Self::BufferStereo(_) => pin_category::BUFFER_STEREO,
            Self::Text(_) => pin_category::TEXT,
            Self::Frequency(_) => pin_category::FREQUENCY,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessOutputBuffer {
    buffer: Vec<UniformedSample>,
    //range: EmitterRange,
    setting: Setting,
    sample_offset: usize,
}

impl ProcessOutputBuffer {
    pub fn new(buffer: Vec<UniformedSample>, setting: Setting) -> Self {
        Self {
            buffer,
            setting,
            //range: EmitterRange {
            //    start: 0.0,
            //    length: 0.0,
            //},
            sample_offset: 0usize,
        }
    }

    pub fn new_sample_offset(buffer: Vec<UniformedSample>, setting: Setting, offset: usize) -> Self {
        let mut item = Self::new(buffer, setting);
        item.sample_offset = offset;
        item
    }
}

#[derive(Debug, Clone)]
pub struct ProcessOutputBufferStereo {
    ch_left: Vec<UniformedSample>,
    ch_right: Vec<UniformedSample>,
    setting: Setting,
}

#[derive(Debug, Clone)]
pub struct ProcessOutputText {
    text: String,
}

#[derive(Debug, Clone)]
pub struct ProcessOutputFrequency {
    frequencies: Vec<SineFrequency>,
    analyzed_sample_len: usize,
    overlap: bool,
}

pub trait TProcess: std::fmt::Debug {
    /// データアイテムの処理が終わったか？
    fn is_finished(&self) -> bool;

    /// 自分が処理可能なノードなのかを確認する。
    fn can_process(&self) -> bool;

    /// 共用アイテムの参照を返す。
    fn get_common_ref(&self) -> &ProcessControlItem;

    /// 共用アイテムの可変参照を返す。
    fn get_common_mut(&mut self) -> &mut ProcessControlItem;

    fn try_process(&mut self, input: &ProcessProcessorInput);
}

/// [`TProcessItem`]traitで使う構造体。
pub struct ProcessItemCreateSetting<'a> {
    pub node: &'a ENode,
    pub setting: &'a Setting,
}

pub struct ProcessItemCreateSettingSystem<'a> {
    pub audio_device: Option<&'a AudioDeviceProxyWeakPtr>,
}

/// アイテムの生成の処理をまとめるためのtrait。
/// 処理アイテム自体はこれを持っても、もたなくてもいいができればこれも[`TProcess`]と一緒に実装した方がいい。
pub trait TProcessItem: TProcess {
    /// アイテムの作成ができるかを確認するための関数。
    fn can_create_item(setting: &ProcessItemCreateSetting) -> anyhow::Result<()>;

    /// 処理アイテムを生成するための関数。
    fn create_item(
        setting: &ProcessItemCreateSetting,
        system_setting: &ProcessItemCreateSettingSystem,
    ) -> anyhow::Result<TProcessItemPtr>;
}

/// [`TProcess`]を実装しているアイテムの外部表示タイプ
pub type TProcessItemPtr = ItemSPtr<dyn TProcess>;

pub fn process_v2(setting: &Setting, nodes: HashMap<String, ENode>, relations: &[Relation]) -> anyhow::Result<()> {
    // 下で`_start_pin`のチェックもやってくれる。
    let node_container = MetaNodeContainer { map: nodes };
    validate_node_relations(&node_container, &relations)?;

    // 依存システムの初期化
    let dependent_systems = node_container.get_dependent_system_categories();
    let mut audio_device_weak_proxy = None;
    if dependent_systems != system_category::NONE {
        // AudioDeviceの初期化
        if !(dependent_systems & system_category::AUDIO_DEVICE).is_zero() {
            assert!(setting.channels > 0);

            let mut config = AudioDeviceConfig::new();
            config
                .set_channels(setting.channels)
                .set_sample_rate(setting.sample_rate as usize);

            audio_device_weak_proxy = Some(AudioDevice::initialize(config));
        }
    }
    let system_setting = ProcessItemCreateSettingSystem {
        audio_device: audio_device_weak_proxy.as_ref(),
    };

    // チェックができたので(validation)、relationを元にGraphを生成する。
    // ただしそれぞれの独立したoutputをルートにして必要となるinputを子としてツリーを構成する。
    let node_map = {
        let mut map = HashMap::new();

        // 各ノードから処理に使うためのアイテムを全部生成しておく。
        // 中でinputピンとoutputピンを作る。
        for (node_name, node) in &node_container.map {
            let processor = node.create_from(setting, &system_setting);
            let node = RelationTreeNode::new_item(&node_name, processor);
            map.insert(node_name.clone(), node);
        }

        // relationsからnext_nodeを入れる。
        for relation in relations {
            let prev = &relation.prev;
            let next = &relation.next;

            // prev → next
            {
                let prev_node = map.get(&prev.node).unwrap().clone();
                let output_pin = prev_node.borrow().get_output_pin(&prev.pin).unwrap();

                // これはnext側にprevを連結するため。
                let mut borrowed = map[&next.node].borrow_mut();
                borrowed.link_pin_output_to_input(&next.pin, output_pin);
                borrowed.append_prev_node(prev.node.clone(), prev_node);
            }
            // next → prev
            {
                let next_node = map.get(&next.node).unwrap().clone();
                let input_pin = next_node.borrow().get_input_pin(&next.pin).unwrap();

                // これはprev側にnextを連結するため。
                let mut borrowed = map[&prev.node].borrow_mut();
                borrowed.link_pin_input_to_output(&prev.pin, input_pin);
                borrowed.append_next_node(next.node.clone(), next_node);
            }
        }

        map
    };

    // 24-12-13 このグラフで使う「処理順」を一つずつ作って
    update_process_graph_connection(&node_map);
    let start_item_groups = StartItemGroup::initialize_groups_with(
        node_container.get_using_process_categories(),
        &node_map.iter().map(|(_, v)| Rc::downgrade(&v.clone())).collect_vec(),
    );

    // そしてcontrol_itemsとnodes、output_treeを使って処理をする。+ setting.
    // VecDequeをStackのように扱って、DFSをコールスタックを使わずに実装することができそう。
    let tick_threshold = setting.get_default_tick_threshold();
    let mut tick_timer = Timer::from_second(tick_threshold);

    // 終了条件は、すべてのノードが終わった時。
    // vvv オーディオレンダリングフレーム処理
    let mut node_queue = VecDeque::new();
    let mut elapsed_time = 0.0;
    loop {
        let prev_to_now_time = tick_timer.tick().as_secs_f64();
        elapsed_time += tick_timer.tick().as_secs_f64();

        // 24-12-12 依存システムの処理。
        if !(dependent_systems & system_category::AUDIO_DEVICE).is_zero() {
            AudioDevice::pre_process(prev_to_now_time);
        }

        // 共通で使う処理時の入力。
        let mut input = ProcessCommonInput {
            time_tick_mode: setting.time_tick_mode,
            elapsed_time,
            frame_time: prev_to_now_time,
            category: process_category::NORMAL,
            required_channel_samples: 0,
        };
        if setting.time_tick_mode == ETimeTickMode::Realtime && audio_device_weak_proxy.is_some() {
            input.required_channel_samples = {
                let audio_device = audio_device_weak_proxy.as_ref().unwrap().upgrade().unwrap();
                let mut proxy = audio_device.lock().unwrap();
                proxy.available_send_counts()
            } / setting.channels;
        }

        let mut end_node_processed = false;
        let mut is_all_finished = true;
        for items_group in &start_item_groups {
            // 同じ処理カテゴリの始発ノードを全部入れる。
            input.category = items_group.category;
            for item in &items_group.start_items {
                node_queue.push_back(item.clone().upgrade().unwrap());
            }

            //
            while !node_queue.is_empty() {
                // 処理する。
                let node = node_queue.pop_front().unwrap();
                let results = node.borrow_mut().process(&input);
                if !(results & process_result::DIFFERENT_CATEGORY).is_zero() {
                    continue;
                }

                // 次ノードをQueueに入れる。
                let next_nodes = node.borrow().get_next_nodes();
                let is_node_end = next_nodes.is_empty();
                for next_node in next_nodes.into_iter().filter(|v| v.borrow().can_process()) {
                    node_queue.push_back(next_node);
                }

                // もしnextがなければ、自分をfinalだとみなしてstartから自分までの処理が終わってるかを確認する。
                if is_node_end {
                    end_node_processed = true;
                    is_all_finished &= node.borrow().is_finished();
                }
            }
        }

        // 24-12-12 依存システムの処理。
        if !(dependent_systems & system_category::AUDIO_DEVICE).is_zero() {
            AudioDevice::post_process(prev_to_now_time);
        }

        if end_node_processed && is_all_finished {
            break;
        }
    }

    // 依存システムの解放
    if dependent_systems != system_category::NONE {
        // AudioDeviceの解放
        if !(dependent_systems & system_category::AUDIO_DEVICE).is_zero() {
            AudioDevice::cleanup();
        }
    }

    Ok(())
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
