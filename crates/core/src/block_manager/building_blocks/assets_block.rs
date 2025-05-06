use alloy::primitives::Address;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AssetsBlock {
    pub assets: Vec<Address>,
    // ...other fields
}
