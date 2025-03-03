use alloy::primitives::{Bytes, U256};
use alloy::{sol, sol_types::SolCall};
use dotenv::dotenv;
use eyre::Result;
use hex;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::{env, fs};

sol! {
    #[sol(rpc)]
    contract GnosisSafe {
        function execTransactionFromModule(address to, uint256 value, bytes memory data, uint8 operation);
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
}

fn read_simulation_config(file_path: &str) -> Result<SimulationConfig> {
    let file_content = fs::read_to_string(file_path)?;
    let config: SimulationConfig = serde_json::from_str(&file_content)?;
    Ok(config)
}

pub async fn run_simulation() -> Result<()> {
    dotenv().ok(); // Load environment variables from .env file

    let api_key = env::var("TENDERLY_ACCESS_KEY")?;
    let account_slug = env::var("TENDERLY_ACCOUNT_SLUG")?;
    let project_slug = env::var("TENDERLY_PROJECT_SLUG")?;

    let config = read_simulation_config("admin_tx.json")?;

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
        config.to.parse().expect("Failed to parse to"),
        U256::from(config.value.parse::<U256>().expect("Failed to parse value")),
        Bytes::from(config.data.parse::<Bytes>().expect("Failed to parse data")),
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
