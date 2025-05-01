use clap::{Parser, Subcommand};
use core::{
    HardwareWalletType, approve_hash, exec_transaction, generate_admin_actions_from_json,
    generate_root_update_txs,
    types::simulation_config::SimulationConfig,
    utils::simulate::{
        simulate_admin_tx_and_generate_safe_hash,
        simulate_timelock_admin_txs_and_generate_safe_hashes,
    },
};
use eyre::{Result, eyre};
use serde_json::Value;
use std::fs;
use std::path::Path;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Simulate an admin transaction and generate safe hash
    Simulate {
        /// Path to the admin transaction JSON file
        #[arg(long = "tx", short = 't')]
        tx_path: String,
    },
    /// Generate root update transactions
    UpdateRoot {
        /// New root value (32 byte hex)
        #[arg(long = "root", short = 'r')]
        root: String,

        /// Product name
        #[arg(long = "product", short = 'p')]
        product: String,

        /// Network ID
        #[arg(long = "network", short = 'n')]
        network_id: u32,

        /// Nonce
        #[arg(long = "nonce")]
        nonce: u32,
    },
    /// Simulate timelock transactions
    SimulateTimelock {
        /// Path to propose transaction JSON
        #[arg(long = "propose", short = 'p')]
        propose_path: String,

        /// Path to execute transaction JSON
        #[arg(long = "execute", short = 'e')]
        execute_path: String,
    },
    /// Approve a Safe transaction hash using a hardware wallet
    ApproveHash {
        /// Path to the transaction JSON file
        #[arg(long = "tx", short = 'p')]
        tx_path: String,

        /// Use Trezor hardware wallet
        #[arg(long = "trezor", short = 't', conflicts_with = "ledger")]
        trezor: bool,

        /// Use Ledger hardware wallet
        #[arg(long = "ledger", short = 'l', conflicts_with = "trezor")]
        ledger: bool,
    },
    /// Propose a Safe transaction hash using a hardware wallet
    ProposeTransaction {
        /// New root value (32 byte hex)
        #[arg(long = "root", short = 'r')]
        root: String,

        /// Product name
        #[arg(long = "product", short = 'p')]
        product: String,

        /// Network ID
        #[arg(long = "network", short = 'n')]
        network_id: u32,

        /// Nonce
        #[arg(long = "nonce")]
        nonce: u32,

        /// Use Trezor hardware wallet
        #[arg(long = "trezor", short = 't', conflicts_with = "ledger")]
        trezor: bool,

        /// Use Ledger hardware wallet
        #[arg(long = "ledger", short = 'l', conflicts_with = "trezor")]
        ledger: bool,
    },
    /// Execute a Safe transaction using a hardware wallet
    ExecTransaction {
        /// Path to the transaction JSON file
        #[arg(long = "tx", short = 'p')]
        tx_path: String,

        /// Use Trezor hardware wallet
        #[arg(long = "trezor", short = 't', conflicts_with = "ledger")]
        trezor: bool,

        /// Use Ledger hardware wallet
        #[arg(long = "ledger", short = 'l', conflicts_with = "trezor")]
        ledger: bool,
    },
    /// Generates admin txs from json
    FromJson {
        /// Path to the JSON file
        #[arg(long = "tx", short = 'p')]
        tx_path: String,

        /// Use Trezor hardware wallet
        #[arg(long = "trezor", short = 't', conflicts_with = "ledger")]
        trezor: bool,

        /// Use Ledger hardware wallet
        #[arg(long = "ledger", short = 'l', conflicts_with = "trezor")]
        ledger: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Simulate { tx_path } => {
            let (simulation_url, safe_hash) =
                simulate_admin_tx_and_generate_safe_hash(tx_path).await?;
            println!("Safe Hash: {}", safe_hash);
            println!("Simulation URL: {}", simulation_url);
        }
        Commands::UpdateRoot {
            root,
            product,
            network_id,
            nonce,
        } => {
            // Remove output directory if it exists, then create it fresh
            if Path::new("output").exists() {
                fs::remove_dir_all("output")?;
            }
            fs::create_dir_all("output")?;

            // Generate transactions
            let (configs, _strategists) =
                generate_root_update_txs(root, product, *network_id, *nonce).await?;

            // Save each config to a numbered JSON file
            for (i, config) in configs.iter().enumerate() {
                let filename = format!("output/tx_{}.json", i);
                let json = serde_json::to_string_pretty(&config)?;
                fs::write(&filename, json)?;
                println!("Saved transaction to: {}", filename);
            }
        }
        Commands::SimulateTimelock {
            propose_path,
            execute_path,
        } => {
            let (simulation_url, propose_hash, execute_hash) =
                simulate_timelock_admin_txs_and_generate_safe_hashes(
                    propose_path.clone(),
                    execute_path.clone(),
                )
                .await?;

            println!("Propose Hash: {}", propose_hash);
            println!("Execute Hash: {}", execute_hash);
            println!("Simulation URL: {}", simulation_url);
        }
        Commands::ApproveHash {
            tx_path,
            trezor,
            ledger,
        } => {
            let wallet_type = match (*trezor, *ledger) {
                (true, false) => HardwareWalletType::TREZOR,
                (false, true) => HardwareWalletType::LEDGER,
                _ => {
                    return Err(eyre::eyre!(
                        "Must specify either --trezor (-t) or --ledger (-l)"
                    ));
                }
            };

            let tx_url = approve_hash(tx_path, wallet_type).await?;
            println!("Transaction URL: {}", tx_url);
        }
        Commands::ExecTransaction {
            tx_path,
            trezor,
            ledger,
        } => {
            let wallet_type = match (*trezor, *ledger) {
                (true, false) => HardwareWalletType::TREZOR,
                (false, true) => HardwareWalletType::LEDGER,
                _ => {
                    return Err(eyre::eyre!(
                        "Must specify either --trezor (-t) or --ledger (-l)"
                    ));
                }
            };

            let tx_url = exec_transaction(tx_path, wallet_type).await?;
            println!("Transaction URL: {}", tx_url);
        }
        Commands::ProposeTransaction {
            root,
            product,
            network_id,
            nonce,
            trezor,
            ledger,
        } => {
            // Generate the transaction configs
            let (configs, strategists) =
                generate_root_update_txs(&root, &product, *network_id, *nonce).await?;

            // Process based on number of configs
            match configs.len() {
                1 => {
                    // Save tx config to file
                    fs::write(
                        "output/single.json",
                        serde_json::to_string_pretty(&configs[0])?,
                    )?;

                    // Simulate single tx
                    let (simulation_url, safe_hash) =
                        simulate_admin_tx_and_generate_safe_hash("output/single.json").await?;
                    println!("\nSimulation URL: {}", simulation_url);

                    // Ask user if they want to approve
                    if prompt_user_confirmation("Would you like to approve this transaction?")? {
                        let wallet_type = if *trezor {
                            HardwareWalletType::TREZOR
                        } else {
                            if !*ledger {
                                return Err(eyre!("Please select a hardware to use"));
                            }
                            HardwareWalletType::LEDGER
                        };

                        let tx_url = approve_hash("output/single.json", wallet_type).await?;

                        // Generate and print summary
                        print_transaction_summary(
                            &product,
                            *network_id,
                            &configs[0],
                            &root,
                            &safe_hash,
                            &strategists,
                            &tx_url,
                            simulation_url,
                        )?;
                    }
                }
                2 => {
                    // Save both tx configs
                    fs::write(
                        "output/propose.json",
                        serde_json::to_string_pretty(&configs[0])?,
                    )?;
                    fs::write(
                        "output/execute.json",
                        serde_json::to_string_pretty(&configs[1])?,
                    )?;

                    // Simulate timelock txs
                    let (simulation_url, propose_hash, execute_hash) =
                        simulate_timelock_admin_txs_and_generate_safe_hashes(
                            "output/propose.json".to_string(),
                            "output/execute.json".to_string(),
                        )
                        .await?;

                    println!("\nSimulation URL: {}", simulation_url);

                    // Handle first transaction
                    if prompt_user_confirmation(
                        "Would you like to approve the propose transaction?",
                    )? {
                        let wallet_type = if *trezor {
                            HardwareWalletType::TREZOR
                        } else {
                            HardwareWalletType::LEDGER
                        };

                        let tx_url = approve_hash("output/propose.json", wallet_type).await?;

                        // Print summary for propose tx
                        print_transaction_summary(
                            &product,
                            *network_id,
                            &configs[0],
                            &root,
                            &propose_hash,
                            &strategists,
                            &tx_url,
                            simulation_url.clone(),
                        )?;
                    }

                    // Handle second transaction
                    if prompt_user_confirmation(
                        "Would you like to approve the execute transaction?",
                    )? {
                        let wallet_type = if *trezor {
                            HardwareWalletType::TREZOR
                        } else {
                            HardwareWalletType::LEDGER
                        };

                        let tx_url = approve_hash("output/execute.json", wallet_type).await?;

                        // Print summary for execute tx
                        print_transaction_summary(
                            &product,
                            *network_id,
                            &configs[1],
                            &root,
                            &execute_hash,
                            &strategists,
                            &tx_url,
                            simulation_url,
                        )?;
                    }
                }
                _ => return Err(eyre!("Unexpected number of transactions generated")),
            }
        }
        Commands::FromJson {
            tx_path,
            trezor,
            ledger,
        } => {
            // Read and parse the JSON file
            let file_content = fs::read_to_string(tx_path)?;
            let json_value: Value = serde_json::from_str(&file_content)?;

            // Generate the configs and descriptions
            let (configs, descriptions) =
                generate_admin_actions_from_json(json_value.clone()).await?;

            // Process based on number of configs
            match configs.len() {
                1 => {
                    // Save tx config to file
                    fs::write(
                        "output/single.json",
                        serde_json::to_string_pretty(&configs[0])?,
                    )?;

                    // Simulate single tx
                    let (simulation_url, safe_hash) =
                        simulate_admin_tx_and_generate_safe_hash("output/single.json").await?;

                    if *trezor || *ledger {
                        // Ask user if they want to approve
                        if prompt_user_confirmation("Would you like to approve this transaction?")?
                        {
                            let wallet_type = if *trezor {
                                HardwareWalletType::TREZOR
                            } else {
                                if !*ledger {
                                    return Err(eyre!("Please select a hardware wallet to use"));
                                }
                                HardwareWalletType::LEDGER
                            };

                            let tx_url = approve_hash("output/single.json", wallet_type).await?;

                            println!("\n# Action Configuration");
                            println!("```json");
                            println!("{}", serde_json::to_string_pretty(&json_value)?);
                            println!("```\n");

                            print_advanced_transaction_summary(
                                None,
                                &configs[0],
                                descriptions[0].clone(),
                                &safe_hash,
                                Some(&tx_url),
                                simulation_url,
                            )?;
                        }
                    } else {
                        println!("\n# Action Configuration");
                        println!("```json");
                        println!("{}", serde_json::to_string_pretty(&json_value)?);
                        println!("```\n");
                        print_advanced_transaction_summary(
                            None,
                            &configs[0],
                            descriptions[0].clone(),
                            &safe_hash,
                            None,
                            simulation_url,
                        )?;
                    }
                }
                2 => {
                    // Save both tx configs
                    fs::write(
                        "output/propose.json",
                        serde_json::to_string_pretty(&configs[0])?,
                    )?;
                    fs::write(
                        "output/execute.json",
                        serde_json::to_string_pretty(&configs[1])?,
                    )?;

                    // Simulate timelock txs
                    let (simulation_url, propose_hash, execute_hash) =
                        simulate_timelock_admin_txs_and_generate_safe_hashes(
                            "output/propose.json".to_string(),
                            "output/execute.json".to_string(),
                        )
                        .await?;

                    println!("\nSimulation URL: {}", simulation_url);

                    if *trezor || *ledger {
                        let send_propose = prompt_user_confirmation(
                            "Would you like to approve the propose transaction?",
                        )?;
                        let send_execute = prompt_user_confirmation(
                            "Would you like to approve the execute transaction?",
                        )?;

                        // Handle first transaction
                        if send_propose {
                            let wallet_type = if *trezor {
                                HardwareWalletType::TREZOR
                            } else {
                                HardwareWalletType::LEDGER
                            };

                            let tx_url = approve_hash("output/propose.json", wallet_type).await?;

                            println!("\n# Action Configuration");
                            println!("```json");
                            println!("{}", serde_json::to_string_pretty(&json_value)?);
                            println!("```\n");

                            // Print summary for propose tx
                            print_advanced_transaction_summary(
                                Some("Propose".to_string()),
                                &configs[0],
                                descriptions[0].clone(),
                                &propose_hash,
                                Some(&tx_url),
                                simulation_url.clone(),
                            )?;
                        }

                        // Handle second transaction
                        if send_execute {
                            let wallet_type = if *trezor {
                                HardwareWalletType::TREZOR
                            } else {
                                HardwareWalletType::LEDGER
                            };

                            let tx_url = approve_hash("output/execute.json", wallet_type).await?;

                            if !send_propose {
                                println!("\n# Action Configuration");
                                println!("```json");
                                println!("{}", serde_json::to_string_pretty(&json_value)?);
                                println!("```\n");
                            }

                            // Print summary for execute tx
                            print_advanced_transaction_summary(
                                Some("Execute".to_string()),
                                &configs[1],
                                descriptions[1].clone(),
                                &execute_hash,
                                Some(&tx_url),
                                simulation_url,
                            )?;
                        }
                    } else {
                        // Print summaries for both transactions without sending them
                        println!("\n# Action Configuration");
                        println!("```json");
                        println!("{}", serde_json::to_string_pretty(&json_value)?);
                        println!("```\n");

                        print_advanced_transaction_summary(
                            Some("Propose".to_string()),
                            &configs[0],
                            descriptions[0].clone(),
                            &propose_hash,
                            None,
                            simulation_url.clone(),
                        )?;

                        print_advanced_transaction_summary(
                            Some("Execute".to_string()),
                            &configs[1],
                            descriptions[1].clone(),
                            &execute_hash,
                            None,
                            simulation_url,
                        )?;
                    }
                }
                _ => return Err(eyre!("Unexpected number of transactions generated")),
            }
        }
    }

    Ok(())
}

// Helper function to prompt user for confirmation
fn prompt_user_confirmation(message: &str) -> Result<bool> {
    println!("\n{} (y/n)", message);
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_lowercase() == "y")
}

// Helper function to print transaction summary
fn print_transaction_summary(
    product: &str,
    network_id: u32,
    tx_config: &SimulationConfig,
    root: &str,
    safe_hash: &str,
    strategists: &Vec<String>,
    tx_url: &str,
    simulation_url: String,
) -> Result<()> {
    println!("\n# {} (Network: {})", product, network_id);
    println!("\n## Transaction Data");
    println!("```json");
    println!("{}", serde_json::to_string_pretty(tx_config)?);
    println!("```");

    println!("\n## Strategists");
    for (i, strategist) in strategists.iter().enumerate() {
        println!("{}. {}", i + 1, strategist);
    }

    println!("\n## New Root");
    println!("`{}`", root);

    println!("\n## Safe Hash");
    println!("`{}`", safe_hash);

    println!("\n## Links");
    println!("- [Proposal Transaction]({})", tx_url);
    println!("- [Simulation]({})", simulation_url);

    Ok(())
}

fn print_advanced_transaction_summary(
    tx_name: Option<String>,
    tx_config: &SimulationConfig,
    descriptions: Vec<String>,
    safe_hash: &str,
    tx_url: Option<&str>,
    simulation_url: String,
) -> Result<()> {
    // Build the title based on tx_name and network_id
    let title = match tx_name {
        Some(name) => format!("\n# {} Transaction Summary ", name),
        None => format!("\n# Transaction Summary "),
    };
    println!("{}", title);

    println!("\n## Transaction Data");
    println!("```json");
    println!("{}", serde_json::to_string_pretty(&tx_config)?);
    println!("```");

    println!("\n## Actions");
    println!("```json");
    for desc in descriptions.iter() {
        println!("{}", desc);
    }
    println!("```");

    println!("\n## Safe Hash");
    println!("`{}`", safe_hash);

    println!("\n## Links");
    match tx_url {
        Some(url) => println!("- [Proposal Transaction]({})", url),
        None => println!("- Proposal Transaction NONE"),
    }
    println!("- [Simulation]({})", simulation_url);

    Ok(())
}
