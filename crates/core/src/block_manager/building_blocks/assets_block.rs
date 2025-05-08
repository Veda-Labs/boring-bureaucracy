use super::building_block::BuildingBlock;
use crate::actions::action::Action;
use crate::block_manager::shared_cache::SharedCache;
use crate::utils::view_request_manager::ViewRequestManager;
use alloy::primitives::Address;
use async_trait::async_trait;
use eyre::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AssetsBlock {
    pub assets: Vec<Address>,
    #[serde(default)]
    pub teller: Option<Address>,
    #[serde(skip_deserializing, default)]
    pub no_read: String,
    // ...other fields
}

#[async_trait]
impl BuildingBlock for AssetsBlock {
    async fn assemble(&self, _vrm: &ViewRequestManager) -> Result<Vec<Box<dyn Action>>> {
        // TODO make RPC calls checking state of Teller/Accountant
        Ok(vec![])
    }

    // TODO maybe this should be share data instead? Then we pass in a mutable block manager?
    // Not sure some how we need to write data like BoringVault address back to the bm so it can share it
    // This also could just return a Json::Value of important addresses, then
    // The bm could maybe store this json data and other blocks can read from it?
    async fn resolve_state(
        &mut self,
        _cache: &SharedCache,
        _vrm: &ViewRequestManager,
    ) -> Result<()> {
        todo!()
    }
}
