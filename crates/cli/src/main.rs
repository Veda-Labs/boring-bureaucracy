use clap::{Parser, Subcommand};
use core::{
    HardwareWalletType, approve_hash, exec_transaction, generate_root_update_txs,
    simulate_admin_tx_and_generate_safe_hash, simulate_timelock_admin_txs_and_generate_safe_hashes,
};
use eyre::Result;
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
            let configs = generate_root_update_txs(root, product, *network_id, *nonce).await?;

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
    }

    Ok(())
}
