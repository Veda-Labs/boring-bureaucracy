// use alloy::primitives::{Address, Bytes, FixedBytes, U256, b256};
use core::{
    HardwareWalletType, approve_hash, generate_notion_markdown, generate_root_update_txs,
    simulate_admin_tx_and_generate_safe_hash, simulate_timelock_admin_txs_and_generate_safe_hashes,
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
    //     b256!("0x89a526fb2b69815032c7c59b737cef4f7275105b4e02cd4c6cc09317876cb406"),
    //     "sc_usd",
    //     146,
    //     35,
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

    // match simulate_timelock_admin_txs_and_generate_safe_hashes(
    //     "tx_0.json".to_string(),
    //     "tx_1.json".to_string(),
    // )
    // .await
    // {
    //     Ok((simulation_url, propose_hash, execute_hash)) => {
    //         println!("Propose Hash: {}", propose_hash);
    //         println!("Execute Hash: {}", execute_hash);
    //         println!("Simulation URL: {}", simulation_url);
    //     }
    //     Err(e) => {
    //         eprintln!("Error: {:?}", e);
    //     }
    // }

    // match approve_hash("output/tx_1.json", HardwareWalletType::TREZOR).await {
    //     Ok(tx_hash) => {
    //         println!("TX: {}", tx_hash);
    //     }
    //     Err(e) => {
    //         eprintln!("Error: {:?}", e);
    //     }
    // }

    let tx_data = serde_json::json!({
        "to": "0xFb6ec7CCBd77a42922a35D22A94fdF7fd54EE4BC",
        "value": 0,
        "data": "0x8f2a0bb000000000000000000000000000000000000000000000000000000000000",
        "operation": 0,
        "safeTxGas": 0,
        "baseGas": 0,
        "gasPrice": 0,
        "gasToken": "0x0000000000000000000000000000000000000000",
        "refundReceiver": "0x0000000000000000000000000000000000000000",
        "nonce": 38
    });

    let markdown = generate_notion_markdown(
        "Nonce 38: Propose Timelock to add Aave",
        "0x973bc1b24f52e81df238f9a7c451e9f3987deda5fceda45e4fcd67aa3f585f14",
        &tx_data,
        Some((
            "0x89a526fb2b69815032c7c59b737cef4f7275105b4e02cd4c6cc09317876cb406",
            &[
                "0xB26AEb430b5Bf6Be55763b42095E82DB9a1838B8",
                "0xE89CeE9837e6Fce3b1Ebd8E1C779b76fd6E20136",
            ],
        )),
    );

    println!("{}", markdown);
}
