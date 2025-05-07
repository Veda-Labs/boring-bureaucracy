use super::{assets_block::AssetsBlock, teller_block::TellerBlock};
use crate::actions::admin_action::AdminAction;
use crate::utils::address_or_contract_name::AddressOrContractName;
use alloy::primitives::Address;
use async_trait::async_trait;
use eyre::{Result, eyre};
use into_trait::IntoTraitObject;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fmt;

pub trait CacheField: Sized + PartialEq + std::fmt::Debug {
    fn parse_from_cache(s: &str) -> eyre::Result<Self>;
    fn to_cache_value(&self) -> Result<Value>;
}

impl CacheField for Address {
    fn parse_from_cache(s: &str) -> eyre::Result<Self> {
        s.parse::<Address>()
            .map_err(|_| eyre!("Invalid address: {}", s))
    }
    fn to_cache_value(&self) -> Result<Value> {
        Ok(Value::String(self.to_string()))
    }
}

impl CacheField for u8 {
    fn parse_from_cache(s: &str) -> eyre::Result<Self> {
        s.parse::<u8>().map_err(|_| eyre!("Invalid u8: {}", s))
    }
    fn to_cache_value(&self) -> Result<Value> {
        Ok(Value::Number((*self).into()))
    }
}

impl CacheField for String {
    fn parse_from_cache(s: &str) -> eyre::Result<Self> {
        Ok(s.to_string())
    }
    fn to_cache_value(&self) -> Result<Value> {
        Ok(Value::String(self.clone()))
    }
}

impl CacheField for AddressOrContractName {
    fn parse_from_cache(s: &str) -> eyre::Result<Self> {
        // Only allow Address from cache
        let addr = s
            .parse::<Address>()
            .map_err(|_| eyre!("Invalid address: {}", s))?;
        Ok(AddressOrContractName::Address(addr))
    }
    fn to_cache_value(&self) -> Result<Value> {
        match self {
            AddressOrContractName::Address(addr) => Ok(Value::String(addr.to_string())),
            AddressOrContractName::ContractName(name) => {
                // Error: name resolution should have happened before this point
                return Err(eyre!(
                    "Tried to use unresolved contract name '{}' in cache field logic",
                    name
                ));
            }
        }
    }
}

pub fn handle_cache_field_generic<T: CacheField>(
    key: &str,
    field: &mut Option<T>,
    cache: &Value,
    resolved: &mut Map<String, Value>,
    missing: &mut Vec<String>,
) -> eyre::Result<()> {
    if let Some(cache_str) = cache.get(key).and_then(|v| v.as_str()) {
        let val = T::parse_from_cache(cache_str)?;
        match field {
            Some(existing) => {
                if *existing != val {
                    return Err(eyre!(
                        "Field '{}' mismatch: struct has {:?}, cache has {:?}",
                        key,
                        existing,
                        val
                    ));
                }
            }
            None => {
                *field = Some(val);
            }
        }
    } else if let Some(existing) = field {
        let val = existing.to_cache_value()?;
        resolved.insert(key.to_string(), val);
    } else {
        missing.push(key.to_string());
    }
    Ok(())
}

#[derive(Debug)]
pub struct MissingCacheValuesError {
    pub missing: Vec<String>,
}

impl fmt::Display for MissingCacheValuesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Missing required cache values: {}",
            self.missing.join(", ")
        )
    }
}

impl std::error::Error for MissingCacheValuesError {}

#[async_trait]
pub trait Actionable {
    async fn to_actions(&self) -> Result<Vec<Box<dyn AdminAction>>>;
    async fn resolve_and_contribute(&mut self, cache: &Value) -> Result<Value>;
}

#[derive(Deserialize, Debug, IntoTraitObject)]
#[trait_name(Actionable)]
pub enum BuildingBlock {
    Assets(AssetsBlock),
    Teller(TellerBlock),
    // ...add more as needed
}
