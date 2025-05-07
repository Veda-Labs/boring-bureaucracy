use super::building_block::Actionable;
use crate::actions::admin_action::AdminAction;
use crate::bindings::{
    auth::Auth, boring_vault::BoringVault, multisig::GnosisSafe,
    teller::TellerWithMultiAssetSupport, timelock::Timelock,
};
use crate::block_manager::shared_cache::{CacheValue, SharedCache};
use crate::utils::address_or_contract_name::{AddressOrContractName, derive_contract_address};
use crate::utils::view_request_manager::ViewRequestManager;
use alloy::primitives::{Address, Bytes};
use alloy::sol_types::SolCall;
use async_trait::async_trait;
use eyre::{Result, eyre};
use log::{error, warn};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BoringVaultBlock {
    #[serde(default)]
    pub deployer: Option<Address>,
    #[serde(default)]
    pub boring_vault: Option<AddressOrContractName>,
    #[serde(default)]
    boring_vault_address: Option<Address>,
    #[serde(default)]
    pub roles_authority: Option<AddressOrContractName>,
    #[serde(default)]
    roles_authority_address: Option<Address>,
    #[serde(default)]
    pub boring_vault_name: Option<String>,
    #[serde(default)]
    pub boring_vault_symbol: Option<String>,
    #[serde(default)]
    pub boring_vault_decimals: Option<u8>,
    #[serde(default)]
    pub hook: Option<AddressOrContractName>,
    #[serde(default)]
    hook_address: Option<Address>,
    #[serde(default)]
    pub manager: Option<AddressOrContractName>,
    #[serde(default)]
    manager_address: Option<Address>,
    #[serde(default)]
    pub teller: Option<AddressOrContractName>,
    #[serde(default)]
    teller_address: Option<Address>,
}

#[async_trait]
impl Actionable for BoringVaultBlock {
    async fn to_actions(&self, vrm: &ViewRequestManager) -> Result<Vec<Box<dyn AdminAction>>> {
        // TODO make RPC calls checking if boring vault is deployed, if not add deploy action
        // then we would error here if the name, symbol, and decimal were not defined
        // Check if roles auth deployed
        // if yes, make sure roles are correct
        // if no, configure all roles

        // Check if boring vault is deployed.
        if let Some(boring_vault) = self.boring_vault_address {
            if vrm.request_code(boring_vault).await?.len() == 0 {
                // Boring vault is not deployed
                if self.boring_vault_name.is_none()
                    || self.boring_vault_symbol.is_none()
                    || self.boring_vault_decimals.is_none()
                {
                    return Err(eyre!(
                        "Deploying boring vault but missing name, symbol, or decimals"
                    ));
                }
            }
        }
        Ok(vec![])
    }

    // TODO current logic is blocking when waiting for address resolution, but
    // this can be refactored to concurrently get values from the cache
    async fn resolve_and_contribute(
        &mut self,
        cache: &SharedCache,
        vrm: &ViewRequestManager,
    ) -> Result<()> {
        if let Some(deployer) = &self.deployer {
            cache
                .set(
                    "deployer",
                    CacheValue::Address(*deployer),
                    "boring_vault_block",
                )
                .await?;
        } else {
            // Read the value from the cache.
            let result = cache.get("deployer", "boring_vault_block").await?;
            match result {
                CacheValue::Address(addr) => self.deployer = Some(addr),
                _ => return Err(eyre!("BoringVaultBlock: Cache deployer is not an address")),
            }
        }

        if let Some(boring_vault) = &self.boring_vault {
            match boring_vault {
                AddressOrContractName::Address(addr) => {
                    self.boring_vault_address = Some(*addr);
                    cache
                        .set(
                            "boring_vault",
                            CacheValue::Address(*addr),
                            "boring_vault_block",
                        )
                        .await?;
                }
                AddressOrContractName::ContractName(name) => {
                    if let Some(deployer) = &self.deployer {
                        let addr = derive_contract_address(name, *deployer);
                        self.boring_vault_address = Some(addr);
                        cache
                            .set(
                                "boring_vault",
                                CacheValue::Address(addr),
                                "boring_vault_block",
                            )
                            .await?;
                    }
                }
            }
        } else {
            // Read the value from the cache.
            let result = cache.get("boring_vault", "boring_vault_block").await?;
            match result {
                CacheValue::Address(addr) => self.boring_vault_address = Some(addr),
                _ => {
                    return Err(eyre!(
                        "BoringVaultBlock: Cache boring_vault is not an address"
                    ));
                }
            }
        }

        if let Some(roles_authority) = &self.roles_authority {
            match roles_authority {
                AddressOrContractName::Address(addr) => {
                    self.roles_authority_address = Some(*addr);
                    cache
                        .set(
                            "roles_authority",
                            CacheValue::Address(*addr),
                            "boring_vault_block",
                        )
                        .await?;
                }
                AddressOrContractName::ContractName(name) => {
                    if let Some(deployer) = &self.deployer {
                        let addr = derive_contract_address(name, *deployer);
                        self.roles_authority_address = Some(addr);
                        cache
                            .set(
                                "roles_authority",
                                CacheValue::Address(addr),
                                "boring_vault_block",
                            )
                            .await?;
                    }
                }
            }
        } else {
            // Read the value from the cache.
            let result = cache.get("roles_authority", "boring_vault_block").await;
            if let Ok(res) = result {
                match res {
                    CacheValue::Address(addr) => self.roles_authority_address = Some(addr),
                    _ => {
                        return Err(eyre!(
                            "BoringVaultBlock: Cache roles_authority is not an address"
                        ));
                    }
                }
            } else {
                // Try to query the roles authority from the boring vault.
                if let Some(boring_vault) = &self.boring_vault_address {
                    let calldata = Bytes::from(Auth::authorityCall::new(()).abi_encode());
                    let result = vrm.request(*boring_vault, calldata).await;
                    if let Ok(res) = result {
                        let data = Auth::authorityCall::abi_decode_returns(&res, true)?;
                        self.roles_authority_address = Some(data.authority);
                        cache
                            .set(
                                "roles_authority",
                                CacheValue::Address(data.authority),
                                "boring_vault_block",
                            )
                            .await?;
                    }
                }
            }
        }

        if let Some(decimals) = self.boring_vault_decimals {
            cache
                .set(
                    "boring_vault_decimals",
                    CacheValue::U8(decimals),
                    "boring_vault_block",
                )
                .await?;
        } else {
            // Try to query decimals from the boring vault.
            if let Some(boring_vault) = &self.boring_vault_address {
                let calldata = Bytes::from(BoringVault::decimalsCall::new(()).abi_encode());
                let result = vrm.request(*boring_vault, calldata).await;
                if let Ok(res) = result {
                    let data = BoringVault::decimalsCall::abi_decode_returns(&res, true)?;
                    self.boring_vault_decimals = Some(data._0);
                    cache
                        .set(
                            "boring_vault_decimals",
                            CacheValue::U8(data._0),
                            "boring_vault_block",
                        )
                        .await?;
                } // else leave boring_vault_decimals as None
            }
        }

        if let Some(hook) = &self.hook {
            match hook {
                AddressOrContractName::Address(addr) => {
                    self.hook_address = Some(*addr);
                    cache
                        .set("hook", CacheValue::Address(*addr), "boring_vault_block")
                        .await?;
                }
                AddressOrContractName::ContractName(name) => {
                    if let Some(deployer) = &self.deployer {
                        let addr = derive_contract_address(name, *deployer);
                        self.hook_address = Some(addr);
                        cache
                            .set("hook", CacheValue::Address(addr), "boring_vault_block")
                            .await?;
                    }
                }
            }
        } else {
            // Read the value from the cache.
            let result = cache.get("hook", "boring_vault_block").await;
            if let Ok(res) = result {
                match res {
                    CacheValue::Address(addr) => self.hook_address = Some(addr),
                    _ => {
                        return Err(eyre!("BoringVaultBlock: Cache hook is not an address"));
                    }
                }
            } else {
                warn!("BoringVaultBlock: hook address not defined locally or in cache");
            }
        }

        if let Some(manager) = &self.manager {
            match manager {
                AddressOrContractName::Address(addr) => {
                    self.manager_address = Some(*addr);
                    cache
                        .set("manager", CacheValue::Address(*addr), "boring_vault_block")
                        .await?;
                }
                AddressOrContractName::ContractName(name) => {
                    if let Some(deployer) = &self.deployer {
                        let addr = derive_contract_address(name, *deployer);
                        self.manager_address = Some(addr);
                        cache
                            .set("manager", CacheValue::Address(addr), "boring_vault_block")
                            .await?;
                    }
                }
            }
        } else {
            // Read the value from the cache.
            let result = cache.get("manager", "boring_vault_block").await;
            if let Ok(res) = result {
                match res {
                    CacheValue::Address(addr) => self.manager_address = Some(addr),
                    _ => {
                        return Err(eyre!("BoringVaultBlock: Cache manager is not an address"));
                    }
                }
            } else {
                warn!("BoringVaultBlock: manager address not defined locally or in cache");
            }
        }

        if let Some(teller) = &self.teller {
            match teller {
                AddressOrContractName::Address(addr) => {
                    self.teller_address = Some(*addr);
                    cache
                        .set("teller", CacheValue::Address(*addr), "boring_vault_block")
                        .await?;
                }
                AddressOrContractName::ContractName(name) => {
                    if let Some(deployer) = &self.deployer {
                        let addr = derive_contract_address(name, *deployer);
                        self.teller_address = Some(addr);
                        cache
                            .set("teller", CacheValue::Address(addr), "boring_vault_block")
                            .await?;
                    }
                }
            }
        } else {
            // Read the value from the cache.
            let result = cache.get("teller", "boring_vault_block").await;
            if let Ok(res) = result {
                match res {
                    CacheValue::Address(addr) => self.teller_address = Some(addr),
                    _ => {
                        return Err(eyre!("BoringVaultBlock: Cache teller is not an address"));
                    }
                }
            } else {
                warn!("BoringVaultBlock: teller address not defined locally or in cache");
            }
        }

        Ok(())
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
                "BoringVault": {
                    "deployer": "0x5F2F11ad8656439d5C14d9B351f8b09cDaC2A02d",
                    "boring_vault": "0xf0bb20865277aBd641a307eCe5Ee04E79073416C",
                    "roles_authority": "0x1111111111111111111111111111111111111111",
                    "boring_vault_name": "Test Vault",
                    "boring_vault_symbol": "TV",
                    "boring_vault_decimals": 18,
                    "hook": "0x2222222222222222222222222222222222222222",
                    "manager": "0x3333333333333333333333333333333333333333",
                    "teller": "0x4444444444444444444444444444444444444444"
                }
            }
        ]);

        let mut manager = setup_block_manager(json).await;
        manager.propogate_shared_data().await.unwrap();

        let cache = manager.cache;
        assert_eq!(
            cache.get_immediate("deployer").await.unwrap(),
            CacheValue::Address(address!("0x5F2F11ad8656439d5C14d9B351f8b09cDaC2A02d"))
        );
        assert_eq!(
            cache.get_immediate("boring_vault").await.unwrap(),
            CacheValue::Address(address!("0xf0bb20865277aBd641a307eCe5Ee04E79073416C"))
        );
        assert_eq!(
            cache.get_immediate("roles_authority").await.unwrap(),
            CacheValue::Address(address!("0x1111111111111111111111111111111111111111"))
        );
        assert_eq!(
            cache.get_immediate("boring_vault_decimals").await.unwrap(),
            CacheValue::U8(18)
        );
        assert_eq!(
            cache.get_immediate("hook").await.unwrap(),
            CacheValue::Address(address!("0x2222222222222222222222222222222222222222"))
        );
        assert_eq!(
            cache.get_immediate("manager").await.unwrap(),
            CacheValue::Address(address!("0x3333333333333333333333333333333333333333"))
        );
        assert_eq!(
            cache.get_immediate("teller").await.unwrap(),
            CacheValue::Address(address!("0x4444444444444444444444444444444444444444"))
        );
    }

    #[tokio::test]
    async fn test_scenario_b_minimal_config() {
        let json = json!([
            {
                "BoringVault": {
                    "deployer": "0x5F2F11ad8656439d5C14d9B351f8b09cDaC2A02d",
                    "boring_vault": "0xf0bb20865277aBd641a307eCe5Ee04E79073416C"
                }
            }
        ]);

        let mut manager = setup_block_manager(json).await;
        manager.propogate_shared_data().await.unwrap();

        let cache = manager.cache;
        assert_eq!(
            cache.get_immediate("deployer").await.unwrap(),
            CacheValue::Address(address!("0x5F2F11ad8656439d5C14d9B351f8b09cDaC2A02d"))
        );
        assert_eq!(
            cache.get_immediate("boring_vault").await.unwrap(),
            CacheValue::Address(address!("0xf0bb20865277aBd641a307eCe5Ee04E79073416C"))
        );

        // Verify other values are not set
        assert_eq!(
            cache.get_immediate("roles_authority").await.unwrap(),
            CacheValue::Address(address!("0x485Bde66Bb668a51f2372E34e45B1c6226798122"))
        );
        assert_eq!(
            cache.get_immediate("boring_vault_decimals").await.unwrap(),
            CacheValue::U8(18)
        );
        assert!(cache.get_immediate("hook").await.is_err());
        assert!(cache.get_immediate("manager").await.is_err());
        assert!(cache.get_immediate("teller").await.is_err());
    }

    #[tokio::test]
    async fn test_scenario_c_global_and_boring_vault() {
        let json = json!([
            {
                "Global": {
                    "deployer": "0x5F2F11ad8656439d5C14d9B351f8b09cDaC2A02d",
                    "boring_vault": "Test Boring Vault V0.100",
                    "roles_authority": "0x4444444444444444444444444444444444444444"
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
            cache.get_immediate("deployer").await.unwrap(),
            CacheValue::Address(address!("0x5F2F11ad8656439d5C14d9B351f8b09cDaC2A02d"))
        );
        let expected_boring_vault = derive_contract_address(
            "Test Boring Vault V0.100",
            address!("0x5F2F11ad8656439d5C14d9B351f8b09cDaC2A02d"),
        );
        assert_eq!(
            cache.get_immediate("boring_vault").await.unwrap(),
            CacheValue::Address(expected_boring_vault)
        );
        assert_eq!(
            cache.get_immediate("roles_authority").await.unwrap(),
            CacheValue::Address(address!("0x4444444444444444444444444444444444444444"))
        );

        // Verify values from BoringVault block
        assert_eq!(
            cache.get_immediate("boring_vault_decimals").await.unwrap(),
            CacheValue::U8(18)
        );
    }
}
