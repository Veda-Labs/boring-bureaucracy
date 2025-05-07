use super::building_block::Actionable;
use crate::actions::admin_action::AdminAction;
use crate::block_manager::shared_cache::SharedCache;
use crate::utils::view_request_manager::ViewRequestManager;
use alloy::primitives::Address;
use async_trait::async_trait;
use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Deserialize)]
pub struct TellerBlock {
    pub teller: Address,
    // ...other fields
}

#[async_trait]
impl Actionable for TellerBlock {
    async fn to_actions(&self, vrm: &ViewRequestManager,) -> Result<Vec<Box<dyn AdminAction>>> {
        Ok(vec![])
    }

    async fn resolve_and_contribute(
        &mut self,
        cache: &SharedCache,
        vrm: &ViewRequestManager,
    ) -> Result<()> {
        todo!()
    }
}
