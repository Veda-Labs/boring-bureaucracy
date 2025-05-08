use super::building_block::BuildingBlock;
use crate::actions::action::Action;
use crate::block_manager::shared_cache::SharedCache;
use crate::utils::view_request_manager::ViewRequestManager;
use alloy::primitives::Address;
use async_trait::async_trait;
use eyre::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TellerBlock {
    pub teller: Address,
    // ...other fields
}

#[async_trait]
impl BuildingBlock for TellerBlock {
    async fn assemble(&self, _vrm: &ViewRequestManager) -> Result<Vec<Box<dyn Action>>> {
        Ok(vec![])
    }

    async fn resolve_state(
        &mut self,
        _cache: &SharedCache,
        _vrm: &ViewRequestManager,
    ) -> Result<()> {
        todo!()
    }
}
