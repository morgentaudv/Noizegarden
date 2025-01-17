pub mod common;
pub mod pin;

use crate::carg::v2::meta::process::EProcessCategoryFlag;
use crate::carg::v2::meta::ENodeSpecifier;
use crate::carg::v2::node::pin::NodePinItemWPtr;
use crate::carg::v2::{ItemSPtr, ItemWPtr, ProcessCommonInput, ProcessProcessorInput, SItemSPtr, TProcessItemPtr};
use itertools::Itertools;
use std::collections::HashMap;
use std::rc::Rc;

/// [`RelationTreeNode::process`]の結果や後処理の指示を示す。
pub mod process_result {
    /// カテゴリが違うので処理しない。
    pub const DIFFERENT_CATEGORY: u64 = 1 << 0;

    /// 子ノードに処理を伝播する。
    pub const PROPAGATE_CHILDREN: u64 = 1 << 1;
}

pub type EProcessResultFlag = u64;

pub type RelationTreeNodePtr = ItemSPtr<RelationTreeNode>;

pub type RelationTreeNodeWPtr = ItemWPtr<RelationTreeNode>;

/// 木のノードアイテム。
#[derive(Debug)]
pub struct RelationTreeNode {
    /// ノードの名前
    pub name: String,
    /// このノードの処理カテゴリ
    pub category: EProcessCategoryFlag,
    /// デバッグ用かも
    #[allow(dead_code)]
    node_specifier: ENodeSpecifier,
    /// 処理グラフに連結されているか
    pub is_connected: bool,
    /// 前からこのノードを依存する前ノードのマップ
    pub(crate) prev_nodes: HashMap<String, RelationTreeNodeWPtr>,
    /// 次に伝播するノードのマップ
    pub(crate) next_nodes: HashMap<String, RelationTreeNodeWPtr>,
    /// このノードから処理するアイテム
    processor: TProcessItemPtr,
    /// フレームプロセスカウンター
    counter: ProcessCounter,
}

#[derive(Debug, Default, Clone)]
struct ProcessCounter {
    process_counter: usize,
}

impl RelationTreeNode {
    /// ノードアイテムを作る。
    pub fn new_item(name: &str, processor: TProcessItemPtr) -> RelationTreeNodePtr {
        let cached_category = processor.borrow().get_common_ref().get_process_category();
        let node_specifier = processor.borrow().get_common_ref().specifier;
        SItemSPtr::new(RelationTreeNode {
            name: name.to_owned(),
            category: cached_category,
            node_specifier,
            is_connected: false,
            prev_nodes: HashMap::new(),
            next_nodes: HashMap::new(),
            processor,
            counter: Default::default(),
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
    pub fn can_process(&self, input: &ProcessCommonInput) -> bool {
        let is_all_prev_processed = self
            .prev_nodes
            .iter()
            .all(|(_, v)| v.upgrade().unwrap().borrow().counter.process_counter >= input.process_counter);
        if !is_all_prev_processed {
            return false;
        }

        self.processor.borrow().can_process()
    }

    /// 処理する。
    pub fn process(&mut self, input: &ProcessCommonInput) -> EProcessResultFlag {
        // もしカテゴリが違っていれば、処理しない。
        if input.category != self.category {
            return process_result::DIFFERENT_CATEGORY;
        }

        //dbg!(&self.name);
        let input = ProcessProcessorInput {
            common: *input,
            children_states: self
                .prev_nodes
                .iter()
                .map(|(_, v)| v.upgrade().unwrap().borrow().is_finished())
                .collect_vec(),
        };

        self.processor.borrow_mut().try_process(&input);

        // 25-01-07 更新
        self.counter.process_counter = input.common.process_counter;

        process_result::PROPAGATE_CHILDREN
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
// EOF
// ----------------------------------------------------------------------------
