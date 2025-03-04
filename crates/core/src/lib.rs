use alloy::primitives::{Address, Bytes, U256};
use alloy::{providers::ProviderBuilder, sol, sol_types::SolCall};
use dotenv::dotenv;
use eyre::Result;
use hex;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::{env, fs};
use toml::Value;

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
    }
}

#[derive(Deserialize)]
struct SimulationConfig {
    network_id: String,
    multisig: String,
    to: String,
    value: String,
    data: String,
    operation: String,
    nonce: u64,
}

fn read_simulation_config(file_path: &str) -> Result<SimulationConfig> {
    let file_content = fs::read_to_string(file_path)?;
    let config: SimulationConfig = serde_json::from_str(&file_content)?;
    Ok(config)
}

fn get_rpc_url(network_id: &str) -> Result<String> {
    let config_content = fs::read_to_string("config.toml")?;
    let config: Value = config_content.parse::<Value>()?;

    let url_value = &config["rpc_endpoints"][network_id];
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

pub async fn run_simulation() -> Result<()> {
    dotenv().ok(); // Load environment variables from .env file

    let api_key = env::var("TENDERLY_ACCESS_KEY")?;
    let account_slug = env::var("TENDERLY_ACCOUNT_SLUG")?;
    let project_slug = env::var("TENDERLY_PROJECT_SLUG")?;

    let config = read_simulation_config("admin_tx.json")?;
    let rpc_url = get_rpc_url(&config.network_id)?;

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
    let operation = config.operation.parse().expect("Failed to parse operation");

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

    println!("Safe Hash: {}", safe_hash);

    let from_address = "0xe2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2";
    // TOOD should chagne state of config.multisig
    let state_objects = json!({
        config.multisig.clone(): {
            "storage": {
                "0xd71a90a935e1abe19645d4f9630a0044413a815e634f2ca5c4b4b04becfec14c": "0x0000000000000000000000000000000000000000000000000000000000000001"
            }
        }
    });

    let client = Client::new();

    // Build input.
    let input = GnosisSafe::execTransactionFromModuleCall::new((
        to_address,
        value,
        data,
        config.operation.parse().expect("Failed to parse operation"),
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
            "from": from_address,
            "to": config.multisig,
            "input": input_hex,
            "gas": 10_000_000,
            "state_objects": state_objects,
        }))
        .send()
        .await?;

    let simulation_result = response.json::<serde_json::Value>().await?;

    if let Some(simulation_id) = simulation_result
        .get("simulation")
        .and_then(|sim| sim.get("id"))
        .and_then(|id| id.as_str())
    {
        let simulation_url = format!(
            "https://dashboard.tenderly.co/{}/{}/simulator/{}",
            account_slug, project_slug, simulation_id
        );
        println!("Simulation URL: {}", simulation_url);
    } else {
        println!("Simulation ID not found in response.");
    }

    Ok(())
}
