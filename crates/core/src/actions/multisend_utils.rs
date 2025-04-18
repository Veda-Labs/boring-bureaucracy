use crate::bindings::multisend::MutliSendCallOnly;
use crate::types::transaction::Transaction;
use alloy::primitives::{Bytes, U256};
use alloy::sol_types::SolCall;

pub fn create_multisend_data(txs: Vec<Transaction>) -> Bytes {
    let mut encoded_transactions = Vec::new();
    for i in 0..txs.len() {
        // operation (0 for Call) - 1 byte
        encoded_transactions.push(0u8);

        // to address - 20 bytes
        encoded_transactions.extend_from_slice(&txs[i].to.as_slice());

        // value - 32 bytes
        encoded_transactions.extend_from_slice(&txs[i].value.to_be_bytes::<32>());

        // data length - 32 bytes
        let data_len = U256::from(txs[i].data.len());
        encoded_transactions.extend_from_slice(&data_len.to_be_bytes::<32>());

        // data - dynamic length
        encoded_transactions.extend_from_slice(&txs[i].data);
    }

    let multisend_data =
        MutliSendCallOnly::multiSendCall::new((Bytes::from(encoded_transactions),)).abi_encode();

    Bytes::from(multisend_data)
}
