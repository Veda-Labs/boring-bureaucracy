use super::building_block::BuildingBlock;
// use crate::actions::sender_type::SenderType;
use crate::actions::action::Action;
// use crate::actions::deploy_contract_action::DeployContract;
use crate::bindings::boring_vault::BoringVault;
use crate::block_manager::shared_cache::{CacheValue, SharedCache};
// use crate::bytecode::BORING_VAULT_BYTECODE;
use crate::utils::address_or_contract_name::{AddressOrContractName, derive_contract_address};
use crate::utils::view_request_manager::ViewRequestManager;
use alloy::primitives::{Address, Bytes};
use alloy::sol_types::SolCall;
use building_block_derive::BuildingBlockCache;
use eyre::Result;
use serde::Deserialize;

#[derive(BuildingBlockCache, Debug, Deserialize)]
pub struct BoringVaultBlock {
    #[serde(default)]
    pub deployer: Option<Address>,
    #[serde(default)]
    pub boring_vault: Option<AddressOrContractName>,
    #[serde(default)]
    pub roles_authority: Option<AddressOrContractName>,
    #[serde(default)]
    #[can_derive]
    pub boring_vault_name: Option<String>,
    #[serde(default)]
    #[can_derive]
    pub boring_vault_symbol: Option<String>,
    #[serde(default)]
    #[can_derive]
    pub boring_vault_decimals: Option<u8>,
    #[serde(default)]
    pub hook: Option<AddressOrContractName>,
    #[serde(default)]
    pub manager: Option<AddressOrContractName>,
    #[serde(default)]
    pub teller: Option<AddressOrContractName>,
    #[serde(default)]
    executor: Option<Address>,
}

// TODO how do we verify after deployment???
// TODO could probably use util funcitons for shared logic between building blocks

impl BoringVaultBlock {
    async fn derive_boring_vault_name(
        &self,
        cache: &SharedCache,
        vrm: &ViewRequestManager,
    ) -> Result<bool> {
        // Read authority of boring vault if deployed.
        let boring_vault = cache.get_address("boring_vault").await;
        let boring_vault = match boring_vault {
            Some(addr) => addr,
            None => return Ok(false),
        };
        if vrm.request_code(boring_vault).await?.len() > 0 {
            // Query name of boring vault.
            let calldata = Bytes::from(BoringVault::nameCall::new(()).abi_encode());
            let result = vrm.request(boring_vault, calldata).await;
            if let Ok(res) = result {
                let data = BoringVault::nameCall::abi_decode_returns(&res, true)?;
                cache
                    .set(
                        "boring_vault_name",
                        CacheValue::String(data._0.to_string()),
                        "boring_vault_block",
                    )
                    .await?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn derive_boring_vault_symbol(
        &self,
        cache: &SharedCache,
        vrm: &ViewRequestManager,
    ) -> Result<bool> {
        // Read authority of boring vault if deployed.
        let boring_vault = cache.get_address("boring_vault").await;
        let boring_vault = match boring_vault {
            Some(addr) => addr,
            None => return Ok(false),
        };
        if vrm.request_code(boring_vault).await?.len() > 0 {
            // Query symbol of boring vault.
            let calldata = Bytes::from(BoringVault::symbolCall::new(()).abi_encode());
            let result = vrm.request(boring_vault, calldata).await;
            if let Ok(res) = result {
                let data = BoringVault::symbolCall::abi_decode_returns(&res, true)?;
                cache
                    .set(
                        "boring_vault_symbol",
                        CacheValue::String(data._0.to_string()),
                        "boring_vault_block",
                    )
                    .await?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn derive_boring_vault_decimals(
        &self,
        cache: &SharedCache,
        vrm: &ViewRequestManager,
    ) -> Result<bool> {
        // Read authority of boring vault if deployed.
        let boring_vault = cache.get_address("boring_vault").await;
        let boring_vault = match boring_vault {
            Some(addr) => addr,
            None => return Ok(false),
        };
        if vrm.request_code(boring_vault).await?.len() > 0 {
            // Query decimals of boring vault.
            let calldata = Bytes::from(BoringVault::decimalsCall::new(()).abi_encode());
            let result = vrm.request(boring_vault, calldata).await;
            if let Ok(res) = result {
                let data = BoringVault::decimalsCall::abi_decode_returns(&res, true)?;
                cache
                    .set(
                        "boring_vault_decimals",
                        CacheValue::U8(data._0),
                        "boring_vault_block",
                    )
                    .await?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn _assemble(
        &self,
        _cache: &SharedCache,
        _vrm: &ViewRequestManager,
    ) -> Result<Vec<Box<dyn Action>>> {
        // let mut actions: Vec<Box<dyn Action>> = Vec::new();
        // TODO make RPC calls checking if boring vault is deployed, if not add deploy action
        // then we would error here if the name, symbol, and decimal were not defined
        // Check if roles auth deployed
        // if yes, make sure roles are correct
        // if no, configure all roles

        // Check if boring vault is deployed.
        // let boring_vault = cache.get_address("boring_vault").await.unwrap();
        // let is_deployed = vrm.request_code(boring_vault).await?.len() > 0;
        // if !is_deployed {
        //     // Boring vault is not deployed
        //     if self.boring_vault_name.is_none()
        //         || self.boring_vault_symbol.is_none()
        //         || self.boring_vault_decimals.is_none()
        //     {
        //         return Err(eyre!(
        //             "Deploying boring vault but missing name, symbol, or decimals"
        //         ));
        //     }
        //     let name = match self.boring_vault.as_ref().unwrap() {
        //         AddressOrContractName::ContractName(name) => name,
        //         AddressOrContractName::Address(_) => {
        //             return Err(eyre!(
        //                 "BoringVaultBlock: Deploying boring vault but no name provided"
        //             ));
        //         }
        //     };

        //     let constructor_args = Bytes::from(
        //         BoringVault::constructorCall::new((
        //             Address::ZERO,
        //             self.boring_vault_name.as_ref().unwrap().clone(),
        //             self.boring_vault_symbol.as_ref().unwrap().clone(),
        //             self.boring_vault_decimals.unwrap(),
        //         ))
        //         .abi_encode(),
        //     );

        //     let deploy_borign_vault_action = DeployContract::new(
        //         self.deployer.unwrap(),
        //         name.to_string(),
        //         BORING_VAULT_BYTECODE,
        //         constructor_args,
        //         U256::ZERO,
        //         0,
        //         SenderType::EOA(self.executor.unwrap()),
        //     );

        //     actions.push(Box::new(deploy_borign_vault_action));
        // }
        // TODO now add all roles auth actions
        // Check if role is configured properly, if not add it

        // Then if hook is Some, set it

        // Set authority to roles authority if needed
        // transfer ownership to the zero address.
        // Ok(actions)
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_manager::block_manager::BlockManager;
    use alloy::primitives::address;
    use serde_json::json;

    const RPC_URL: &str = "https://eth.llamarpc.com";

    async fn setup_block_manager(json: serde_json::Value) -> BlockManager {
        let mut manager = BlockManager::new(RPC_URL.to_string()).await.unwrap();
        manager.create_blocks_from_json_value(json).unwrap();
        manager
    }

    #[tokio::test]
    async fn test_scenario_a_full_config() {
        let json = json!([
            {
                "Global": {
                    "deployer": "0x5F2F11ad8656439d5C14d9B351f8b09cDaC2A02d"
                }
            },
            {
                "BoringVault": {
                    "boring_vault": "0xf0bb20865277aBd641a307eCe5Ee04E79073416C",
                    "roles_authority": "0x1111111111111111111111111111111111111111",
                    "boring_vault_name": "Test Vault",
                    "boring_vault_symbol": "TV",
                    "boring_vault_decimals": 18,
                    "hook": "0x2222222222222222222222222222222222222222",
                    "manager": "0x3333333333333333333333333333333333333333",
                    "teller": "0x4444444444444444444444444444444444444444",
                    "executor": "0x4444444444444444444444444444444444444444"
                }
            }
        ]);

        let mut manager = setup_block_manager(json).await;
        manager.propogate_shared_data().await.unwrap();

        let cache = manager.cache;
        assert_eq!(
            cache.get("deployer").await.unwrap(),
            CacheValue::Address(address!("0x5F2F11ad8656439d5C14d9B351f8b09cDaC2A02d"))
        );
        assert_eq!(
            cache.get("boring_vault").await.unwrap(),
            CacheValue::Address(address!("0xf0bb20865277aBd641a307eCe5Ee04E79073416C"))
        );
        assert_eq!(
            cache.get("roles_authority").await.unwrap(),
            CacheValue::Address(address!("0x1111111111111111111111111111111111111111"))
        );
        assert_eq!(
            cache.get("boring_vault_decimals").await.unwrap(),
            CacheValue::U8(18)
        );
        assert_eq!(
            cache.get("hook").await.unwrap(),
            CacheValue::Address(address!("0x2222222222222222222222222222222222222222"))
        );
        assert_eq!(
            cache.get("manager").await.unwrap(),
            CacheValue::Address(address!("0x3333333333333333333333333333333333333333"))
        );
        assert_eq!(
            cache.get("teller").await.unwrap(),
            CacheValue::Address(address!("0x4444444444444444444444444444444444444444"))
        );
    }

    #[tokio::test]
    async fn test_scenario_b_minimal_config() {
        let json = json!([
            {
                "Global": {
                    "deployer": "0x5F2F11ad8656439d5C14d9B351f8b09cDaC2A02d"
                }
            },
            {
                "BoringVault": {
                    "deployer": "0x5F2F11ad8656439d5C14d9B351f8b09cDaC2A02d",
                    "boring_vault": "0xf0bb20865277aBd641a307eCe5Ee04E79073416C",
                    "executor": "0x4444444444444444444444444444444444444444"
                }
            }
        ]);

        let mut manager = setup_block_manager(json).await;
        manager.propogate_shared_data().await.unwrap();

        let cache = manager.cache;
        assert_eq!(
            cache.get("deployer").await.unwrap(),
            CacheValue::Address(address!("0x5F2F11ad8656439d5C14d9B351f8b09cDaC2A02d"))
        );
        assert_eq!(
            cache.get("boring_vault").await.unwrap(),
            CacheValue::Address(address!("0xf0bb20865277aBd641a307eCe5Ee04E79073416C"))
        );
        assert_eq!(
            cache.get("roles_authority").await.unwrap(),
            CacheValue::Address(address!("0x485Bde66Bb668a51f2372E34e45B1c6226798122"))
        );
        assert_eq!(
            cache.get("teller").await.unwrap(),
            CacheValue::Address(address!("0x9AA79C84b79816ab920bBcE20f8f74557B514734"))
        );

        // Verify other values are not set
        assert_eq!(
            cache.get("boring_vault_decimals").await.unwrap(),
            CacheValue::U8(18)
        );
        assert!(cache.get("hook").await.is_none());
        assert!(cache.get("manager").await.is_none());
    }

    #[tokio::test]
    async fn test_scenario_c_global_and_boring_vault() {
        let json = json!([
            {
                "Global": {
                    "deployer": "0x5F2F11ad8656439d5C14d9B351f8b09cDaC2A02d",
                    "boring_vault": "Test Boring Vault V0.100",
                    "roles_authority": "0x4444444444444444444444444444444444444444",
                    "executor": "0x4444444444444444444444444444444444444444"
                }
            },
            {
                "BoringVault": {
                    "boring_vault_name": "Test Vault",
                    "boring_vault_symbol": "TV",
                    "boring_vault_decimals": 18
                }
            }
        ]);

        let mut manager = setup_block_manager(json).await;
        manager.propogate_shared_data().await.unwrap();

        let cache = manager.cache;
        // Verify values from Global block
        assert_eq!(
            cache.get("deployer").await.unwrap(),
            CacheValue::Address(address!("0x5F2F11ad8656439d5C14d9B351f8b09cDaC2A02d"))
        );
        let expected_boring_vault = derive_contract_address(
            "Test Boring Vault V0.100",
            address!("0x5F2F11ad8656439d5C14d9B351f8b09cDaC2A02d"),
        );
        assert_eq!(
            cache.get("boring_vault").await.unwrap(),
            CacheValue::Address(expected_boring_vault)
        );
        assert_eq!(
            cache.get("roles_authority").await.unwrap(),
            CacheValue::Address(address!("0x4444444444444444444444444444444444444444"))
        );

        // Verify values from BoringVault block
        assert_eq!(
            cache.get("boring_vault_decimals").await.unwrap(),
            CacheValue::U8(18)
        );
    }
}
