use std::ops::{Deref, DerefMut};
use crate::carg::v2::EProcessOutput;
use crate::carg::v2::meta::ENodeSpecifier;
use crate::carg::v2::meta::input::EProcessInputContainer;
use crate::carg::v2::meta::process::EProcessCategoryFlag;
use crate::carg::v2::node::pin::{NodePinItem, NodePinItemList, NodePinItemWPtr};

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
                assert_eq!(borrowed.is_output, false);
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

    /// 処理順のカテゴリを返す。
    pub fn get_process_category(&self) -> EProcessCategoryFlag {
        self.specifier.get_process_category()
    }

    /// ステートが一体しているか
    pub fn is_state(&self, state: EProcessState) -> bool {
        self.state == state
    }

    /// ステートを更新する。
    pub fn set_state(&mut self, state: EProcessState) {
        self.state = state;
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
// EOF
// ----------------------------------------------------------------------------
