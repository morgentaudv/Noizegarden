use std::collections::HashMap;

use super::v2::{self};
use crate::carg::v2::meta;
use crate::carg::v2::meta::node::ENode;
use crate::carg::v2::meta::setting::Setting;
use crate::carg::v2::meta::system::SystemSetting;

/// @brief パーシングされたノードのコンテナ。
/// これだけで一連の処理ができる。
#[derive(Debug)]
pub enum ENodeContainer {
    None,
    V2 {
        setting: Setting,
        system_setting: SystemSetting,
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
                system_setting,
                nodes,
                relations,
            } => v2::process_v2(setting, system_setting, nodes.clone(), relations),
        }
    }
}

// ----------------------------------------------------------------------------
// EOF
// ----------------------------------------------------------------------------



