pub mod actions;
pub mod bindings;
pub mod processors;
pub mod types;
pub mod utils;
use crate::{
    actions::{multisend_utils::create_multisend_data, timelock_action::TimelockAction, set_merkle_root_action::SetMerkleRoot},
    bindings::{
        manager::ManagerWithMerkleVerification, multisend::MutliSendCallOnly, multisig::GnosisSafe,
        timelock::Timelock,
    },
    processors::{
        asset_update::process_asset_updates, root_update::process_merkle_root_update,
        solver_update::process_solver_update, update_fees::process_fee_updates,
        process_strategist_roles_update::process_strategist_roles_update,
    },
    types::transaction::Transaction,
    utils::simulate::generate_safe_hash_and_return_params,
};
use actions::admin_action::AdminAction;
use alloy::network::EthereumWallet;
use alloy::primitives::{Address, Bytes, FixedBytes, U256};
use alloy::providers::Provider;
use alloy::signers::ledger::{self, LedgerSigner};
use alloy::signers::trezor::{self, TrezorSigner};
use alloy::{providers::ProviderBuilder, sol_types::SolCall};
use dotenv::dotenv;
use eyre::{Result, eyre};
use hex;
use processors::withdraw_asset_update::process_queue_asset_updates;
use reqwest::Client;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::env;
use types::{config_wrapper::ConfigWrapper, simulation_config::SimulationConfig};
pub use utils::simulate::{
    simulate_admin_tx_and_generate_safe_hash, simulate_timelock_admin_txs_and_generate_safe_hashes,
};
// Should return the min number of actions which really should just be 2
// could revert if you have a value that relies on multiple multisigs...
pub async fn generate_admin_actions_from_json(
    value: Value,
) -> Result<(Vec<SimulationConfig>, Vec<Vec<String>>)> {
    dotenv().ok();
    // Extract the required fields from the JSON
    let network_id = value["network_id"]
        .as_u64()
        .ok_or_else(|| eyre!("network_id must be a number"))? as u32;

    let nonce = value["nonce"]
        .as_u64()
        .ok_or_else(|| eyre!("nonce must be a number"))? as u32;

    let actions = value["actions"]
        .as_array()
        .ok_or_else(|| eyre!("actions must be an array"))?;

    // Load config from default path
    let cw = ConfigWrapper::from_file(None)?;

    // Check that all products use the same multisig address
    let mut multisig_addresses = HashSet::new();

    for action in actions {
        let product = action["product"]
            .as_str()
            .ok_or_else(|| eyre!("product must be a string"))?;

        // Get multisig address for this product
        let multisig_address =
            cw.get_product_config_value(product, network_id, "multisig_address")?;

        multisig_addresses.insert(multisig_address);
    }

    if multisig_addresses.len() > 1 {
        return Err(eyre!(
            "Cannot combine actions for products with different multisig addresses"
        ));
    }

    // Now process each action
    let mut admin_actions: HashMap<Option<Address>, Vec<Box<dyn AdminAction>>> = HashMap::new();

    for action in actions {
        let product = action["product"]
            .as_str()
            .ok_or_else(|| eyre!("product must be a string"))?;

        let timelock_addr =
            match cw.get_product_config_value(product, network_id, "timelock_address") {
                Ok(addr) => Some(addr.parse::<Address>()?),
                Err(_) => None, // Timelock is optional
            };

        let mut action_sub_set = admin_actions.entry(timelock_addr).or_insert_with(Vec::new);

        // Process merkle root updates if present
        if let Some(root_str) = action["new_root"].as_str() {
            process_merkle_root_update(&mut action_sub_set, &cw, product, network_id, root_str)?;
        }

        // Process asset updates if present
        if let Some(new_assets) = action["new_assets"].as_array() {
            for asset_update in new_assets {
                process_asset_updates(&mut action_sub_set, &cw, product, network_id, asset_update)
                    .await?;
            }
        }

        // Process withdraw asset updates if present.
        if let Some(new_queue_assets) = action["new_queue_assets"].as_array() {
            for queue_asset in new_queue_assets {
                process_queue_asset_updates(action_sub_set, &cw, product, network_id, queue_asset)
                    .await?;
            }
        }

        if let Some(fee_data) = action["update_fees"].as_object() {
            process_fee_updates(
                &mut action_sub_set,
                &cw,
                product,
                network_id,
                &Value::Object(fee_data.clone()),
            )?;
        }

        // Process solver updates if present
        if let Some(solver_data) = action["update_solver"].as_object() {
            process_solver_update(
                &mut action_sub_set,
                &cw,
                product,
                network_id,
                &Value::Object(solver_data.clone()),
            )?;
        }

        // Process strategist updates (roles and potentially Merkle root for removal)
        if let Some(strategist_update_data_val) = action.get("update_strategist") {
            if let Some(strategist_update_data_obj) = strategist_update_data_val.as_object() {
                // Process role updates (add or revoke)
                process_strategist_roles_update(
                    &mut action_sub_set,
                    &cw,
                    product,
                    network_id,
                    strategist_update_data_val, // Pass the original Value
                )?;

                // If operation is "revoke_roles", also set Merkle root to zero
                if strategist_update_data_obj.get("operation").and_then(|v| v.as_str()) == Some("revoke_roles") {
                    let strategist_address_str = strategist_update_data_obj
                        .get("strategist_address")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| eyre!("'strategist_address' is required when revoking roles for 'update_strategist'"))?;
                    let strategist_addr = strategist_address_str.parse::<Address>()?;
                    
                    let manager_addr_str = 
                        cw.get_product_config_value(product, network_id, "manager_address")?;
                    let manager_addr = manager_addr_str.parse::<Address>()?;

                    let zero_root = FixedBytes::<32>::ZERO; // This is bytes32(0)

                    let set_root_action = SetMerkleRoot::new(
                        manager_addr,
                        strategist_addr,
                        zero_root,
                    );
                    action_sub_set.push(Box::new(set_root_action));
                }
            } else {
                return Err(eyre!("'update_strategist' must be an object"));
            }
        }
    }

    // Get the multisig address (we know there's only one from earlier validation)
    let multisig_address: Address = multisig_addresses.into_iter().next().unwrap().parse()?;

    let mut txs_0 = Vec::new();
    let mut txs_1 = Vec::new();

    let mut descriptions = Vec::new();
    descriptions.push(vec![]);
    descriptions.push(vec![]);

    for (timelock_addr, actions) in admin_actions {
        match timelock_addr {
            Some(addr) => {
                // Create propose and execute timelock actions
                // Read the min delay.
                let rpc_url = cw.get_rpc_url(network_id)?;
                let provider = ProviderBuilder::new().on_builtin(&rpc_url).await?;
                let timelock = Timelock::new(addr, provider);
                let min_delay = timelock.getMinDelay().call().await?.delay;
                let mut timelock_action = TimelockAction::new(addr, min_delay, actions);
                txs_0.push(Transaction {
                    to: timelock_action.target(),
                    value: timelock_action.value(),
                    data: timelock_action.data(),
                });
                descriptions[0].push(serde_json::to_string_pretty(&timelock_action.describe())?);
                timelock_action.toggle_mode(); // Change mode to execute.
                txs_1.push(Transaction {
                    to: timelock_action.target(),
                    value: timelock_action.value(),
                    data: timelock_action.data(),
                });
                descriptions[1].push(serde_json::to_string_pretty(&timelock_action.describe())?);
            }
            None => {
                for action in actions {
                    txs_0.push(Transaction {
                        to: action.target(),
                        value: action.value(),
                        data: action.data(),
                    });
                    descriptions[0].push(serde_json::to_string_pretty(&action.describe())?);
                }
            }
        }
    }

    // Convert txs to multisend txs if needed.
    let multisend = cw.get_multisend_address(network_id as u32)?;
    let multisend_addr: Address = multisend.parse().unwrap();

    let mut final_configs = Vec::new();

    match txs_0.len() {
        0 => return Err(eyre!("No transactions to send")),
        1 => final_configs.push(SimulationConfig {
            network_id,
            multisig: multisig_address.to_string(),
            to: txs_0[0].to.to_string(),
            value: txs_0[0].value.to_string(),
            data: format!("0x{}", hex::encode(txs_0[0].data.clone())),
            operation: 0,
            nonce,
        }),
        _ => {
            let data = create_multisend_data(txs_0);
            final_configs.push(SimulationConfig {
                network_id,
                multisig: multisig_address.to_string(),
                to: multisend_addr.to_string(),
                value: "0".to_string(),
                data: format!("0x{}", hex::encode(data)),
                operation: 1,
                nonce,
            });
        }
    }

    match txs_1.len() {
        0 => {} // Do nothing
        1 => final_configs.push(SimulationConfig {
            network_id,
            multisig: multisig_address.to_string(),
            to: txs_1[0].to.to_string(),
            value: txs_1[0].value.to_string(),
            data: format!("0x{}", hex::encode(txs_1[0].data.clone())),
            operation: 0,
            nonce: nonce + 1,
        }),
        _ => {
            let data = create_multisend_data(txs_1);
            final_configs.push(SimulationConfig {
                network_id,
                multisig: multisig_address.to_string(),
                to: multisend_addr.to_string(),
                value: "0".to_string(),
                data: format!("0x{}", hex::encode(data)),
                operation: 1,
                nonce: nonce + 1,
            });
        }
    }

    Ok((final_configs, descriptions))
}

// TODO append calldata to the end of approve hash call that has the nonce?
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
    let cw = ConfigWrapper::from_file(None)?;

    // Get required addresses from config
    let strategists = cw.get_product_strategists(product_name, network_id)?;

    if strategists.is_empty() {
        return Err(eyre::eyre!("Strategists array cannot be empty"));
    }

    let manager_address =
        cw.get_product_config_value(product_name, network_id, "manager_address")?;

    let multisig_address =
        cw.get_product_config_value(product_name, network_id, "multisig_address")?;

    let timelock_address =
        cw.get_product_config_value_or_default(product_name, network_id, "timelock_address");

    // Parse addresses
    let manager_addr: Address = manager_address.parse()?;
    let timelock_addr: Address = timelock_address.parse()?;

    let mut txs = Vec::new();
    if timelock_addr != Address::ZERO {
        // Load env variables
        dotenv().ok();

        // Read the min delay.
        let rpc_url = cw.get_rpc_url(network_id)?;
        let provider = ProviderBuilder::new().on_builtin(&rpc_url).await?;
        let timelock = Timelock::new(timelock_addr, provider);
        let min_delay = timelock.getMinDelay().call().await?.delay;

        let mut targets = Vec::with_capacity(strategists.len());
        let mut values = Vec::with_capacity(strategists.len());
        let mut data = Vec::with_capacity(strategists.len());
        let predecessor = FixedBytes::<32>::ZERO;
        let salt = FixedBytes::<32>::ZERO;

        for strategist in &strategists {
            targets.push(manager_addr);
            values.push(U256::ZERO);
            let strategist_addr = strategist.parse()?;
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
            let strategist_addr = strategists[0].parse()?;
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
            for strategist in &strategists {
                let strategist_addr = strategist.parse()?;
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
                to: cw.get_multisend_address(network_id)?,
                value: "0".to_string(),
                data: format!("0x{}", hex::encode(multisend_data)),
                operation: 1,
                nonce,
            });
        }
    }

    Ok((txs, strategists))
}

pub enum HardwareWalletType {
    TREZOR,
    LEDGER,
}

// TODO append tx data to the end of approve hash
pub async fn approve_hash(admin_tx_path: &str, wallet_type: HardwareWalletType) -> Result<String> {
    dotenv().ok(); // Load environment variables from .env file

    let cw = ConfigWrapper::from_file(None)?;

    let config = SimulationConfig::from_file(admin_tx_path)?;
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
    let rpc_url = cw.get_rpc_url(config.network_id)?;
    let provider = ProviderBuilder::new().on_builtin(&rpc_url).await?;
    let safe_address = config.multisig();
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

    let block_explorer_url = cw.get_block_explorer_url(config.network_id)?;
    let tx_hash_hex = hex::encode(tx_hash.as_slice());
    Ok(format!("{}/tx/0x{}", block_explorer_url, tx_hash_hex))
}

// TODO my new function needs to print out pretty markdown
pub async fn exec_transaction(
    admin_tx_path: &str,
    wallet_type: HardwareWalletType,
) -> Result<String> {
    dotenv().ok(); // Load environment variables from .env file

    let cw = ConfigWrapper::from_file(None)?;

    let api_key = env::var("TENDERLY_ACCESS_KEY")?;
    let account_slug = env::var("TENDERLY_ACCOUNT_SLUG")?;
    let project_slug = env::var("TENDERLY_PROJECT_SLUG")?;

    let config = SimulationConfig::from_file(admin_tx_path)?;
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
    let rpc_url = cw.get_rpc_url(config.network_id)?;
    let provider = ProviderBuilder::new().on_builtin(&rpc_url).await?;
    let safe_address = config.multisig();
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
    let threshold: u32 = safe.getThreshold().call().await?.threshold.try_into()?;

    // Get the owners
    let mut owners = safe.getOwners().call().await?.owners;

    // Sort the owners in ascending order.
    owners.sort();

    // Iterate through each owner, and check if they signed the safe hash, if so append them to the final signature.
    let mut signatures = Vec::new();
    let mut signer_count: u32 = 0;
    let mut has_not_approved = Vec::new();
    for owner in &owners {
        // Check if owner has approved the safe hash.
        let has_approved = !safe
            .approvedHashes(*owner, safe_hash)
            .call()
            .await?
            ._0
            .is_zero();
        if has_approved {
            // r: 32 bytes - padded address
            signatures.extend_from_slice(owner.into_word().as_slice());

            // s: 32 bytes - all zeros
            signatures.extend_from_slice(&[0u8; 32]);

            // v: 1 byte - always 1
            signatures.push(1);

            signer_count += 1;
        } else {
            has_not_approved.push(*owner);
        }
        if signer_count >= threshold {
            break;
        }
    }
    if signer_count < threshold {
        // Find which owners haven't approved yet
        let remaining_needed = threshold - signer_count;

        println!("\nNeed {} more signature(s) from:", remaining_needed);
        for (i, owner) in has_not_approved.iter().enumerate() {
            println!("{}. {}", i + 1, owner);
        }

        return Err(eyre!(
            "Not enough signers, have {}, need {}",
            signer_count,
            threshold
        ));
    }

    let safe_tx_gas = U256::ZERO;
    let base_gas = U256::ZERO;
    let gas_price = U256::ZERO;
    let gas_token = Address::ZERO;
    let refund_receiver = Address::ZERO;

    let to_address: Address = config.to();
    let value = config.value();
    let data = config.data();
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

    let block_explorer_url = cw.get_block_explorer_url(config.network_id)?;
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
