use alloy::primitives::{Address, Bytes, U256};
use serde_json::Value;

pub trait AdminAction {
    fn target(&self) -> Address;
    fn value(&self) -> U256 {
        U256::ZERO
    }
    fn data(&self) -> Bytes; // encode to tx data
    fn describe(&self) -> Value;
}
