use serde::{Deserialize, Serialize};

/// [`Relation`]の各ノードのピン情報を保持する。
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RelationItemPin
{
    /// ノード名。
    pub node: String,
    /// ノードのピン名。
    pub pin: String,
}

impl RelationItemPin
{
    /// どれかが指定されずの状態か？
    /// これがfalseが返って来たとしても有効なわけではないことに要注意。
    pub fn is_any_empty(&self) -> bool { self.node.is_empty() || self.pin.is_empty() }

    /// 特殊なインプットノード名なのかを確認する。
    pub fn is_special_prev_node(&self) -> bool
    {
        if self.node == "_start_pin" {
            return true;
        }

        false
    }
}

/// [`ENode`]間の関係性を記述する。
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Relation
{
    /// 出力側
    pub prev: RelationItemPin,
    /// 入力側
    pub next: RelationItemPin,
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------

