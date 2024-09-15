use std::collections::{HashSet, VecDeque};
use crate::carg::v2::MetaNodeContainer;
use crate::carg::v2::meta::relation::Relation;

/// 次のことを検査する。
///
/// * inputとoutputが空白なものがあるかを確認する。
/// * それぞれのノードに対してCycleになっていないかを確認する。
pub fn validate_node_relations(nodes: &MetaNodeContainer, relations: &[Relation]) -> anyhow::Result<()>
{
    let mut is_start_node_exist = false;

    for relation in relations {
        // inputとoutputが空白なものがあるかを確認する。
        if relation.prev.is_any_empty()
        {
            return Err(anyhow::anyhow!("input node is empty somewhat."));
        }
        if relation.next.is_any_empty()
        {
            return Err(anyhow::anyhow!("output node is empty somewhat."));
        }

        // まずrelationsからnodesに当てはまらないノード文字列があるかを確認する。
        // prev/next指定のノード情報が本当に有効かを確認。
        {
            let prev_node = &relation.prev;
            if !nodes.is_valid_prev_node_pin(prev_node)
            {
                return Err(anyhow::anyhow!("Given relation info ({:?}) is not exist in node map.", prev_node));
            }
            // 特殊ノードなのかも確認。
            if relation.prev.is_special_prev_node() { is_start_node_exist = true; }
        }

        // そしてinputとoutputの互換性を確認する。
        // 具体的にはoutputに対してinputの組み方とタイプを検証する。
        {
            let next_node = &relation.next;
            if !nodes.is_valid_next_node_pin(next_node)
            {
                return Err(anyhow::anyhow!("Given relation info ({:?}) is not exist in node map.", next_node));
            }
        }

        // そしてprev/nextがお互いに繋げられるかを確認。
        if !nodes.is_valid_relation(&relation)
        {
            return Err(anyhow::anyhow!(
                    "prev node ({:?}) does not support next node ({:?}).",
                    &relation.prev,
                    &relation.next
                ));
        }
    }

    if !is_start_node_exist
    {
        return Err(anyhow::anyhow!("There is no start pin node. '_start_pin'."));
    }

    // それぞれのノードに対してCycleになっていないかを確認する。
    // 一番簡単な方法？ではprevとして使っているノードだけを検査し、
    // ノードからの経路をチェックして2回目以上通ることがあればCycle判定にする。
    for (node_name, _) in &nodes.map {
        let mut name_queue: VecDeque<&String> = VecDeque::new();
        name_queue.push_back(node_name);

        let mut route_set: HashSet<GraphNodeRoute> = HashSet::new();

        while !name_queue.is_empty() {
            let search_name = name_queue.pop_front().unwrap();

            for relation in relations {
                // Prevか？
                if !(*relation.prev.node == *search_name) {
                    continue;
                }

                // prev-nextの経路を作って、今持っている経路リストにすでにあるかを確認する。
                // もしあれば、Cycleになっていると判断する。
                let route_item = GraphNodeRoute {
                    from: (*search_name).clone(),
                    to: relation.next.node.clone(),
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
