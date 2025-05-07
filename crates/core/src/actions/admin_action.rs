use alloy::primitives::{Address, Bytes, U256};
use serde_json::Value;

// TODO I think this needs some CallerType logic so that we know how to send the tx
// TODO I think this needs some priority logic so it knows how to order txs. IE
// A building block would mark a deploy contract action as P0, then subsequent configuration txs as P1.
// That way it knows to send the P0 first then the P1.

#[derive(Copy, Clone)]
pub enum CallerType {
    EOA(Address),
    Multisig(Address),
    Timelock(Address),
}
pub trait AdminAction: Send + Sync {
    fn target(&self) -> Address;
    fn value(&self) -> U256 {
        U256::ZERO
    }
    fn data(&self) -> Bytes; // encode to tx data
    fn get_priority(&self) -> u32 {
        0
    }
    fn get_caller(&self) -> CallerType {
        CallerType::EOA(Address::ZERO)
    }
    fn describe(&self) -> Value;
}
