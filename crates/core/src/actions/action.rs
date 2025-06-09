use super::sender_type::SenderType;
use alloy::primitives::{Address, Bytes, U256};
use serde_json::Value;

// TODO add a chain_id to send the tx on? So building blocks can be used for multichain deployment?
pub trait Action: Send + Sync {
    fn target(&self) -> Address;
    fn value(&self) -> U256 {
        U256::ZERO
    }
    fn data(&self) -> Bytes; // encode to tx data
    fn priority(&self) -> u32 {
        0
    }
    fn sender(&self) -> SenderType {
        SenderType::EOA(Address::ZERO)
    }
    fn operation(&self) -> u8 {
        0
    }
    fn describe(&self) -> Value;
}
