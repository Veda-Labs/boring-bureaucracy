use dotenv::dotenv;
use eyre::Result;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::{env, fs};

#[derive(Deserialize)]
struct SimulationConfig {
    network_id: String,
    to: String,
    input: String,
}

fn read_simulation_config(file_path: &str) -> Result<SimulationConfig, Box<dyn std::error::Error>> {
    let file_content = fs::read_to_string(file_path)?;
    let config: SimulationConfig = serde_json::from_str(&file_content)?;
    Ok(config)
}

pub async fn run_simulation() -> Result<()> {
    dotenv().ok(); // Load environment variables from .env file

    let api_key = env::var("TENDERLY_ACCESS_KEY")?;
    let account_slug = env::var("TENDERLY_ACCOUNT_SLUG")?;
    let project_slug = env::var("TENDERLY_PROJECT_SLUG")?;

    let client = Client::new();
    let response = client.post(&format!(
        "https://api.tenderly.co/api/v1/account/{}/project/{}/simulate",
        account_slug, project_slug
    ))
    .header("X-Access-Key", api_key)
    .json(&json!({
        "save": true,
        "save_if_fails": true,
        "simulation_type": "full",
        "network_id": "1",
        "from": "0xe2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2",
        "to": "0x6b175474e89094c44da98b954eedeac495271d0f",
        "input": "0x40c10f19000000000000000000000000e58b9ee93700a616b50509c8292977fa7a0f8ce10000000000000000000000000000000000000000000000001bc16d674ec80000",
        "gas": 8000000,
        "state_objects": {
            "0x6b175474e89094c44da98b954eedeac495271d0f": {
                "storage": {
                    "0xedd7d04419e9c48ceb6055956cbb4e2091ae310313a4d1fa7cbcfe7561616e03": "0x0000000000000000000000000000000000000000000000000000000000000001"
                }
            }
        }
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
