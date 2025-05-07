use super::building_block::Actionable;
use crate::actions::admin_action::AdminAction;
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
    async fn to_actions(&self) -> Result<Vec<Box<dyn AdminAction>>> {
        Ok(vec![])
    }

    async fn resolve_and_contribute(
        &mut self,
        cache: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let data = json!({"boring_vault": Address::ZERO});
        Ok(data)
    }
}
