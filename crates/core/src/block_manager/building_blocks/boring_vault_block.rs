use super::building_block::{Actionable, MissingCacheValuesError, handle_cache_field_generic};
use crate::actions::admin_action::AdminAction;
use crate::block_manager::block_manager::BlockManager;
use crate::utils::address_or_contract_name::AddressOrContractName;
use alloy::primitives::Address;
use async_trait::async_trait;
use eyre::{Result, eyre};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

#[derive(Debug, Deserialize)]
pub struct BoringVaultBlock {
    #[serde(default)]
    pub deployer: Option<Address>,
    #[serde(default)]
    pub boring_vault: Option<AddressOrContractName>,
    #[serde(default)]
    pub boring_vault_name: Option<String>,
    #[serde(default)]
    pub boring_vault_symbol: Option<String>,
    #[serde(default)]
    pub boring_vault_decimals: Option<u8>,
    // TODO Hook address, Manager, Teller so we assign the Roles needed.
    // TODO roles authority
}

#[async_trait]
impl Actionable for BoringVaultBlock {
    async fn to_actions(&self) -> Result<Vec<Box<dyn AdminAction>>> {
        // TODO make RPC calls checking if boring vault is deployed, if not add deploy action
        Ok(vec![])
    }

    async fn resolve_and_contribute(&mut self, cache: &Value) -> Result<Value> {
        let mut resolved = Map::new();
        let mut missing = Vec::new();

        handle_cache_field_generic(
            "deployer",
            &mut self.deployer,
            cache,
            &mut resolved,
            &mut missing,
        )?;

        // Before handling boring vault check if it is a ContractName we need to query from deployer.
        if let Some(bv) = &self.boring_vault {
            match bv {
                AddressOrContractName::ContractName(name) => {
                    // Query address from deployer, then assign boring_vault to be the address derived
                    todo!()
                }
                AddressOrContractName::Address(addr) => {
                    // Nothing to do
                }
            }
        }

        handle_cache_field_generic(
            "boring_vault",
            &mut self.boring_vault,
            cache,
            &mut resolved,
            &mut missing,
        )?;

        handle_cache_field_generic(
            "boring_vault_name",
            &mut self.boring_vault_name,
            cache,
            &mut resolved,
            &mut missing,
        )?;

        handle_cache_field_generic(
            "boring_vault_symbol",
            &mut self.boring_vault_symbol,
            cache,
            &mut resolved,
            &mut missing,
        )?;

        handle_cache_field_generic(
            "boring_vault_decimals",
            &mut self.boring_vault_decimals,
            cache,
            &mut resolved,
            &mut missing,
        )?;

        if resolved.is_empty() {
            // No new values to report to cache.
            // Check if we are missing values.
            // If so return error.
            if !missing.is_empty() {
                return Err(eyre!(MissingCacheValuesError { missing }));
            }
            // We have no missing, and nothing to add to cache we are done.
            Ok(Value::Null)
        } else {
            // We have something to add to the cache
            Ok(Value::Object(resolved))
        }
    }
}
