use super::container::ENodeContainer;
use crate::carg::v2::meta::input::{EInputContainerCategoryFlag, EProcessInputContainer};
use crate::carg::v2::meta::node::{ENode, MetaNodeContainer};
use crate::carg::v2::meta::output::EProcessOutputContainer;
use crate::carg::v2::meta::{pin_category, ENodeSpecifier, EPinCategoryFlag};
use crate::carg::v2::utility::validate_node_relations;
use crate::wave::analyze::sine_freq::SineFrequency;
use crate::{
    math::timer::Timer,
    wave::sample::UniformedSample,
};
use itertools::Itertools;
use meta::relation::Relation;
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};
use std::rc::Weak;
use std::{
    cell::RefCell,
    collections::{HashMap, VecDeque},
    rc::Rc,
};
use crate::carg::v2::meta::setting::{ETimeTickMode, Setting};

pub mod adapter;
pub mod analyzer;
pub mod base;
pub mod emitter;
pub mod meta;
pub mod output;
mod special;
mod utility;
pub mod mix;

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
// RelationTreeNode
// ----------------------------------------------------------------------------

pub type RelationTreeNodePtr = ItemSPtr<RelationTreeNode>;

pub type RelationTreeNodeWPtr = ItemWPtr<RelationTreeNode>;

/// 木のノードアイテム。
#[derive(Debug)]
pub struct RelationTreeNode {
    /// ノードの名前
    pub name: String,
    /// 前からこのノードを依存する前ノードのマップ
    prev_nodes: HashMap<String, RelationTreeNodeWPtr>,
    /// 次に伝播するノードのマップ
    next_nodes: HashMap<String, RelationTreeNodeWPtr>,
    /// このノードから処理するアイテム
    processor: TProcessItemPtr,
}

impl RelationTreeNode {
    /// ノードアイテムを作る。
    pub fn new_item(name: &str, processor: TProcessItemPtr) -> RelationTreeNodePtr {
        SItemSPtr::new(RelationTreeNode {
            name: name.to_owned(),
            prev_nodes: HashMap::new(),
            next_nodes: HashMap::new(),
            processor,
        })
    }

    /// 前からこのノードを依存する前ノードを登録する。
    pub fn append_prev_node(&mut self, node_name: String, prev: RelationTreeNodePtr) {
        self.prev_nodes.insert(node_name, Rc::downgrade(&prev));
    }

    /// 次に処理を伝播するノードを登録する。
    pub fn append_next_node(&mut self, node_name: String, next: RelationTreeNodePtr) {
        self.next_nodes.insert(node_name, Rc::downgrade(&next));
    }

    ///
    pub fn link_pin_output_to_input(&mut self, input_pin: &str, output_pin: NodePinItemWPtr) {
        self.processor
            .borrow_mut()
            .get_common_mut()
            .link_pin_output_to_input(input_pin, output_pin);
    }

    pub fn link_pin_input_to_output(&mut self, output_pin: &str, input_pin: NodePinItemWPtr) {
        self.processor
            .borrow_mut()
            .get_common_mut()
            .link_pin_input_to_output(output_pin, input_pin);
    }

    /// ノードの識別子を返す。
    pub fn get_specifier(&self) -> ENodeSpecifier {
        self.processor.borrow().get_common_ref().specifier
    }

    /// `pin_name`のOutputピンが存在する場合、そのピンのWeakPtrを返す。
    pub fn get_output_pin(&self, pin_name: &str) -> Option<NodePinItemWPtr> {
        self.processor.borrow().get_common_ref().get_output_pin(pin_name)
    }

    /// `pin_name`のInputピンが存在する場合、そのピンのWeakPtrを返す。
    pub fn get_input_pin(&self, pin_name: &str) -> Option<NodePinItemWPtr> {
        self.processor.borrow().get_common_ref().get_input_pin(pin_name)
    }

    /// ノード自分の処理が可能か？
    /// たとえば、インプットが全部更新されている状態だったりとか。
    pub fn can_process(&self) -> bool {
        self.processor.borrow().can_process()
    }

    pub fn process(&mut self, input: &ProcessCommonInput) {
        let input = ProcessProcessorInput {
            common: *input,
            children_states: self
                .prev_nodes
                .iter()
                .map(|(_, v)| v.upgrade().unwrap().borrow().is_finished())
                .collect_vec(),
        };
        self.processor.borrow_mut().try_process(&input);
    }

    /// 次につながっているノードのリストを返す。
    pub fn get_next_nodes(&self) -> Vec<RelationTreeNodePtr> {
        self.next_nodes.iter().filter_map(|(_, v)| v.upgrade()).collect_vec()
    }

    /// ノードの処理活動が終わったか？
    pub fn is_finished(&self) -> bool {
        self.processor.borrow().is_finished()
    }
}

// ----------------------------------------------------------------------------
// ProcessControlItem
// ----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct NodePinItem {
    /// ピンの名前
    name: String,
    /// ピンのカテゴリフラグ（複数可）
    categories: EPinCategoryFlag,
    /// Output用ピンなのか？
    is_output: bool,
    /// このノードが最後に更新された時間
    elapsed_time: f64,
    /// アップデートがリクエストされている状態か
    is_update_requested: bool,
    /// 連結しているピンのリスト
    linked_pins: Vec<NodePinItemWPtr>,
    /// Inputのコンテナ
    input: EProcessInputContainer,
    /// [`EProcessInputContainer`]の指定カテゴリフラグ
    input_flag: EInputContainerCategoryFlag,
    /// Outputのコンテナ
    output: EProcessOutputContainer,
}

pub type NodePinItemSPtr = ItemSPtr<NodePinItem>;
pub type NodePinItemWPtr = ItemWPtr<NodePinItem>;

pub type NodePinItemList = HashMap<String, NodePinItemSPtr>;

impl NodePinItem {
    /// 新規アイテムの生成。
    pub fn new_item(
        name: &str,
        categories: EPinCategoryFlag,
        is_output: bool,
        input_flag: EInputContainerCategoryFlag,
    ) -> NodePinItemSPtr {
        SItemSPtr::new(Self {
            name: name.to_owned(),
            categories,
            is_output,
            elapsed_time: 0.0,
            linked_pins: vec![],
            is_update_requested: false,
            input: EProcessInputContainer::Uninitialized,
            input_flag,
            output: EProcessOutputContainer::Empty,
        })
    }

    pub fn downgrade(item: &NodePinItemSPtr) -> NodePinItemWPtr {
        Rc::downgrade(item)
    }

    pub fn insert_to_output(&mut self, new_output: EProcessOutput) -> anyhow::Result<()> {
        // カテゴリを見て`new_output`がサポートできない種類であればエラーを返す。
        if !new_output.check(self.categories) {
            return Err(anyhow::anyhow!(
                "Not supported output category of ({} pin, {} flags).",
                self.name,
                self.categories
            ));
        }

        // もし現在のOutputコンテナとカテゴリが違ったら、作り治す。
        if self.output.as_pin_category_flag() != new_output.as_pin_category_flag() {
            self.output.reset_with(new_output);
            self.notify_update_to_next_pins();
            return Ok(());
        }

        // 既存コンテナに入れる。
        self.output.insert_with(new_output).expect("Failed to insert output");
        self.notify_update_to_next_pins();
        Ok(())
    }

    /// 繋がっているピンにアップデート通知を送る。
    pub fn notify_update_to_next_pins(&mut self) {
        for linked_pin in &mut self.linked_pins {
            // Upgradeしてフラグを更新する。
            if let Some(pin) = linked_pin.upgrade() {
                pin.borrow_mut().is_update_requested = true;
            }
        }
    }

    /// Input処理を行う。
    pub fn process_input(&mut self) {
        assert_eq!(self.is_output, false);

        // もしUninitializedなら、初期化する。
        if !self.input.is_initialized() {
            // 初期化する。
            self.input.initialize(self.input_flag);
        }

        // output側から情報を処理する。
        // ただしinputではlinked_pinsの数は1個までで、Emptyなことはないと。
        assert_eq!(self.linked_pins.len(), 1);
        let output_pin = self.linked_pins.first().unwrap().upgrade().unwrap();
        let output = &output_pin.borrow().output;
        self.input.process(output);
    }
}

#[derive(Debug, Clone)]
pub struct ProcessControlItem {
    /// アイテムの状態を表す。
    pub state: EProcessState,
    /// 状態からの細部制御ルーティン番号
    pub state_rtn: [u64; 4],
    /// アイテムの識別子タイプ
    pub specifier: ENodeSpecifier,
    /// 経過した時間（秒単位）
    pub elapsed_time: f64,
    /// Input用ピンのリスト（ノードに入る側）
    pub input_pins: NodePinItemList,
    /// Output用ピンのリスト（ノード側出る側）
    pub output_pins: NodePinItemList,
}

impl ProcessControlItem {
    pub fn new(specifier: ENodeSpecifier) -> Self {
        Self {
            state: EProcessState::Stopped,
            state_rtn: [0; 4],
            specifier,
            elapsed_time: 0.0,
            input_pins: specifier.create_input_pins(),
            output_pins: specifier.create_output_pins(),
        }
    }

    /// `pin_name`のOutputピンが存在する場合、そのピンのWeakPtrを返す。
    pub fn get_output_pin(&self, pin_name: &str) -> Option<NodePinItemWPtr> {
        match self.output_pins.get(pin_name) {
            None => None,
            Some(v) => Some(NodePinItem::downgrade(v)),
        }
    }

    /// `pin_name`のInputピンが存在する場合、そのピンのWeakPtrを返す。
    pub fn get_input_pin(&self, pin_name: &str) -> Option<NodePinItemWPtr> {
        match self.input_pins.get(pin_name) {
            None => None,
            Some(v) => Some(NodePinItem::downgrade(v)),
        }
    }

    /// `input_pin`名前のInputピンが存在すれば、`output_pin`をそのピンのリンク先としてリストに入れる。
    pub fn link_pin_output_to_input(&mut self, input_pin: &str, output_pin: NodePinItemWPtr) {
        if let Some(v) = self.get_input_pin(input_pin) {
            v.upgrade().unwrap().borrow_mut().linked_pins.push(output_pin);
        }
    }

    /// `output_pin`名前のOutputピンが存在すれば、`input_pin`をそのピンのリンク先としてリストに入れる。
    pub fn link_pin_input_to_output(&mut self, output_pin: &str, input_pin: NodePinItemWPtr) {
        if let Some(v) = self.get_output_pin(output_pin) {
            v.upgrade().unwrap().borrow_mut().linked_pins.push(input_pin);
        }
    }

    /// Outputピンが繋がっているすべてのInputピンに対し更新要請があるかを確認する。
    pub fn is_all_input_pins_update_notified(&self) -> bool {
        if self.input_pins.is_empty() {
            return true;
        }

        self.input_pins
            .iter()
            .filter(|(_, v)| v.borrow().linked_pins.len() > 0)
            .all(|(_, v)| v.borrow().is_update_requested)
    }

    /// Updateフラグが立っているすべてのInputピンを更新する。
    pub fn process_input_pins(&mut self) {
        //
        for (_, pin) in &mut self.input_pins {
            let mut borrowed = pin.borrow_mut();
            if borrowed.is_update_requested {
                // 何をやるかはちょっと考える…
                assert!(borrowed.is_output == false);
                borrowed.process_input();
            }
        }

        // フラグを全部リセット
        self.reset_all_input_pins_update_flag();
    }

    /// すべてのInputピンの更新フラグをリセットする。
    pub fn reset_all_input_pins_update_flag(&mut self) {
        if self.input_pins.is_empty() {
            return;
        }

        self.input_pins.iter_mut().for_each(|(_, v)| {
            v.borrow_mut().is_update_requested = false;
        });
    }

    /// `new_output`を`pin_name`のoutputピンに入れる。
    pub fn insert_to_output_pin(&mut self, pin_name: &str, new_output: EProcessOutput) -> anyhow::Result<()> {
        match self.output_pins.get_mut(pin_name) {
            None => Err(anyhow::anyhow!("Failed to find output pin `{}`.", pin_name)),
            Some(v) => v.borrow_mut().insert_to_output(new_output),
        }
    }

    pub fn get_input_internal(&self, pin_name: &str) -> Option<InputInternal> {
        let borrowed = self.input_pins.get(pin_name)?.borrow();
        Some(InputInternal { borrowed })
    }

    pub fn get_input_internal_mut(&mut self, pin_name: &str) -> Option<InputInternalMut> {
        let borrowed = self.input_pins.get(pin_name)?.borrow_mut();
        Some(InputInternalMut { borrowed })
    }

    /// `pin_name`のOutputピンが他のノードのピンに繋がっているかを確認。
    pub fn is_output_pin_connected(&self, pin_name: &str) -> bool {
        match self.output_pins.get(pin_name) {
            None => false,
            Some(v) => v.borrow().linked_pins.is_empty() == false,
        }
    }
}

/// [`ProcessControlItem::get_input_internal`]関数からの構造体。
pub struct InputInternal<'a> {
    borrowed: std::cell::Ref<'a, NodePinItem>,
}

impl Deref for InputInternal<'_> {
    type Target = EProcessInputContainer;

    fn deref(&self) -> &Self::Target {
        &self.borrowed.input
    }
}

/// [`ProcessControlItem::get_input_internal_mut`]関数からの構造体。
pub struct InputInternalMut<'a> {
    borrowed: std::cell::RefMut<'a, NodePinItem>,
}

impl Deref for InputInternalMut<'_> {
    type Target = EProcessInputContainer;

    fn deref(&self) -> &Self::Target {
        &self.borrowed.input
    }
}

impl DerefMut for InputInternalMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.borrowed.input
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

/// [`TProcess`]を実装しているアイテムの外部表示タイプ
pub type TProcessItemPtr = ItemSPtr<dyn TProcess>;

pub fn process_v2(setting: &Setting, nodes: HashMap<String, ENode>, relations: &[Relation]) -> anyhow::Result<()> {
    // 下で`_start_pin`のチェックもやってくれる。
    let node_container = MetaNodeContainer { map: nodes };
    validate_node_relations(&node_container, &relations)?;

    // チェックができたので(validation)、relationを元にGraphを生成する。
    // ただしそれぞれの独立したoutputをルートにして必要となるinputを子としてツリーを構成する。
    let node_map = {
        let mut map = HashMap::new();

        // 各ノードから処理に使うためのアイテムを全部生成しておく。
        // 中でinputピンとoutputピンを作る。
        for (node_name, node) in &node_container.map {
            let processor = node.create_from(setting);
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

    // そしてcontrol_itemsとnodes、output_treeを使って処理をする。+ setting.
    // VecDequeをStackのように扱って、DFSをコールスタックを使わずに実装することができそう。
    let start_node = node_map.get("_start_pin").unwrap().clone();
    let tick_threshold = setting.get_default_tick_threshold();
    let mut tick_timer = Timer::from_second(tick_threshold);

    // 終了条件は、すべてのノードが終わった時。
    let mut node_queue = VecDeque::new();
    let mut elapsed_time = 0.0;
    loop {
        let prev_to_now_time = tick_timer.tick().as_secs_f64();
        //elapsed_time += tick_timer.tick().as_secs_f64();
        elapsed_time += 5.0 / 1000.0;

        // 共通で使う処理時の入力。
        let input = ProcessCommonInput {
            time_tick_mode: setting.time_tick_mode,
            elapsed_time,
            frame_time: prev_to_now_time,
        };
        node_queue.push_back(start_node.clone());

        //
        let mut end_node_processed = false;
        let mut is_all_finished = true;
        while !node_queue.is_empty() {
            // 処理する。
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
                end_node_processed = true;
                is_all_finished &= process_node.borrow().is_finished();
            }
        }

        println!("{:?}s", prev_to_now_time);

        if end_node_processed && is_all_finished {
            break;
        }
    }

    println!("{:?}s", elapsed_time);

    Ok(())
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
