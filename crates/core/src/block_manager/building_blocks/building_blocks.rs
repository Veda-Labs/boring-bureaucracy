use super::{
    assets_block::AssetsBlock, boring_vault_block::BoringVaultBlock, building_block::BuildingBlock,
    global_block::GlobalBlock, teller_block::TellerBlock,
};
use into_trait::IntoTraitObject;
use serde::Deserialize;

// TODO I guess you could do "meta building blocks" who do not create actions, rather they only
// add more values to the cache? So in this case Global Block is a meta building block
// TODO guess I could make a Timelock BuildingBlock that would just deploy a new timelock? Or I guess configure an existing one maybe?
#[derive(Deserialize, Debug, IntoTraitObject)]
#[trait_name(BuildingBlock)]
pub enum BuildingBlocks {
    Global(GlobalBlock),
    BoringVault(BoringVaultBlock),
    Assets(AssetsBlock),
    Teller(TellerBlock),
    // ...add more as needed
}
