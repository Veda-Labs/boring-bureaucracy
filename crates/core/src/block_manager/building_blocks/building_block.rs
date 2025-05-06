use super::{assets_block::AssetsBlock, teller_block::TellerBlock};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BuildingBlock {
    #[serde(rename = "assets")]
    Assets(AssetsBlock),
    #[serde(rename = "teller")]
    Teller(TellerBlock),
    // ...add more as needed
}
