use super::building_block::BuildingBlock;
use crate::actions::action::Action;
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
use serde::Deserialize;

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
    #[serde(default)]
    pub multisig: Option<AddressOrContractName>,
    #[serde(default)]
    pub timelock: Option<AddressOrContractName>,
    #[serde(default)]
    executor: Option<Address>,
    #[serde(default)]
    timelock_admin: Option<Address>,
}

impl GlobalBlock {
    async fn derive_roles_authority(
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
            // Query authority of boring vault.
            let calldata = Bytes::from(Auth::authorityCall::new(()).abi_encode());
            let result = vrm.request(boring_vault, calldata).await;
            if let Ok(res) = result {
                let data = Auth::authorityCall::abi_decode_returns(&res, true)?;
                cache
                    .set(
                        "roles_authority",
                        CacheValue::Address(data.authority),
                        "global_block",
                    )
                    .await?;
                return Ok(true);
            }
        }

        Ok(false)
    }

    async fn derive_teller(&self, cache: &SharedCache, vrm: &ViewRequestManager) -> Result<bool> {
        // Read hook of boring vault if deployed.
        let boring_vault = cache.get_address("boring_vault").await;
        let boring_vault = match boring_vault {
            Some(addr) => addr,
            None => return Ok(false),
        };
        if vrm.request_code(boring_vault).await?.len() > 0 {
            // Query hook of boring vault.
            let calldata = Bytes::from(BoringVault::hookCall::new(()).abi_encode());
            let result = vrm.request(boring_vault, calldata).await;
            if let Ok(res) = result {
                let data = BoringVault::hookCall::abi_decode_returns(&res, true)?;
                if data.hook != Address::ZERO {
                    cache
                        .set("teller", CacheValue::Address(data.hook), "global_block")
                        .await?;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    async fn derive_multisig(&self, cache: &SharedCache, vrm: &ViewRequestManager) -> Result<bool> {
        // Read owner of roles authority if deployed.
        let roles_authority = cache.get_address("roles_authority").await;
        let roles_authority = match roles_authority {
            Some(addr) => addr,
            None => return Ok(false),
        };
        if vrm.request_code(roles_authority).await?.len() > 0 {
            // Query owner of roles_authority.
            let calldata = Bytes::from(Auth::ownerCall::new(()).abi_encode());
            let result = vrm.request(roles_authority, calldata).await;
            if let Ok(res) = result {
                let data = Auth::ownerCall::abi_decode_returns(&res, true)?;
                // Check if the owner is actually a multisig by trying to call nonce on it.
                let calldata = Bytes::from(GnosisSafe::nonceCall::new(()).abi_encode());
                let result = vrm.request(data.owner, calldata).await;
                if result.is_ok() {
                    // Call succeeded, so owner is a multisig.
                    cache
                        .set("multisig", CacheValue::Address(data.owner), "global_block")
                        .await?;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    async fn derive_timelock(&self, cache: &SharedCache, vrm: &ViewRequestManager) -> Result<bool> {
        // Read owner of roles authority if deployed.
        let roles_authority = cache.get_address("roles_authority").await;
        let roles_authority = match roles_authority {
            Some(addr) => addr,
            None => return Ok(false),
        };
        if vrm.request_code(roles_authority).await?.len() > 0 {
            // Query owner of roles_authority.
            let calldata = Bytes::from(Auth::ownerCall::new(()).abi_encode());
            let result = vrm.request(roles_authority, calldata).await;
            if let Ok(res) = result {
                let data = Auth::ownerCall::abi_decode_returns(&res, true)?;
                // Check if the owner is actually a timelock by trying to call getMinDelay on it.
                let calldata = Bytes::from(Timelock::getMinDelayCall::new(()).abi_encode());
                let result = vrm.request(data.owner, calldata).await;
                if result.is_ok() {
                    // Call succeeded, so owner is a timelock.
                    cache
                        .set("timelock", CacheValue::Address(data.owner), "global_block")
                        .await?;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    async fn derive_accountant(
        &self,
        cache: &SharedCache,
        vrm: &ViewRequestManager,
    ) -> Result<bool> {
        // Read accountant of teller if deployed.
        let teller = cache.get_address("teller").await;
        let teller = match teller {
            Some(addr) => addr,
            None => return Ok(false),
        };
        if vrm.request_code(teller).await?.len() > 0 {
            // Query accountant of teller.
            let calldata =
                Bytes::from(TellerWithMultiAssetSupport::accountantCall::new(()).abi_encode());
            let result = vrm.request(teller, calldata).await;
            if let Ok(res) = result {
                let data =
                    TellerWithMultiAssetSupport::accountantCall::abi_decode_returns(&res, true)?;
                cache
                    .set("accountant", CacheValue::Address(data._0), "global_block")
                    .await?;
                return Ok(true);
            }
        }
        Ok(false)
    }
}

#[async_trait]
impl BuildingBlock for GlobalBlock {
    async fn assemble(&self, _vrm: &ViewRequestManager) -> Result<Vec<Box<dyn Action>>> {
        Ok(vec![])
    }

    async fn resolve_values(&self, cache: &SharedCache) -> Result<()> {
        if let Some(network_id) = self.network_id {
            cache
                .set("network_id", CacheValue::U32(network_id), "global_block")
                .await?;
        }
        if let Some(deployer) = self.deployer {
            cache
                .set("deployer", CacheValue::Address(deployer), "global_block")
                .await?;
        }
        if let Some(boring_vault) = &self.boring_vault {
            cache
                .set(
                    "boring_vault",
                    CacheValue::Address(boring_vault.resolve_to_address(self.deployer)?),
                    "global_block",
                )
                .await?;
        }
        if let Some(roles_authority) = &self.roles_authority {
            cache
                .set(
                    "roles_authority",
                    CacheValue::Address(roles_authority.resolve_to_address(self.deployer)?),
                    "global_block",
                )
                .await?;
        }
        if let Some(teller) = &self.teller {
            cache
                .set(
                    "teller",
                    CacheValue::Address(teller.resolve_to_address(self.deployer)?),
                    "global_block",
                )
                .await?;
        }
        if let Some(accountant) = &self.accountant {
            cache
                .set(
                    "accountant",
                    CacheValue::Address(accountant.resolve_to_address(self.deployer)?),
                    "global_block",
                )
                .await?;
        }
        if let Some(manager) = &self.manager {
            cache
                .set(
                    "manager",
                    CacheValue::Address(manager.resolve_to_address(self.deployer)?),
                    "global_block",
                )
                .await?;
        }
        if let Some(multisig) = &self.multisig {
            cache
                .set(
                    "multisig",
                    CacheValue::Address(multisig.resolve_to_address(self.deployer)?),
                    "global_block",
                )
                .await?;
        }
        if let Some(timelock) = &self.timelock {
            cache
                .set(
                    "timelock",
                    CacheValue::Address(timelock.resolve_to_address(self.deployer)?),
                    "global_block",
                )
                .await?;
        }
        if let Some(executor) = self.executor {
            cache
                .set("executor", CacheValue::Address(executor), "global_block")
                .await?;
        }
        if let Some(timelock_admin) = self.timelock_admin {
            cache
                .set(
                    "timelock_admin",
                    CacheValue::Address(timelock_admin),
                    "global_block",
                )
                .await?;
        }

        Ok(())
    }

    async fn report_missing_values(&self, cache: &SharedCache) -> Result<Vec<(String, bool)>> {
        let mut requires = Vec::new();
        if self.deployer.is_none() && cache.get("deployer").await.is_none() {
            requires.push(("deployer".to_string(), false));
        }
        if self.boring_vault.is_none() && cache.get("boring_vault").await.is_none() {
            requires.push(("boring_vault".to_string(), false));
        }
        if self.roles_authority.is_none() && cache.get("roles_authority").await.is_none() {
            requires.push(("roles_authority".to_string(), true));
        }
        if self.teller.is_none() && cache.get("teller").await.is_none() {
            requires.push(("teller".to_string(), true));
        }
        if self.accountant.is_none() && cache.get("accountant").await.is_none() {
            requires.push(("accountant".to_string(), true));
        }
        if self.manager.is_none() && cache.get("manager").await.is_none() {
            requires.push(("manager".to_string(), false));
        }
        if self.multisig.is_none() && cache.get("multisig").await.is_none() {
            requires.push(("multisig".to_string(), true));
        }
        if self.timelock.is_none() && cache.get("timelock").await.is_none() {
            requires.push(("timelock".to_string(), true));
        }
        if self.executor.is_none() && cache.get("executor").await.is_none() {
            requires.push(("executor".to_string(), false));
        }
        if self.timelock_admin.is_none() && cache.get("timelock_admin").await.is_none() {
            requires.push(("timelock_admin".to_string(), false));
        }

        Ok(requires)
    }

    async fn derive_value(
        &self,
        key: &str,
        cache: &SharedCache,
        vrm: &ViewRequestManager,
    ) -> Result<bool> {
        match key {
            "roles_authority" => self.derive_roles_authority(cache, vrm).await,
            "teller" => self.derive_teller(cache, vrm).await,
            "accountant" => self.derive_accountant(cache, vrm).await,
            "multisig" => self.derive_multisig(cache, vrm).await,
            "timelock" => self.derive_timelock(cache, vrm).await,

            _ => {
                return Err(eyre!(
                    "global_block: Requested resolution of {}, which is not supported",
                    key
                ));
            }
        }
    }

    async fn resolve_state(
        &mut self,
        _cache: &SharedCache,
        _vrm: &ViewRequestManager,
    ) -> Result<()> {
        // // Globals does not read anything from cache, only writes to it.
        // if let Some(deployer) = &self.deployer {
        //     cache
        //         .set("deployer", CacheValue::Address(*deployer), "global_block")
        //         .await?;
        // }

        // if let Some(network_id) = self.network_id {
        //     cache
        //         .set("network_id", CacheValue::U32(network_id), "global_block")
        //         .await?;
        // }

        // if let Some(boring_vault) = &self.boring_vault {
        //     match boring_vault {
        //         AddressOrContractName::Address(addr) => {
        //             self.boring_vault_address = Some(*addr);
        //             cache
        //                 .set("boring_vault", CacheValue::Address(*addr), "global_block")
        //                 .await?;
        //         }
        //         AddressOrContractName::ContractName(name) => {
        //             if let Some(deployer) = &self.deployer {
        //                 let addr = derive_contract_address(name, *deployer);
        //                 self.boring_vault_address = Some(addr);
        //                 cache
        //                     .set("boring_vault", CacheValue::Address(addr), "global_block")
        //                     .await?;
        //             }
        //         }
        //     }
        // }

        // // So add an else statement
        // if let Some(roles_authority) = &self.roles_authority {
        //     match roles_authority {
        //         AddressOrContractName::Address(addr) => {
        //             self.roles_authority_address = Some(*addr);
        //             cache
        //                 .set(
        //                     "roles_authority",
        //                     CacheValue::Address(*addr),
        //                     "global_block",
        //                 )
        //                 .await?;
        //         }
        //         AddressOrContractName::ContractName(name) => {
        //             if let Some(deployer) = &self.deployer {
        //                 let addr = derive_contract_address(name, *deployer);
        //                 self.roles_authority_address = Some(addr);
        //                 cache
        //                     .set("roles_authority", CacheValue::Address(addr), "global_block")
        //                     .await?;
        //             }
        //         }
        //     }
        // } else {
        //     // Get roles authority from boring vault if set.
        //     if let Some(boring_vault) = &self.boring_vault_address {
        //         let calldata = Bytes::from(Auth::authorityCall::new(()).abi_encode());
        //         let result = vrm.request(*boring_vault, calldata).await;
        //         if let Ok(res) = result {
        //             let data = Auth::authorityCall::abi_decode_returns(&res, true)?;
        //             self.roles_authority_address = Some(data.authority);
        //             cache
        //                 .set(
        //                     "roles_authority",
        //                     CacheValue::Address(data.authority),
        //                     "global_block",
        //                 )
        //                 .await?;
        //         }
        //     }
        // }

        // if let Some(teller) = &self.teller {
        //     match teller {
        //         AddressOrContractName::Address(addr) => {
        //             self.teller_address = Some(*addr);
        //             cache
        //                 .set("teller", CacheValue::Address(*addr), "global_block")
        //                 .await?;
        //         }
        //         AddressOrContractName::ContractName(name) => {
        //             if let Some(deployer) = &self.deployer {
        //                 let addr = derive_contract_address(name, *deployer);
        //                 self.teller_address = Some(addr);
        //                 cache
        //                     .set("teller", CacheValue::Address(addr), "global_block")
        //                     .await?;
        //             }
        //         }
        //     }
        // } else {
        //     // Try getting hook from boring vault.
        //     if let Some(boring_vault) = &self.boring_vault_address {
        //         let calldata = Bytes::from(BoringVault::hookCall::new(()).abi_encode());
        //         let result = vrm.request(*boring_vault, calldata).await;
        //         if let Ok(res) = result {
        //             if res.len() > 0 {
        //                 let data = BoringVault::hookCall::abi_decode_returns(&res, true)?;
        //                 println!("Deserialized Data!");
        //                 if data.hook != Address::ZERO {
        //                     // Only update this if hook is set
        //                     self.teller_address = Some(data.hook);
        //                     cache
        //                         .set("teller", CacheValue::Address(data.hook), "global_block")
        //                         .await?;
        //                 }
        //             }
        //         }
        //     }
        // }

        // if let Some(accountant) = &self.accountant {
        //     match accountant {
        //         AddressOrContractName::Address(addr) => {
        //             self.accountant_address = Some(*addr);
        //             cache
        //                 .set("accountant", CacheValue::Address(*addr), "global_block")
        //                 .await?;
        //         }
        //         AddressOrContractName::ContractName(name) => {
        //             if let Some(deployer) = &self.deployer {
        //                 let addr = derive_contract_address(name, *deployer);
        //                 self.accountant_address = Some(addr);
        //                 cache
        //                     .set("accountant", CacheValue::Address(addr), "global_block")
        //                     .await?;
        //             }
        //         }
        //     }
        // } else {
        //     // Read accountant from teller.
        //     if let Some(teller) = &self.teller_address {
        //         let calldata =
        //             Bytes::from(TellerWithMultiAssetSupport::accountantCall::new(()).abi_encode());
        //         let result = vrm.request(*teller, calldata).await;
        //         if let Ok(res) = result {
        //             let data = TellerWithMultiAssetSupport::accountantCall::abi_decode_returns(
        //                 &res, true,
        //             )?;
        //             if data._0 != Address::ZERO {
        //                 // Only update this if hook is set
        //                 self.accountant_address = Some(data._0);
        //                 cache
        //                     .set("accountant", CacheValue::Address(data._0), "global_block")
        //                     .await?;
        //             }
        //         }
        //     }
        // }

        // if let Some(manager) = &self.manager {
        //     match manager {
        //         AddressOrContractName::Address(addr) => {
        //             self.manager_address = Some(*addr);
        //             cache
        //                 .set("manager", CacheValue::Address(*addr), "global_block")
        //                 .await?;
        //         }
        //         AddressOrContractName::ContractName(name) => {
        //             if let Some(deployer) = &self.deployer {
        //                 let addr = derive_contract_address(name, *deployer);
        //                 self.manager_address = Some(addr);
        //                 cache
        //                     .set("manager", CacheValue::Address(addr), "global_block")
        //                     .await?;
        //             }
        //         }
        //     }
        // }

        // if let Some(multisig) = &self.multisig {
        //     match multisig {
        //         AddressOrContractName::Address(addr) => {
        //             self.multisig_address = Some(*addr);
        //             cache
        //                 .set("multisig", CacheValue::Address(*addr), "global_block")
        //                 .await?;
        //         }
        //         AddressOrContractName::ContractName(name) => {
        //             if let Some(deployer) = &self.deployer {
        //                 let addr = derive_contract_address(name, *deployer);
        //                 self.multisig_address = Some(addr);
        //                 cache
        //                     .set("multisig", CacheValue::Address(addr), "global_block")
        //                     .await?;
        //             }
        //         }
        //     }
        // } else {
        //     // Read owner of roles authority and attempt to call nonce on it.
        //     if let Some(roles_authority) = &self.roles_authority_address {
        //         let calldata = Bytes::from(Auth::ownerCall::new(()).abi_encode());
        //         let result = vrm.request(*roles_authority, calldata).await;
        //         if let Ok(res) = result {
        //             if res.len() > 0 {
        //                 let data = Auth::ownerCall::abi_decode_returns(&res, true)?;
        //                 let potential_multisig = data.owner;
        //                 // Attempt to call nonce on potential multisig.
        //                 let calldata = Bytes::from(GnosisSafe::nonceCall::new(()).abi_encode());
        //                 let result = vrm.request(potential_multisig, calldata).await;
        //                 if result.is_ok() {
        //                     // Don't care what the nonce is but we know this is a multisig
        //                     self.multisig_address = Some(potential_multisig);
        //                     cache
        //                         .set(
        //                             "multisig",
        //                             CacheValue::Address(potential_multisig),
        //                             "global_block",
        //                         )
        //                         .await?;
        //                 }
        //             }
        //         }
        //     }
        // }

        // if let Some(timelock) = &self.timelock {
        //     match timelock {
        //         AddressOrContractName::Address(addr) => {
        //             self.timelock_address = Some(*addr);
        //             cache
        //                 .set("timelock", CacheValue::Address(*addr), "global_block")
        //                 .await?;
        //         }
        //         AddressOrContractName::ContractName(name) => {
        //             if let Some(deployer) = &self.deployer {
        //                 let addr = derive_contract_address(name, *deployer);
        //                 self.timelock_address = Some(addr);
        //                 cache
        //                     .set("timelock", CacheValue::Address(addr), "global_block")
        //                     .await?;
        //             }
        //         }
        //     }
        // } else {
        //     // Read owner of roles authority and attempt to call getMinDelay on it.
        //     if let Some(roles_authority) = &self.roles_authority_address {
        //         let calldata = Bytes::from(Auth::ownerCall::new(()).abi_encode());
        //         let result = vrm.request(*roles_authority, calldata).await;
        //         if let Ok(res) = result {
        //             if res.len() > 0 {
        //                 let data = Auth::ownerCall::abi_decode_returns(&res, true)?;
        //                 let potential_timelock = data.owner;
        //                 // Attempt to call getMinDelay on potential timelock.
        //                 let calldata = Bytes::from(Timelock::getMinDelayCall::new(()).abi_encode());
        //                 let result = vrm.request(potential_timelock, calldata).await;
        //                 if result.is_ok() {
        //                     // Don't care what the min delay is but we know this is a timelock
        //                     self.timelock_address = Some(potential_timelock);
        //                     cache
        //                         .set(
        //                             "timelock",
        //                             CacheValue::Address(potential_timelock),
        //                             "global_block",
        //                         )
        //                         .await?;
        //                 }
        //             }
        //         }
        //     }
        // }

        // if let Some(executor) = self.executor {
        //     cache
        //         .set("executor", CacheValue::Address(executor), "global_block")
        //         .await?;
        // }

        // if let Some(timelock_admin) = self.timelock_admin {
        //     cache
        //         .set(
        //             "timelock_admin",
        //             CacheValue::Address(timelock_admin),
        //             "global_block",
        //         )
        //         .await?;
        // }

        Ok(())
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::block_manager::block_manager::BlockManager;
//     use alloy::primitives::address;
//     use serde_json::json;

//     const RPC_URL: &str = "https://eth.llamarpc.com";

//     async fn setup_block_manager(json: serde_json::Value) -> BlockManager {
//         let mut manager = BlockManager::new(RPC_URL.to_string()).await.unwrap();
//         manager.create_blocks_from_json_value(json).unwrap();
//         manager
//     }

//     #[tokio::test]
//     async fn test_scenario_a_minimal_config() {
//         let json = json!(
//             [
//                 {
//                     "Global": {
//                         "boring_vault": "0xf0bb20865277aBd641a307eCe5Ee04E79073416C",
//                         "deployer": "0x5F2F11ad8656439d5C14d9B351f8b09cDaC2A02d",
//                         "network_id": 1,
//                     }
//                 }
//             ]
//         );

//         let mut manager = setup_block_manager(json).await;
//         manager.propogate_shared_data().await.unwrap();

//         // Verify cache values
//         let cache = manager.cache;
//         assert_eq!(
//             cache.get("deployer", "test").await.unwrap(),
//             CacheValue::Address(address!("0x5F2F11ad8656439d5C14d9B351f8b09cDaC2A02d"))
//         );
//         assert_eq!(
//             cache.get("boring_vault", "test").await.unwrap(),
//             CacheValue::Address(address!("0xf0bb20865277aBd641a307eCe5Ee04E79073416C"))
//         );
//         assert_eq!(
//             cache.get("network_id", "test").await.unwrap(),
//             CacheValue::U32(1)
//         );
//         assert_eq!(
//             cache.get("roles_authority", "test").await.unwrap(),
//             CacheValue::Address(address!("0x485Bde66Bb668a51f2372E34e45B1c6226798122"))
//         );
//         assert_eq!(
//             cache.get("teller", "test").await.unwrap(),
//             CacheValue::Address(address!("0x9AA79C84b79816ab920bBcE20f8f74557B514734"))
//         );
//         assert_eq!(
//             cache.get("accountant", "test").await.unwrap(),
//             CacheValue::Address(address!("0x0d05D94a5F1E76C18fbeB7A13d17C8a314088198"))
//         );
//         assert_eq!(
//             cache.get("multisig", "test").await.unwrap(),
//             CacheValue::Address(address!("0xCEA8039076E35a825854c5C2f85659430b06ec96"))
//         );
//         // Verify manager and timelock are not set
//         assert!(cache.get_immediate("manager").await.is_err());
//         assert!(cache.get_immediate("timelock").await.is_err());
//     }

//     #[tokio::test]
//     async fn test_scenario_b_full_config() {
//         let json = json!(
//             [
//                 {
//                     "Global": {
//                         "boring_vault": "0xf0bb20865277aBd641a307eCe5Ee04E79073416C",
//                         "deployer": "0x5F2F11ad8656439d5C14d9B351f8b09cDaC2A02d",
//                         "network_id": 1,
//                         "roles_authority": "Test Roles Authority V0.0",
//                         "teller": "0x2222222222222222222222222222222222222222",
//                         "accountant": "0x3333333333333333333333333333333333333333",
//                         "manager": "0x4444444444444444444444444444444444444444",
//                         "multisig": "0x5555555555555555555555555555555555555555",
//                         "timelock": "0x6666666666666666666666666666666666666666"
//                     }
//                 }
//             ]
//         );

//         let mut manager = setup_block_manager(json).await;
//         manager.propogate_shared_data().await.unwrap();

//         // Verify all cache values
//         let cache = manager.cache;
//         assert_eq!(
//             cache.get("boring_vault", "test").await.unwrap(),
//             CacheValue::Address(address!("0xf0bb20865277aBd641a307eCe5Ee04E79073416C"))
//         );
//         let expected_roles_authority = derive_contract_address(
//             "Test Roles Authority V0.0",
//             address!("0x5F2F11ad8656439d5C14d9B351f8b09cDaC2A02d"),
//         );
//         assert_eq!(
//             cache.get("roles_authority", "test").await.unwrap(),
//             CacheValue::Address(expected_roles_authority)
//         );
//         // ... verify other addresses
//     }

//     #[tokio::test]
//     async fn test_scenario_c_conflicting_boring_vaults() {
//         let json = json!(
//             [
//                 {
//                     "Global": {
//                         "boring_vault": "0xf0bb20865277aBd641a307eCe5Ee04E79073416C",
//                         "deployer": "0x5F2F11ad8656439d5C14d9B351f8b09cDaC2A02d",
//                         "network_id": 1,
//                         "roles_authority": "0x1111111111111111111111111111111111111111",
//                         "teller": "0x2222222222222222222222222222222222222222",
//                         "accountant": "0x3333333333333333333333333333333333333333",
//                         "manager": "0x4444444444444444444444444444444444444444",
//                         "multisig": "0x5555555555555555555555555555555555555555",
//                         "timelock": "0x6666666666666666666666666666666666666666"
//                     }
//                 },
//                 {
//                     "Global": {
//                         "boring_vault": "0xAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
//                         "deployer": "0x5F2F11ad8656439d5C14d9B351f8b09cDaC2A02d",
//                         "network_id": 1,
//                         "roles_authority": "0x1111111111111111111111111111111111111111",
//                         "teller": "0x2222222222222222222222222222222222222222",
//                         "accountant": "0x3333333333333333333333333333333333333333",
//                         "manager": "0x4444444444444444444444444444444444444444",
//                         "multisig": "0x5555555555555555555555555555555555555555",
//                         "timelock": "0x6666666666666666666666666666666666666666"
//                     }
//                 }
//             ]
//         );

//         let mut manager = setup_block_manager(json).await;
//         let result = manager.propogate_shared_data().await;

//         // This should fail because of conflicting boring_vault addresses
//         assert!(result.is_err());
//         let err = result.unwrap_err();
//         println!("Error: {}", err);
//         assert!(err.to_string().contains("Cache value mismatch"));
//     }
// }
