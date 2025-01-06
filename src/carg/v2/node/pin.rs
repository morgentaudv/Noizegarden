use std::collections::HashMap;
use std::rc::Rc;
use crate::carg::v2::{EProcessOutput, ItemSPtr, ItemWPtr, SItemSPtr};
use crate::carg::v2::meta::EPinCategoryFlag;
use crate::carg::v2::meta::input::{EInputContainerCategoryFlag, EProcessInputContainer};
use crate::carg::v2::meta::output::EProcessOutputContainer;

#[derive(Debug, Clone)]
pub struct NodePinItem {
    /// ピンの名前
    name: String,
    /// ピンのカテゴリフラグ（複数可）
    categories: EPinCategoryFlag,
    /// Output用ピンなのか？
    pub(super) is_output: bool,
    /// このノードが最後に更新された時間
    pub(crate) elapsed_time: f64,
    /// アップデートがリクエストされている状態か
    pub(super) is_update_requested: bool,
    /// 連結しているピンのリスト
    pub(crate) linked_pins: Vec<NodePinItemWPtr>,
    /// Inputのコンテナ
    pub(crate) input: EProcessInputContainer,
    /// [`EProcessInputContainer`]の指定カテゴリフラグ
    input_flag: EInputContainerCategoryFlag,
    /// Outputのコンテナ
    pub(crate) output: EProcessOutputContainer,
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

    pub fn try_initialize(&mut self) {
        // もしUninitializedなら、初期化する。
        if !self.input.is_initialized() {
            // 初期化する。
            self.input.initialize(self.input_flag);
        }
    }

    /// Input処理を行う。
    pub fn process_input(&mut self) {
        assert_eq!(self.is_output, false);

        // output側から情報を処理する。
        // ただしinputではlinked_pinsの数は1個までで、Emptyなことはないと。
        assert_eq!(self.linked_pins.len(), 1);
        let output_pin = self.linked_pins.first().unwrap().upgrade().unwrap();
        let output = &output_pin.borrow().output;
        self.input.process(output);
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
