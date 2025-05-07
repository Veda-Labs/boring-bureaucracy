use super::{assets_block::AssetsBlock, global_block::GlobalBlock, teller_block::TellerBlock};
use crate::actions::admin_action::AdminAction;
use crate::block_manager::shared_cache::SharedCache;
use crate::utils::address_or_contract_name::AddressOrContractName;
use crate::utils::view_request_manager::ViewRequestManager;
use alloy::primitives::Address;
use async_trait::async_trait;
use eyre::{Result, eyre};
use into_trait::IntoTraitObject;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fmt;

#[async_trait]
pub trait Actionable: Send + Sync {
    async fn to_actions(&self) -> Result<Vec<Box<dyn AdminAction>>>;
    async fn resolve_and_contribute(
        &mut self,
        cache: &SharedCache,
        vrm: &ViewRequestManager,
    ) -> Result<()>;
}

#[derive(Deserialize, Debug, IntoTraitObject)]
#[trait_name(Actionable)]
pub enum BuildingBlock {
    Global(GlobalBlock),
    Assets(AssetsBlock),
    Teller(TellerBlock),
    // ...add more as needed
}
