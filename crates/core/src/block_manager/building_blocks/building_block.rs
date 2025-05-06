use super::{assets_block::AssetsBlock, teller_block::TellerBlock};
use crate::actions::admin_action::AdminAction;
use eyre::Result;
use into_trait::IntoTraitObject;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub trait Actionable {
    fn to_actions(&self) -> Result<Vec<Box<dyn AdminAction>>>;
    fn resolve_and_contribute(&mut self, cache: &Value) -> Result<Value>;
}

#[derive(Deserialize, Debug, IntoTraitObject)]
#[trait_name(Actionable)]
pub enum BuildingBlock {
    Assets(AssetsBlock),
    Teller(TellerBlock),
    // ...add more as needed
}
