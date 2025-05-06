use super::building_block::Actionable;
use crate::actions::admin_action::AdminAction;
use crate::block_manager::block_manager::BlockManager;
use alloy::primitives::Address;
use eyre::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Deserialize)]
pub struct AssetsBlock {
    pub assets: Vec<Address>,
    #[serde(default)]
    pub teller: Option<Address>,
    #[serde(skip_deserializing, default)]
    pub no_read: String,
    // ...other fields
}

impl Actionable for AssetsBlock {
    fn to_actions(&self) -> Result<Vec<Box<dyn AdminAction>>> {
        Ok(vec![])
    }

    // TODO maybe this should be share data instead? Then we pass in a mutable block manager?
    // Not sure some how we need to write data like BoringVault address back to the bm so it can share it
    // This also could just return a Json::Value of important addresses, then
    // The bm could maybe store this json data and other blocks can read from it?
    fn resolve_and_contribute(&mut self, cache: &Value) -> Result<Value> {
        // Ask block manager for missing values, I think this should be
        let data = json!({"boring_vault": Address::ZERO});
        Ok(data)
    }
}
