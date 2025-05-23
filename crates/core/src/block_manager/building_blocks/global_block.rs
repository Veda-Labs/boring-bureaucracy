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
use building_block_derive::BuildingBlockCache;
use eyre::Result;
use serde::Deserialize;

#[derive(BuildingBlockCache, Debug, Deserialize)]
pub struct GlobalBlock {
    #[serde(default)]
    pub deployer: Option<Address>,
    #[serde(default)]
    pub network_id: Option<u32>,
    #[serde(default)]
    pub boring_vault: Option<AddressOrContractName>,
    #[serde(default)]
    #[can_derive]
    pub roles_authority: Option<AddressOrContractName>,
    #[serde(default)]
    #[can_derive]
    pub teller: Option<AddressOrContractName>,
    #[serde(default)]
    #[can_derive]
    pub accountant: Option<AddressOrContractName>,
    #[serde(default)]
    pub manager: Option<AddressOrContractName>,
    #[serde(default)]
    #[can_derive]
    pub multisig: Option<AddressOrContractName>,
    #[serde(default)]
    #[can_derive]
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
    async fn _assemble(
        &self,
        _cache: &SharedCache,
        _vrm: &ViewRequestManager,
    ) -> Result<Vec<Box<dyn Action>>> {
        Ok(vec![])
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
