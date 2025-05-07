use super::{
    assets_block::AssetsBlock, boring_vault_block::BoringVaultBlock, global_block::GlobalBlock,
    teller_block::TellerBlock,
};
use crate::actions::admin_action::AdminAction;
use crate::block_manager::shared_cache::SharedCache;
use crate::utils::view_request_manager::ViewRequestManager;
use async_trait::async_trait;
use eyre::Result;
use into_trait::IntoTraitObject;
use serde::Deserialize;

#[async_trait]
pub trait Actionable: Send + Sync {
    async fn to_actions(&self, vrm: &ViewRequestManager) -> Result<Vec<Box<dyn AdminAction>>>;
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
    BoringVault(BoringVaultBlock),
    Assets(AssetsBlock),
    Teller(TellerBlock),
    // ...add more as needed
}
