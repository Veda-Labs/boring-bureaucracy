use super::building_block::Actionable;
use crate::actions::admin_action::AdminAction;
use crate::utils::address_or_contract_name::AddressOrContractName;
use alloy::primitives::Address;
use eyre::Result;
use serde::Deserialize;
use serde_json::{Map, Value, json};

#[derive(Debug, Deserialize)]
pub struct GlobalBlock {
    #[serde(default)]
    pub deployer: Option<Address>,
    #[serde(default)]
    pub network_id: Option<u32>,
    #[serde(default)]
    pub boring_vault: Option<AddressOrContractName>,
    #[serde(default)]
    pub teller: Option<AddressOrContractName>,
    #[serde(default)]
    pub accountant: Option<AddressOrContractName>,
    #[serde(default)]
    pub manager: Option<AddressOrContractName>,
    // #[serde(skip_deserializing, default)]
    // pub no_read: String,
    // ...other fields
}

impl Actionable for GlobalBlock {
    fn to_actions(&self) -> Result<Vec<Box<dyn AdminAction>>> {
        Ok(vec![])
    }

    fn resolve_and_contribute(&mut self, _cache: &Value) -> Result<Value> {
        // Globals does not read anything from cache, only writes to it.
        let mut resolved = Map::new();
        if let Some(boring_vault) = &self.boring_vault {
            match boring_vault {
                AddressOrContractName::Address(addr) => {
                    resolved.insert("boring_vault".to_string(), Value::String(addr.to_string()));
                }
                AddressOrContractName::ContractName(name) => {
                    todo!("Query boring_vault from deployer.")
                }
            }
        }
        // Resolve
        // Ask block manager for missing values, I think this should be
        let data = json!({"boring_vault": Address::ZERO});
        Ok(data)
    }
}
