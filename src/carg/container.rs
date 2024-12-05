use std::collections::HashMap;

use super::v2::{self};
use crate::carg::v2::meta;
use crate::carg::v2::meta::node::ENode;
use crate::carg::v2::meta::setting::Setting;
use crate::wave::sample::UniformedSample;

/// @brief パーシングされたノードのコンテナ。
/// これだけで一連の処理ができる。
#[derive(Debug)]
pub enum ENodeContainer {
    None,
    V2 {
        setting: Setting,
        nodes: HashMap<String, ENode>,
        relations: Vec<meta::relation::Relation>,
    },
}

unsafe impl Sync for ENodeContainer {}
unsafe impl Send for ENodeContainer {}

impl ENodeContainer {
    pub fn process(&self) -> anyhow::Result<()> {
        match self {
            ENodeContainer::None => Ok(()),
            ENodeContainer::V2 {
                setting,
                nodes,
                relations,
            } => v2::process_v2(setting, nodes.clone(), relations),
        }
    }
}

struct UniformedSampleBufferItem {
    buffer: Vec<UniformedSample>,
    start_index: usize,
    length: usize,
}


