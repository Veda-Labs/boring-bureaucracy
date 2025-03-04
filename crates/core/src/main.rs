use alloy::primitives::{Address, Bytes, FixedBytes, U256, b256};
use core::{
    generate_root_update_tx, simulate_admin_tx_and_generate_safe_hash,
    simulate_timelock_admin_txs_and_generate_safe_hashes,
};

#[tokio::main]
async fn main() {
    // match simulate_admin_tx_and_generate_safe_hash("admin_tx.json").await {
    //     Ok((simulation_url, safe_hash)) => {
    //         println!("Safe Hash: {}", safe_hash);
    //         println!("Simulation URL: {}", simulation_url);
    //     }
    //     Err(e) => {
    //         eprintln!("Error: {:?}", e);
    //     }
    // }

    // match generate_root_update_tx(
    //     b256!("0x85a1d638adb1d66ab2fd111cc71199c952980658e19ee82b06d76cf42b341b74"),
    //     "sc_eth",
    //     146,
    //     34,
    // )
    // .await
    // {
    //     Ok(configs) => {
    //         for (i, config) in configs.iter().enumerate() {
    //             let json = serde_json::to_string_pretty(&config)
    //                 .expect("Failed to serialize config to JSON");
    //             println!("Transaction {i}:\n{json}\n");
    //         }
    //     }
    //     Err(e) => {
    //         eprintln!("Error: {:?}", e);
    //     }
    // }

    match simulate_timelock_admin_txs_and_generate_safe_hashes(
        "tx_0.json".to_string(),
        "tx_1.json".to_string(),
    )
    .await
    {
        Ok((simulation_url, propose_hash, execute_hash)) => {
            println!("Propose Hash: {}", propose_hash);
            println!("Execute Hash: {}", execute_hash);
            println!("Simulation URL: {}", simulation_url);
        }
        Err(e) => {
            eprintln!("Error: {:?}", e);
        }
    }
}
