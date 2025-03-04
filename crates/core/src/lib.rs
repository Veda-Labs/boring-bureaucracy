use alloy::primitives::{Address, Bytes, FixedBytes, U256};
use alloy::{providers::ProviderBuilder, sol, sol_types::SolCall};
use dotenv::dotenv;
use eyre::Result;
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
        function execTransactionFromModule(address to, uint256 value, bytes memory data, uint8 operation);
        function getTransactionHash(
            address to,
            uint256 value,
            bytes calldata data,
            uint8 operation,
            uint256 safeTxGas,
            uint256 baseGas,
            uint256 gasPrice,
            address gasToken,
            address refundReceiver,
            uint256 _nonce
        ) public view returns (bytes32);
        function enableModule(address module) external;
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

pub async fn simulate_admin_tx_and_generate_safe_hash(
    admin_tx_path: &str,
) -> Result<(String, String)> {
    dotenv().ok(); // Load environment variables from .env file

    let api_key = env::var("TENDERLY_ACCESS_KEY")?;
    let account_slug = env::var("TENDERLY_ACCOUNT_SLUG")?;
    let project_slug = env::var("TENDERLY_PROJECT_SLUG")?;

    let config = read_simulation_config(admin_tx_path)?;
    let rpc_url = get_rpc_url(config.network_id)?;

    // Calculate safe hash
    let safe_tx_gas = U256::ZERO;
    let base_gas = U256::ZERO;
    let gas_price = U256::ZERO;
    let gas_token = Address::ZERO;
    let refund_receiver = Address::ZERO;

    let safe_address = config.multisig.parse().expect("Failed to parse to");
    let to_address = config.to.parse().expect("Failed to parse to");
    let value = U256::from(config.value.parse::<U256>().expect("Failed to parse value"));
    let data = Bytes::from(config.data.parse::<Bytes>().expect("Failed to parse data"));
    let operation = config.operation;

    // Call getTransactionHash
    let provider = ProviderBuilder::new().on_builtin(&rpc_url).await?;

    let safe = GnosisSafe::new(safe_address, provider);

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
        GnosisSafe::execTransactionFromModuleCall::new((to_address, value, data, config.operation))
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

    let safe_hash_hex = format!("0x{}", hex::encode(safe_hash));
    Ok((simulation_url, safe_hash_hex))
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
    fs::write(
        "create_vnet.json",
        serde_json::to_string_pretty(&create_vnet_response_json)?,
    )?;

    let vnet_id = create_vnet_response_json
        .get("id")
        .and_then(|id| id.as_str())
        .ok_or_else(|| eyre::eyre!("Vnet ID not found in response"))?;

    // Build input.
    let from_addr: Address = from_address.parse()?;
    let input = GnosisSafe::enableModuleCall::new((from_addr,)).abi_encode();
    let input_hex = hex::encode(input);

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Call enable module specifying from address as a module.
    client
        .post(&format!(
            "https://api.tenderly.co/api/v1/account/{}/project/{}/vnets/{}/transactions",
            account_slug, project_slug, vnet_id
        ))
        .header("X-Access-Key", api_key.clone())
        .json(&json!({
            "callArgs": {
                "from": propose_config.multisig,
                "to": propose_config.multisig,
                "gas": format!("0x{:x}", 10_000_000),
                "gasPrice": "0x0",
                "value": "0x0",
                "data": format!("0x{}", input_hex)
            },
            "blockOverrides": {
              "time": format!("0x{:x}", timestamp)
            }
        }))
        .send()
        .await?;

    let to_address: Address = propose_config.to.parse().expect("Failed to parse to");
    let value = U256::from(
        propose_config
            .value
            .parse::<U256>()
            .expect("Failed to parse value"),
    );
    let data = Bytes::from(
        propose_config
            .data
            .parse::<Bytes>()
            .expect("Failed to parse data"),
    );
    let operation = propose_config.operation;

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
            }
        }))
        .send()
        .await?;

    let to_address: Address = execute_config.to.parse().expect("Failed to parse to");
    let value = U256::from(
        execute_config
            .value
            .parse::<U256>()
            .expect("Failed to parse value"),
    );
    let data = Bytes::from(
        execute_config
            .data
            .parse::<Bytes>()
            .expect("Failed to parse data"),
    );
    let operation = execute_config.operation;

    // Build input.
    let input =
        GnosisSafe::execTransactionFromModuleCall::new((to_address, value, data, operation))
            .abi_encode();

    let input_hex = hex::encode(input);

    let send_execute_tx_response = client
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
            }
        }))
        .send()
        .await?;

    let send_execute_tx_response_json =
        send_execute_tx_response.json::<serde_json::Value>().await?;

    fs::write(
        "execute_tx.json",
        serde_json::to_string_pretty(&send_execute_tx_response_json)?,
    )?;

    Ok(("".to_string(), "".to_string(), "".to_string()))
}

pub async fn generate_root_update_tx(
    new_root: FixedBytes<32>,
    product_name: &str,
    network_id: u32,
    nonce: u32,
) -> Result<Vec<SimulationConfig>> {
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

        network_value
            .or(default_value)
            .ok_or_else(|| eyre::eyre!("Config value not found for key: {}", key))
            .map(|v| v.clone())
    };

    // Get required addresses from config
    let strategists_value = get_product_value("strategists")?;
    let strategists = strategists_value
        .as_array()
        .ok_or_else(|| eyre::eyre!("Strategists must be an array"))?;

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

        for (i, strategist) in strategists.iter().enumerate() {
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
            todo!()
            // txs.push(SimulationConfig {
            //     network_id,
            //     multisig: multisig_address.to_string(),
            //     to: manager_address.to_string(),
            //     value: "0".to_string(),
            //     data: "0x".to_string(),
            //     operation: 0,
            //     nonce,
            // });
        } else {
            // Need to use MultiSend contract
            todo!()
        }
    }

    Ok(txs)
}
