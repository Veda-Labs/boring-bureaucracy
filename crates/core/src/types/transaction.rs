use alloy::primitives::{Address, Bytes, U256};

#[derive(Clone)]
pub struct Transaction {
    pub to: Address,
    pub value: U256,
    pub data: Bytes,
}
