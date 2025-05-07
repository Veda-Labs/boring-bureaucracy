use super::building_block::Actionable;
use crate::actions::admin_action::AdminAction;
use crate::utils::address_or_contract_name::AddressOrContractName;
use alloy::primitives::Address;
use async_trait::async_trait;
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
    pub roles_authority: Option<AddressOrContractName>,
    #[serde(default)]
    pub teller: Option<AddressOrContractName>,
    #[serde(default)]
    pub accountant: Option<AddressOrContractName>,
    #[serde(default)]
    pub manager: Option<AddressOrContractName>,
    // #[serde(skip_deserializing, default)]
    // pub no_read: String,
    // ...other fields
    // TODO multisig address(owner of roles auth)
    // TODO timelock address(owner of roles auth)
}

#[async_trait]
impl Actionable for GlobalBlock {
    async fn to_actions(&self) -> Result<Vec<Box<dyn AdminAction>>> {
        Ok(vec![])
    }

    // TODO this needs to be refactored like the boring vault block!
    async fn resolve_and_contribute(&mut self, _cache: &Value) -> Result<Value> {
        // Globals does not read anything from cache, only writes to it.
        let mut resolved = Map::new();

        if let Some(deployer) = &self.deployer {
            resolved.insert("deployer".to_string(), Value::String(deployer.to_string()));
        }

        if let Some(network_id) = self.network_id {
            resolved.insert("network_id".to_string(), Value::Number(network_id.into()));
        }

        if let Some(boring_vault) = &self.boring_vault {
            match boring_vault {
                AddressOrContractName::Address(addr) => {
                    resolved.insert("boring_vault".to_string(), Value::String(addr.to_string()));
                }
                AddressOrContractName::ContractName(_name) => {
                    todo!("Query boring_vault from deployer.")
                }
            }
        }

        // TODO below addresses can be derived from previous addresses, assuming boring_vault is set.
        // So add an else statement
        if let Some(roles_authority) = &self.roles_authority {
            match roles_authority {
                AddressOrContractName::Address(addr) => {
                    resolved.insert(
                        "roles_authority".to_string(),
                        Value::String(addr.to_string()),
                    );
                }
                AddressOrContractName::ContractName(_name) => {
                    todo!("Query roles_authority from deployer.")
                    // TODO if this created actions, then it should also save the value into the struct.
                }
            }
        }

        if let Some(teller) = &self.teller {
            match teller {
                AddressOrContractName::Address(addr) => {
                    resolved.insert("teller".to_string(), Value::String(addr.to_string()));
                }
                AddressOrContractName::ContractName(_name) => {
                    todo!("Query teller from deployer.")
                }
            }
        }

        if let Some(accountant) = &self.accountant {
            match accountant {
                AddressOrContractName::Address(addr) => {
                    resolved.insert("accountant".to_string(), Value::String(addr.to_string()));
                }
                AddressOrContractName::ContractName(_name) => {
                    todo!("Query accountant from deployer.")
                }
            }
        }

        if let Some(manager) = &self.manager {
            match manager {
                AddressOrContractName::Address(addr) => {
                    resolved.insert("manager".to_string(), Value::String(addr.to_string()));
                }
                AddressOrContractName::ContractName(_name) => {
                    todo!("Query manager from deployer.")
                }
            }
        }
        Ok(Value::Object(resolved))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_global_block_from_json_and_resolve() {
        // Example JSON with all fields as addresses
        let json_data = json!({
            "deployer": "0x0000000000000000000000000000000000000001",
            "network_id": 42,
            "boring_vault": "0x0000000000000000000000000000000000000002",
            "roles_authority": "0x0000000000000000000000000000000000000003",
            "teller": "0x0000000000000000000000000000000000000004",
            "accountant": "0x0000000000000000000000000000000000000005",
            "manager": "0x0000000000000000000000000000000000000006"
        });

        // Deserialize
        let mut block: GlobalBlock = serde_json::from_value(json_data).unwrap();

        // Call resolve_and_contribute
        let resolved = block.resolve_and_contribute(&Value::Null).await.unwrap();
        let obj = resolved.as_object().unwrap();

        assert_eq!(
            obj.get("deployer").unwrap(),
            "0x0000000000000000000000000000000000000001"
        );
        assert_eq!(obj.get("network_id").unwrap(), 42);
        assert_eq!(
            obj.get("boring_vault").unwrap(),
            "0x0000000000000000000000000000000000000002"
        );
        assert_eq!(
            obj.get("roles_authority").unwrap(),
            "0x0000000000000000000000000000000000000003"
        );
        assert_eq!(
            obj.get("teller").unwrap(),
            "0x0000000000000000000000000000000000000004"
        );
        assert_eq!(
            obj.get("accountant").unwrap(),
            "0x0000000000000000000000000000000000000005"
        );
        assert_eq!(
            obj.get("manager").unwrap(),
            "0x0000000000000000000000000000000000000006"
        );

        // Call to_actions and check it returns nothing
        let actions = block.to_actions().await.unwrap();
        assert!(actions.is_empty());
    }

    #[tokio::test]
    async fn test_global_block_with_missing_fields() {
        // Only network_id and deployer
        let json_data = json!({
            "deployer": "0x0000000000000000000000000000000000000001",
            "network_id": 1
        });

        let mut block: GlobalBlock = serde_json::from_value(json_data).unwrap();
        let resolved = block.resolve_and_contribute(&Value::Null).await.unwrap();
        let obj = resolved.as_object().unwrap();

        assert_eq!(
            obj.get("deployer").unwrap(),
            "0x0000000000000000000000000000000000000001"
        );
        assert_eq!(obj.get("network_id").unwrap(), 1);
        assert!(obj.get("boring_vault").is_none());
        assert!(obj.get("roles_authority").is_none());
        assert!(obj.get("teller").is_none());
        assert!(obj.get("accountant").is_none());
        assert!(obj.get("manager").is_none());

        let actions = block.to_actions().await.unwrap();
        assert!(actions.is_empty());
    }
}
