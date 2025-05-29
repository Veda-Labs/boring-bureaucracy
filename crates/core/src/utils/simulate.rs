use crate::bindings::multisig::GnosisSafe::{self, GnosisSafeInstance};
use crate::types::{config_wrapper::ConfigWrapper, simulation_config::SimulationConfig};
use alloy::primitives::{Address, Bytes, U256};
use alloy::{providers::ProviderBuilder, sol_types::SolCall};
use dotenv::dotenv;
use eyre::Result;
use hex;
use reqwest::Client;
use serde_json::{Value, json};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub async fn generate_safe_hash_and_return_params(
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

    let to_address: Address = config.to();
    let value = config.value();
    let data = config.data();
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

pub async fn simulate_admin_tx_and_generate_safe_hash(
    admin_tx_path: &str,
) -> Result<(String, String)> {
    dotenv().ok(); // Load environment variables from .env file

    let api_key = env::var("TENDERLY_ACCESS_KEY")?;
    let account_slug = env::var("TENDERLY_ACCOUNT_SLUG")?;
    let project_slug = env::var("TENDERLY_PROJECT_SLUG")?;

    let config = SimulationConfig::from_file(admin_tx_path)?;
    let cw = ConfigWrapper::from_file(None)?;

    // Call getTransactionHash
    let rpc_url = cw.get_rpc_url(config.network_id)?;
    let provider = ProviderBuilder::new().on_builtin(&rpc_url).await?;
    let safe_address = config.multisig();
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

    let simulation_result = response.json::<Value>().await?;

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
        .unwrap_or("Simulation Failed".to_string());
    // .ok_or_else(|| eyre::eyre!("Simulation ID not found in response"))?;

    Ok((simulation_url, safe_hash))
}

pub async fn simulate_timelock_admin_txs_and_generate_safe_hashes(
    propose_tx_path: String,
    execute_tx_path: String,
) -> Result<(String, String, String)> {
    dotenv().ok();

    let cw = ConfigWrapper::from_file(None)?;

    let api_key = env::var("TENDERLY_ACCESS_KEY")?;
    let account_slug = env::var("TENDERLY_ACCOUNT_SLUG")?;
    let project_slug = env::var("TENDERLY_PROJECT_SLUG")?;

    let propose_config = SimulationConfig::from_file(&propose_tx_path)?;
    let execute_config = SimulationConfig::from_file(&execute_tx_path)?;

    // Validate matching fields between propose and execute configs
    if propose_config.network_id != execute_config.network_id {
        return Err(eyre::eyre!(
            "Network IDs do not match: propose={}, execute={}",
            propose_config.network_id,
            execute_config.network_id
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
    let rpc_url = cw.get_rpc_url(propose_config.network_id)?;
    let provider = ProviderBuilder::new().on_builtin(&rpc_url).await?;
    let safe_address = propose_config.multisig();
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
