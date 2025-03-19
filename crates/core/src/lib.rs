use GnosisSafe::GnosisSafeInstance;
use alloy::network::EthereumWallet;
use alloy::primitives::{Address, Bytes, FixedBytes, U256};
use alloy::providers::Provider;
use alloy::rpc::types::Filter;
use alloy::signers::ledger::{self, LedgerSigner};
use alloy::signers::trezor::{self, TrezorSigner};
use alloy::sol_types::SolEvent;
use alloy::{providers::ProviderBuilder, sol, sol_types::SolCall};
use dotenv::dotenv;
use eyre::{Result, eyre};
use hex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs};
use toml::Value;
use uuid::Uuid;

sol! {
    #[sol(rpc)]
    contract GnosisSafe {
        event ApproveHash(bytes32 indexed approvedHash, address indexed owner);
        function execTransactionFromModule(address to, uint256 value, bytes memory data, uint8 operation);
        function getTransactionHash(
            address to,
            uint256 value,
            bytes memory data,
            uint8 operation,
            uint256 safeTxGas,
            uint256 baseGas,
            uint256 gasPrice,
            address gasToken,
            address refundReceiver,
            uint256 _nonce
        ) public view returns (bytes32);
        function enableModule(address module) external;
        function approveHash(bytes32 safeHash) external;
        function getOwners() external view returns(address[] memory owners);
        function getThreshold() external view returns(uint256 threshold);
        function nonce() external view returns(uint256 nonce);
        function execTransaction(
            address to,
            uint256 value,
            bytes calldata data,
            uint8 operation,
            uint256 safeTxGas,
            uint256 baseGas,
            uint256 gasPrice,
            address gasToken,
            address payable refundReceiver,
            bytes memory signatures
        ) external;
    }
}

sol! {
    #[sol(rpc)]
    contract ManagerWithMerkleVerification {
        function setManageRoot(address strategist, bytes32 root) external;
    }
}

sol! {
    #[sol(rpc)]
    contract MutliSendCallOnly {
        function multiSend(bytes memory transactions) external;
    }
}

sol! {
    #[sol(rpc)]
    contract Timelock {
        function scheduleBatch(address[] memory targets, uint256[] memory values, bytes[] memory payloads, bytes32 predecessor, bytes32 salt, uint256 delay) external;
        function executeBatch(address[] memory targets, uint256[] memory values, bytes[] memory payloads, bytes32 predecessor, bytes32 salt) external;
        function getMinDelay() external view returns(uint256 delay);
    }
}

#[derive(Serialize, Deserialize)]
pub struct SimulationConfig {
    pub network_id: u32,
    pub multisig: String,
    pub to: String,
    pub value: String,
    pub data: String,
    pub operation: u8,
    pub nonce: u32,
}

fn read_simulation_config(file_path: &str) -> Result<SimulationConfig> {
    let file_content = fs::read_to_string(file_path)?;
    let config: SimulationConfig = serde_json::from_str(&file_content)?;
    Ok(config)
}

fn get_rpc_url(network_id: u32) -> Result<String> {
    let config_content = fs::read_to_string("config.toml")?;
    let config: Value = config_content.parse::<Value>()?;

    let url_value = &config["rpc_endpoints"][&network_id.to_string()];
    let url_str = url_value
        .as_str()
        .ok_or_else(|| eyre::eyre!("URL not found for network_id: {}", network_id))?;

    if url_str.starts_with("env:") {
        let env_var = &url_str[4..];
        env::var(env_var).map_err(|_| eyre::eyre!("Environment variable {} not set", env_var))
    } else {
        Ok(url_str.to_string())
    }
}

fn get_block_explorer_url(network_id: u32) -> Result<String> {
    let config_content = fs::read_to_string("config.toml")?;
    let config: Value = config_content.parse::<Value>()?;

    let url_value = &config["block_explorers"][&network_id.to_string()];
    let url_str = url_value.as_str().ok_or_else(|| {
        eyre::eyre!(
            "Block explorer URL not found for network_id: {}",
            network_id
        )
    })?;

    Ok(url_str.trim_end_matches('/').to_string())
}

pub async fn simulate_admin_tx_and_generate_safe_hash(
    admin_tx_path: &str,
) -> Result<(String, String)> {
    dotenv().ok(); // Load environment variables from .env file

    let api_key = env::var("TENDERLY_ACCESS_KEY")?;
    let account_slug = env::var("TENDERLY_ACCOUNT_SLUG")?;
    let project_slug = env::var("TENDERLY_PROJECT_SLUG")?;

    let config = read_simulation_config(admin_tx_path)?;

    // Call getTransactionHash
    let rpc_url = get_rpc_url(config.network_id)?;
    let provider = ProviderBuilder::new().on_builtin(&rpc_url).await?;
    let safe_address = config.multisig.parse().expect("Failed to parse to");
    let safe = GnosisSafe::new(safe_address, provider);

    let (safe_hash, to_address, value, data, operation) =
        generate_safe_hash_and_return_params(&safe, &config).await?;

    let from_address = "0xe2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2";
    let state_objects = json!({
        config.multisig.clone(): {
            "storage": {
                "0xd71a90a935e1abe19645d4f9630a0044413a815e634f2ca5c4b4b04becfec14c": "0x0000000000000000000000000000000000000000000000000000000000000001"
            }
        }
    });

    let client = Client::new();

    // Build input.
    let input =
        GnosisSafe::execTransactionFromModuleCall::new((to_address, value, data, operation))
            .abi_encode();

    let input_hex = hex::encode(input);

    let response = client
        .post(&format!(
            "https://api.tenderly.co/api/v1/account/{}/project/{}/simulate",
            account_slug, project_slug
        ))
        .header("X-Access-Key", api_key)
        .json(&json!({
            "save": true,
            "save_if_fails": true,
            "simulation_type": "full",
            "network_id": config.network_id,
            "from": from_address,
            "to": config.multisig,
            "input": input_hex,
            "gas": 10_000_000,
            "state_objects": state_objects,
        }))
        .send()
        .await?;

    let simulation_result = response.json::<serde_json::Value>().await?;

    let simulation_url = simulation_result
        .get("simulation")
        .and_then(|sim| sim.get("id"))
        .and_then(|id| id.as_str())
        .map(|simulation_id| {
            format!(
                "https://dashboard.tenderly.co/{}/{}/simulator/{}",
                account_slug, project_slug, simulation_id
            )
        })
        .ok_or_else(|| eyre::eyre!("Simulation ID not found in response"))?;

    Ok((simulation_url, safe_hash))
}

async fn generate_safe_hash_and_return_params(
    safe: &GnosisSafeInstance<
        (),
        alloy::providers::fillers::FillProvider<
            alloy::providers::fillers::JoinFill<
                alloy::providers::Identity,
                alloy::providers::fillers::JoinFill<
                    alloy::providers::fillers::GasFiller,
                    alloy::providers::fillers::JoinFill<
                        alloy::providers::fillers::BlobGasFiller,
                        alloy::providers::fillers::JoinFill<
                            alloy::providers::fillers::NonceFiller,
                            alloy::providers::fillers::ChainIdFiller,
                        >,
                    >,
                >,
            >,
            alloy::providers::RootProvider,
        >,
    >,
    config: &SimulationConfig,
) -> Result<(String, Address, U256, Bytes, u8)> {
    let safe_tx_gas = U256::ZERO;
    let base_gas = U256::ZERO;
    let gas_price = U256::ZERO;
    let gas_token = Address::ZERO;
    let refund_receiver = Address::ZERO;

    let to_address: Address = config.to.parse().expect("Failed to parse to");
    let value = U256::from(config.value.parse::<U256>().expect("Failed to parse value"));
    let data = Bytes::from(config.data.parse::<Bytes>().expect("Failed to parse data"));
    let operation = config.operation;

    let safe_hash = safe
        .getTransactionHash(
            to_address,
            value,
            data.clone(),
            operation,
            safe_tx_gas,
            base_gas,
            gas_price,
            gas_token,
            refund_receiver,
            U256::from(config.nonce),
        )
        .call()
        .await?
        ._0;

    Ok((
        format!("0x{}", hex::encode(safe_hash)),
        to_address,
        value,
        data,
        operation,
    ))
}

pub async fn simulate_timelock_admin_txs_and_generate_safe_hashes(
    propose_tx_path: String,
    execute_tx_path: String,
) -> Result<(String, String, String)> {
    dotenv().ok();

    let api_key = env::var("TENDERLY_ACCESS_KEY")?;
    let account_slug = env::var("TENDERLY_ACCOUNT_SLUG")?;
    let project_slug = env::var("TENDERLY_PROJECT_SLUG")?;

    let propose_config = read_simulation_config(&propose_tx_path)?;
    let execute_config = read_simulation_config(&execute_tx_path)?;

    // Validate matching fields between propose and execute configs
    if propose_config.network_id != execute_config.network_id {
        return Err(eyre::eyre!(
            "Network IDs do not match: propose={}, execute={}",
            propose_config.network_id,
            execute_config.network_id
        ));
    }
    if propose_config.to != execute_config.to {
        return Err(eyre::eyre!(
            "Target addresses do not match: propose={}, execute={}",
            propose_config.to,
            execute_config.to
        ));
    }
    if propose_config.operation != execute_config.operation {
        return Err(eyre::eyre!(
            "Operations do not match: propose={}, execute={}",
            propose_config.operation,
            execute_config.operation
        ));
    }

    let from_address = "0xe2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2";

    let vnet_slug = format!("vnet-{}", Uuid::new_v4());
    let client = Client::new();

    let create_vnet_response = client
        .post(&format!(
            "https://api.tenderly.co/api/v1/account/{}/project/{}/vnets",
            account_slug, project_slug
        ))
        .header("X-Access-Key", api_key.clone())
        .json(&json!({
            "slug": vnet_slug,
            "fork_config": {
                "network_id": propose_config.network_id
            },
            "virtual_network_config": {
                "chain_config": {
                    "chain_id": propose_config.network_id
                }
            }
        }))
        .send()
        .await?;

    let create_vnet_response_json = create_vnet_response.json::<serde_json::Value>().await?;

    let vnet_id = create_vnet_response_json
        .get("id")
        .and_then(|id| id.as_str())
        .ok_or_else(|| eyre::eyre!("Vnet ID not found in response"))?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Generate Safe Hash
    let rpc_url = get_rpc_url(propose_config.network_id)?;
    let provider = ProviderBuilder::new().on_builtin(&rpc_url).await?;
    let safe_address = propose_config.multisig.parse().expect("Failed to parse to");
    let safe = GnosisSafe::new(safe_address, provider);

    let (propose_safe_hash_hex, to_address, value, data, operation) =
        generate_safe_hash_and_return_params(&safe, &propose_config).await?;

    // Build input.
    let input =
        GnosisSafe::execTransactionFromModuleCall::new((to_address, value, data, operation))
            .abi_encode();

    let input_hex = hex::encode(input);

    client
        .post(&format!(
            "https://api.tenderly.co/api/v1/account/{}/project/{}/vnets/{}/transactions",
            account_slug, project_slug, vnet_id
        ))
        .header("X-Access-Key", api_key.clone())
        .json(&json!({
            "callArgs": {
                "from": from_address,
                "to": propose_config.multisig,
                "gas": format!("0x{:x}", 10_000_000),
                "gasPrice": "0x0",
                "value": "0x0",
                "data": format!("0x{}", input_hex)
            },
            "blockOverrides": {
              "time": format!("0x{:x}", timestamp + 1)
            },
            "stateOverrides": {
                propose_config.multisig.clone(): {
                    "stateDiff": {
                        "0xd71a90a935e1abe19645d4f9630a0044413a815e634f2ca5c4b4b04becfec14c": "0x0000000000000000000000000000000000000000000000000000000000000001"
                    }
                }
            }
        }))
        .send()
        .await?;

    let (execute_safe_hash_hex, to_address, value, data, operation) =
        generate_safe_hash_and_return_params(&safe, &execute_config).await?;

    // Build input.
    let input =
        GnosisSafe::execTransactionFromModuleCall::new((to_address, value, data, operation))
            .abi_encode();

    let input_hex = hex::encode(input);

    let _response = client
        .post(&format!(
            "https://api.tenderly.co/api/v1/account/{}/project/{}/vnets/{}/transactions",
            account_slug, project_slug, vnet_id
        ))
        .header("X-Access-Key", api_key.clone())
        .json(&json!({
            "callArgs": {
                "from": from_address,
                "to": execute_config.multisig,
                "gas": format!("0x{:x}", 10_000_000),
                "gasPrice": "0x0",
                "value": "0x0",
                "data": format!("0x{}", input_hex)
            },
            "blockOverrides": {
              "time": format!("0x{:x}", timestamp + 30 * 86_400)
            },
            "stateOverrides": {
                execute_config.multisig.clone(): {
                    "stateDiff": {
                        "0xd71a90a935e1abe19645d4f9630a0044413a815e634f2ca5c4b4b04becfec14c": "0x0000000000000000000000000000000000000000000000000000000000000001"
                    }
                }
            }
        }))
        .send()
        .await?;

    // NOTE for debugging
    // let response_json = response.json::<serde_json::Value>().await?;
    // fs::write("tmp.json", serde_json::to_string_pretty(&response_json)?)?;

    let vnet_url = format!(
        "https://dashboard.tenderly.co/{}/{}/testnet/{}",
        account_slug, project_slug, vnet_id
    );

    Ok((vnet_url, propose_safe_hash_hex, execute_safe_hash_hex))
}

fn get_multisend_address(network_id: u32) -> Result<String> {
    let config_content = fs::read_to_string("config.toml")?;
    let config: Value = config_content.parse::<Value>()?;

    // Try network specific value first
    let network_value = config
        .get("multi_send_address")
        .and_then(|m| m.get(&network_id.to_string()))
        .and_then(|m| m.as_str());

    // Fallback to default if network specific not found
    let default_value = config
        .get("multi_send_address")
        .and_then(|m| m.get("default"))
        .and_then(|m| m.as_str());

    let address_str = network_value
        .or(default_value)
        .ok_or_else(|| eyre::eyre!("Multisend address not found for network_id: {}", network_id))?;

    Ok(address_str.to_string())
}

pub async fn generate_root_update_txs(
    root_str: &String,
    product_name: &str,
    network_id: u32,
    nonce: u32,
) -> Result<(Vec<SimulationConfig>, Vec<String>)> {
    // Trim "0x" prefix if present
    let root_str = root_str.trim_start_matches("0x");
    // Convert hex string to Vec<u8>
    let root_bytes = hex::decode(root_str)?;
    // Convert to fixed-size bytes
    let new_root = FixedBytes::<32>::from_slice(&root_bytes);

    // Read and parse config.toml
    let config_content = fs::read_to_string("config.toml")?;
    let config: Value = config_content.parse::<Value>()?;

    // Helper function to get config value from product section
    let get_product_value = |key: &str| -> Result<Value> {
        // Try network specific value first
        let network_value = config
            .get("product")
            .and_then(|p| p.get(product_name))
            .and_then(|p| p.get(&network_id.to_string()))
            .and_then(|p| p.get(key));

        // Fallback to default if network specific not found
        let default_value = config
            .get("product")
            .and_then(|p| p.get(product_name))
            .and_then(|p| p.get("default"))
            .and_then(|p| p.get(key));

        if key == "timelock_address" {
            Ok(network_value.or(default_value).cloned().unwrap_or_else(|| {
                Value::String("0x0000000000000000000000000000000000000000".to_string())
            }))
        } else {
            network_value
                .or(default_value)
                .ok_or_else(|| eyre::eyre!("Config value not found for key: {}", key))
                .map(|v| v.clone())
        }
    };

    // Get required addresses from config
    let strategists_value = get_product_value("strategists")?;
    let strategists = strategists_value
        .as_array()
        .ok_or_else(|| eyre::eyre!("Strategists must be an array"))?;

    if strategists.is_empty() {
        return Err(eyre::eyre!("Strategists array cannot be empty"));
    }

    let manager_value = get_product_value("manager_address")?;
    let manager_address = manager_value
        .as_str()
        .ok_or_else(|| eyre::eyre!("Manager address must be a string"))?;

    let multisig_value = get_product_value("multisig_address")?;
    let multisig_address = multisig_value
        .as_str()
        .ok_or_else(|| eyre::eyre!("Multisig address must be a string"))?;

    let timelock_value = get_product_value("timelock_address")?;
    let timelock_address = timelock_value
        .as_str()
        .ok_or_else(|| eyre::eyre!("Timelock address must be a string"))?;

    // Parse addresses
    let manager_addr: Address = manager_address.parse()?;
    let timelock_addr: Address = timelock_address.parse()?;

    let mut txs = Vec::new();
    if timelock_addr != Address::ZERO {
        // Load env variables
        dotenv().ok();

        // Read the min delay.
        let rpc_url = get_rpc_url(network_id)?;
        let provider = ProviderBuilder::new().on_builtin(&rpc_url).await?;
        let timelock = Timelock::new(timelock_addr, provider);
        let min_delay = timelock.getMinDelay().call().await?.delay;

        let mut targets = Vec::with_capacity(strategists.len());
        let mut values = Vec::with_capacity(strategists.len());
        let mut data = Vec::with_capacity(strategists.len());
        let predecessor = FixedBytes::<32>::ZERO;
        let salt = FixedBytes::<32>::ZERO;

        for strategist in strategists {
            targets.push(manager_addr);
            values.push(U256::ZERO);
            let strategist_address = strategist
                .as_str()
                .ok_or_else(|| eyre::eyre!("Strategist address must be a string"))?;
            let strategist_addr = strategist_address.parse()?;
            let bytes_data =
                ManagerWithMerkleVerification::setManageRootCall::new((strategist_addr, new_root))
                    .abi_encode();
            data.push(Bytes::from(bytes_data));
        }

        // We need to propose 2 txs, a schedule and execute.
        let propose_batch_tx_data = Timelock::scheduleBatchCall::new((
            targets.clone(),
            values.clone(),
            data.clone(),
            predecessor,
            salt,
            min_delay,
        ))
        .abi_encode();
        let execute_batch_tx_data =
            Timelock::executeBatchCall::new((targets, values, data, predecessor, salt))
                .abi_encode();
        txs.push(SimulationConfig {
            network_id,
            multisig: multisig_address.to_string(),
            to: timelock_address.to_string(),
            value: "0".to_string(),
            data: format!("0x{}", hex::encode(propose_batch_tx_data)),
            operation: 0,
            nonce,
        });
        txs.push(SimulationConfig {
            network_id,
            multisig: multisig_address.to_string(),
            to: timelock_address.to_string(),
            value: "0".to_string(),
            data: format!("0x{}", hex::encode(execute_batch_tx_data)),
            operation: 0,
            nonce: nonce + 1, // Advance nonce by 1
        });
    } else {
        if strategists.len() == 1 {
            // No need to use MultiSend, make call directly to manager.
            let strategist_address = strategists[0]
                .as_str()
                .ok_or_else(|| eyre::eyre!("Strategist address must be a string"))?;
            let strategist_addr = strategist_address.parse()?;
            let bytes_data =
                ManagerWithMerkleVerification::setManageRootCall::new((strategist_addr, new_root))
                    .abi_encode();
            txs.push(SimulationConfig {
                network_id,
                multisig: multisig_address.to_string(),
                to: manager_address.to_string(),
                value: "0".to_string(),
                data: format!("0x{}", hex::encode(bytes_data)),
                operation: 0,
                nonce,
            });
        } else {
            // Need to use MultiSend contract.
            let mut targets = Vec::with_capacity(strategists.len());
            let mut values = Vec::with_capacity(strategists.len());
            let mut data = Vec::with_capacity(strategists.len());
            for strategist in strategists {
                let strategist_address = strategist
                    .as_str()
                    .ok_or_else(|| eyre::eyre!("Strategist address must be a string"))?;
                let strategist_addr = strategist_address.parse()?;
                targets.push(manager_addr);
                values.push(U256::ZERO);
                data.push(
                    ManagerWithMerkleVerification::setManageRootCall::new((
                        strategist_addr,
                        new_root,
                    ))
                    .abi_encode(),
                );
            }

            let mut encoded_transactions = Vec::new();

            for i in 0..targets.len() {
                // operation (0 for Call) - 1 byte
                encoded_transactions.push(0u8);

                // to address - 20 bytes
                encoded_transactions.extend_from_slice(&targets[i].as_slice());

                // value - 32 bytes
                encoded_transactions.extend_from_slice(&values[i].to_be_bytes::<32>());

                // data length - 32 bytes
                let data_len = U256::from(data[i].len());
                encoded_transactions.extend_from_slice(&data_len.to_be_bytes::<32>());

                // data - dynamic length
                encoded_transactions.extend_from_slice(&data[i]);
            }

            // Create the final transaction
            let multisend_data =
                MutliSendCallOnly::multiSendCall::new((Bytes::from(encoded_transactions),))
                    .abi_encode();

            txs.push(SimulationConfig {
                network_id,
                multisig: multisig_address.to_string(),
                to: get_multisend_address(network_id)?,
                value: "0".to_string(),
                data: format!("0x{}", hex::encode(multisend_data)),
                operation: 1,
                nonce,
            });
        }
    }

    let strategists = strategists.into_iter().map(|strategist| strategist.as_str().unwrap().to_string()).collect();

    Ok((txs, strategists))
}

pub enum HardwareWalletType {
    TREZOR,
    LEDGER,
}

pub async fn approve_hash(admin_tx_path: &str, wallet_type: HardwareWalletType) -> Result<String> {
    dotenv().ok(); // Load environment variables from .env file

    let config = read_simulation_config(admin_tx_path)?;
    let derivation_path = env::var("DERIVATION_PATH")?;
    let wallet;
    let signer_addr: Address;

    match wallet_type {
        HardwareWalletType::TREZOR => {
            let signer = TrezorSigner::new(
                trezor::HDPath::Other(derivation_path),
                Some(config.network_id as u64),
            )
            .await?;
            signer_addr = signer.get_address().await?;
            wallet = EthereumWallet::from(signer);
        }
        HardwareWalletType::LEDGER => {
            let signer = LedgerSigner::new(
                ledger::HDPath::Other(derivation_path),
                Some(config.network_id as u64),
            )
            .await?;
            signer_addr = signer.get_address().await?;
            let wallet_signer = signer;
            wallet = EthereumWallet::from(wallet_signer);
        }
    }

    // Call getTransactionHash
    let rpc_url = get_rpc_url(config.network_id)?;
    let provider = ProviderBuilder::new().on_builtin(&rpc_url).await?;
    let safe_address = config.multisig.parse().expect("Failed to parse to");
    let safe = GnosisSafe::new(safe_address, provider.clone());

    let owners = safe.getOwners().call().await?.owners;

    if !owners.contains(&signer_addr) {
        return Err(eyre::eyre!(
            "Signer address {} is not an owner of the Safe {}",
            signer_addr,
            safe_address
        ));
    }

    let (safe_hash_str, _to_address, _value, _data, _operation) =
        generate_safe_hash_and_return_params(&safe, &config).await?;

    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .on_http(rpc_url.parse()?);
    let safe = GnosisSafe::new(safe_address, provider.clone());

    // Trim "0x" prefix if present
    let safe_hash_str = safe_hash_str.trim_start_matches("0x");
    // Convert hex string to Vec<u8>
    let safe_hash_bytes = hex::decode(safe_hash_str)?;
    // Convert to fixed-size bytes
    let safe_hash = FixedBytes::<32>::from_slice(&safe_hash_bytes);

    let approve_hash_tx_request = safe.approveHash(safe_hash).into_transaction_request();

    // println!("Signer Address: {}", signer.get_address().await?);
    // Print verification info
    let selector = "0xd4d9bdcd"; // approveHash selector
    println!("Please verify the following on your hardware wallet:");
    println!("approveHash Selector: {}", selector);
    println!("Safe Hash: 0x{}", safe_hash_str);
    println!("Data: {}{}", selector, safe_hash_str);
    println!("Recipient: {}", config.multisig);

    let tx_hash = provider
        .send_transaction(approve_hash_tx_request)
        .await?
        .with_required_confirmations(3)
        .watch()
        .await?;

    let block_explorer_url = get_block_explorer_url(config.network_id)?;
    let tx_hash_hex = hex::encode(tx_hash.as_slice());
    Ok(format!("{}/tx/0x{}", block_explorer_url, tx_hash_hex))
}

pub async fn exec_transaction(
    admin_tx_path: &str,
    wallet_type: HardwareWalletType,
) -> Result<String> {
    dotenv().ok(); // Load environment variables from .env file

    let api_key = env::var("TENDERLY_ACCESS_KEY")?;
    let account_slug = env::var("TENDERLY_ACCOUNT_SLUG")?;
    let project_slug = env::var("TENDERLY_PROJECT_SLUG")?;

    let config = read_simulation_config(admin_tx_path)?;
    let derivation_path = env::var("DERIVATION_PATH")?;
    let wallet;
    let signer_addr: Address;

    match wallet_type {
        HardwareWalletType::TREZOR => {
            let signer = TrezorSigner::new(
                trezor::HDPath::Other(derivation_path),
                Some(config.network_id as u64),
            )
            .await?;
            signer_addr = signer.get_address().await?;
            wallet = EthereumWallet::from(signer);
        }
        HardwareWalletType::LEDGER => {
            let signer = LedgerSigner::new(
                ledger::HDPath::Other(derivation_path),
                Some(config.network_id as u64),
            )
            .await?;
            signer_addr = signer.get_address().await?;
            let wallet_signer = signer;
            wallet = EthereumWallet::from(wallet_signer);
        }
    }

    // Call getTransactionHash
    let rpc_url = get_rpc_url(config.network_id)?;
    let provider = ProviderBuilder::new().on_builtin(&rpc_url).await?;
    let safe_address = config.multisig.parse().expect("Failed to parse to");
    let safe = GnosisSafe::new(safe_address, provider.clone());

    let (safe_hash_str, _to_address, _value, _data, _operation) =
        generate_safe_hash_and_return_params(&safe, &config).await?;

    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .on_http(rpc_url.parse()?);
    let safe = GnosisSafe::new(safe_address, provider.clone());

    // Trim "0x" prefix if present
    let safe_hash_str = safe_hash_str.trim_start_matches("0x");
    // Convert hex string to Vec<u8>
    let safe_hash_bytes = hex::decode(safe_hash_str)?;
    // Convert to fixed-size bytes
    let safe_hash = FixedBytes::<32>::from_slice(&safe_hash_bytes);

    println!("Safe Hash: {}", safe_hash);

    // Get the nonce.
    let nonce = safe.nonce().call().await?.nonce.to::<u32>();

    if nonce != config.nonce {
        return Err(eyre!(
            "Transaction nonce({}) does not match safe nonce({})",
            config.nonce,
            nonce
        ));
    }

    // Get the threshold.
    let threshold = safe.getThreshold().call().await?.threshold;

    let latest_block = provider.get_block_number().await?;
    let filter = Filter::new()
        .address(safe_address)
        .event_signature(GnosisSafe::ApproveHash::SIGNATURE_HASH)
        .topic1(safe_hash)
        .from_block(0)
        .to_block(latest_block);
    // .from_block(latest_block);

    let logs = provider.get_logs(&filter).await?;

    println!("Found {} log(s)", logs.len());

    // Convert logs into Vec<Address>, parse the owner address from topic2
    let mut approvers: Vec<Address> = logs
        .iter()
        .map(|log| {
            let topic_bytes = log.topics()[2].as_slice();
            Address::from_slice(&topic_bytes[12..]) // Convert last 20 bytes to Address
        })
        .collect();

    // Sort addresses in ascending order
    approvers.sort();

    // Deduplicate in case an owner approved multiple times
    approvers.dedup();

    let threshold = threshold.to::<usize>();
    if approvers.len() > threshold {
        // Take only what we need for threshold
        approvers.truncate(threshold);
    } else if approvers.len() < threshold {
        let owners = safe.getOwners().call().await?.owners;
        // Find which owners haven't approved yet
        let remaining_needed = threshold - approvers.len();
        let mut available_signers: Vec<Address> = owners
            .iter()
            .filter(|owner| !approvers.contains(owner))
            .copied()
            .collect();
        
        // Sort by address for consistent output
        available_signers.sort();

        println!("\nNeed {} more signature(s) from:", remaining_needed);
        for (i, owner) in available_signers.iter().enumerate() {
            println!("{}. {}", i + 1, owner);
        }

        return Err(eyre!(
            "Not enough signers, have {}, need {}",
            approvers.len(),
            threshold
        ));
    }

    // Create signatures bytes
    let mut signatures = Vec::new();
    for approver in approvers {
        // r: 32 bytes - padded address
        signatures.extend_from_slice(approver.into_word().as_slice());

        // s: 32 bytes - all zeros
        signatures.extend_from_slice(&[0u8; 32]);

        // v: 1 byte - always 1
        signatures.push(1);
    }

    let safe_tx_gas = U256::ZERO;
    let base_gas = U256::ZERO;
    let gas_price = U256::ZERO;
    let gas_token = Address::ZERO;
    let refund_receiver = Address::ZERO;

    let to_address: Address = config.to.parse().expect("Failed to parse to");
    let value = U256::from(config.value.parse::<U256>().expect("Failed to parse value"));
    let data = Bytes::from(config.data.parse::<Bytes>().expect("Failed to parse data"));
    let operation = config.operation;

    // Simulate the tx using tenderly.
    let client = Client::new();

    // Build input.
    let input = GnosisSafe::execTransactionCall::new((
        to_address,
        value,
        data.clone(),
        operation,
        safe_tx_gas,
        base_gas,
        gas_price,
        gas_token,
        refund_receiver,
        Bytes::from(signatures.clone()),
    ))
    .abi_encode();

    let input_hex = hex::encode(input);

    let response = client
        .post(&format!(
            "https://api.tenderly.co/api/v1/account/{}/project/{}/simulate",
            account_slug, project_slug
        ))
        .header("X-Access-Key", api_key)
        .json(&json!({
            "save": true,
            "save_if_fails": true,
            "simulation_type": "full",
            "network_id": config.network_id,
            "from": signer_addr,
            "to": config.multisig,
            "input": input_hex,
            "gas": 10_000_000,
        }))
        .send()
        .await?;

    let simulation_result = response.json::<serde_json::Value>().await?;

    let simulation_url = simulation_result
        .get("simulation")
        .and_then(|sim| sim.get("id"))
        .and_then(|id| id.as_str())
        .map(|simulation_id| {
            format!(
                "https://dashboard.tenderly.co/{}/{}/simulator/{}",
                account_slug, project_slug, simulation_id
            )
        })
        .ok_or_else(|| eyre::eyre!("Simulation ID not found in response"))?;

    println!("Simulation url: {}", simulation_url);

    let exec_transaction_tx_request = safe
        .execTransaction(
            to_address,
            value,
            data,
            operation,
            safe_tx_gas,
            base_gas,
            gas_price,
            gas_token,
            refund_receiver,
            Bytes::from(signatures),
        )
        .into_transaction_request();

    let tx_hash = provider
        .send_transaction(exec_transaction_tx_request)
        .await?
        .with_required_confirmations(3)
        .watch()
        .await?;

    let block_explorer_url = get_block_explorer_url(config.network_id)?;
    let tx_hash_hex = hex::encode(tx_hash.as_slice());
    Ok(format!("{}/tx/0x{}", block_explorer_url, tx_hash_hex))
}

pub fn generate_notion_markdown(
    title: &str,
    safe_hash: &str,
    tx_data: &serde_json::Value,
    root_info: Option<(&str, &[&str])>, // (root_hash, strategist_addresses)
) -> String {
    format!(
        r#"- {title}

SafeTxHash: `{safe_hash}`

Proposal TXN: Link
<details>
<summary>Transaction Details</summary>
         ```json
        {}
         ```
</details>

        {}

        Diff: here

"#,
        serde_json::to_string_pretty(tx_data).unwrap(),
        if let Some((root, addresses)) = root_info {
            format!(
                "- Set Root: `{}` for `{}` and `{}`",
                root, addresses[0], addresses[1]
            )
        } else {
            String::new()
        }
    )
}
