use alloy::primitives::Address;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TellerBlock {
    pub teller: Address,
    // ...other fields
}
