use itertools::Itertools;
use crate::carg::v2::node::RelationTreeNodeWPtr;

/// ノードの依存システムのカテゴリビットフラグ
pub mod process_category {
    /// ノーマル
    pub const NORMAL: u64 = 1 << 0;

    /// デバイスアウトプットのノードだけ使用。
    /// 他のノードで使うのは禁止。
    pub const BUS_MASTER_OUTPUT: u64 = 1 << 63;
}

/// [`process_category`]のフラグ制御のための補助タイプ
pub type EProcessCategoryFlag = u64;

/// [`TProcess`]の処理の設定やメタ情報を記載するためのTrait。
pub trait TProcessCategory {
    /// ノードの処理をどの順に行うかを決める。
    ///
    /// 現在（24-12-12）は複数のフラグを入れて複数の処理グループで選択的に行わせるのは禁止。
    fn get_process_category() -> EProcessCategoryFlag {
        process_category::NORMAL
    }
}

/// ある処理順グループの開始にあたる処理アイテムのリストをまとめている。
#[derive(Debug)]
pub struct StartItemGroup {
    /// このグループの処理カテゴリ
    pub category: EProcessCategoryFlag,
    /// 開始アイテムのリスト
    pub start_items: Vec<RelationTreeNodeWPtr>,
}

impl StartItemGroup {
    ///
    pub fn initialize_groups_with(
        _orders: EProcessCategoryFlag,
        node_map: &[RelationTreeNodeWPtr],
    ) -> Vec<StartItemGroup> {
        // @MEMO 実装中気になったことだけど、あとで音源の再生によって動的にグラフを変更する必要があれば
        // その時にはどうすればいいか？

        let mut groups = vec![];
        for bit in 0..=63 {
            let target_category = 1 << bit;

            // まずnode_mapからターゲットに当てはまるノードがあるかを探す。
            // もしあったら、リスト化する。
            // なかったら無視して次のカテゴリを探す。
            let target_nodes = node_map
                .iter()
                .filter(|v| {
                    let v = v.upgrade().unwrap();
                    let x = v.borrow().category;
                    x == target_category
                })
                .map(|v| v.clone())
                .collect_vec();
            if target_nodes.is_empty() {
                continue;
            }

            let mut insert_nodes = vec![];
            for weak_node in target_nodes {
                // ~ならstartノードになれる。
                // 1. 本当の巨大のマップから連結している
                // 2. prevに自分と同じのカテゴリのノードがつながっていない。
                let node = weak_node.upgrade().unwrap();
                let borrowed = node.borrow();

                // つながっていること前提
                if !borrowed.is_connected {
                    continue;
                }

                // prevにないけどnextになにかつながっているか
                if borrowed.prev_nodes.is_empty() && !borrowed.next_nodes.is_empty() {
                    insert_nodes.push(weak_node.clone());
                    continue;
                }
                // それともprevに同じカテゴリのノードがなければOK。
                if !borrowed
                    .prev_nodes
                    .iter()
                    .any(|(_, v)| v.upgrade().unwrap().borrow().category == target_category)
                {
                    insert_nodes.push(weak_node.clone());
                    continue;
                }
            }

            if !insert_nodes.is_empty() {
                groups.push(StartItemGroup {
                    category: target_category,
                    start_items: insert_nodes,
                });
            }
        }

        groups
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------
