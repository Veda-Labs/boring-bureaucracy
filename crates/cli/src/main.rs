use clap::{Parser, Subcommand};
use core::simulate_admin_tx_and_generate_safe_hash;
use eyre::Result;

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
    /// Generate admin transaction data
    Generate {
        /// Path to save the generated admin transaction JSON
        #[arg(long = "output", short = 'o')]
        output: String,

        /// Network ID for the transaction
        #[arg(long = "network", short = 'n')]
        network_id: u32,

        /// Multisig address
        #[arg(long = "multisig", short = 'm')]
        multisig: String,
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
        Commands::Generate {
            output,
            network_id,
            multisig,
        } => {
            // TODO: Implement admin tx generation
            println!(
                "TODO: Generate admin tx for network {} and multisig {} to {}",
                network_id, multisig, output
            );
        }
    }

    Ok(())
}
