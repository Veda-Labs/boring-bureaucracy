use alloy::primitives::{Address, Bytes, U256};
use eyre::Result;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize)]
pub struct SimulationConfig {
    pub network_id: u32,
    pub multisig: String,
    pub to: String,
    pub value: String,
    pub data: String,
    pub operation: u8,
    pub nonce: u32,
}

impl SimulationConfig {
    pub fn from_file(file_path: &str) -> Result<Self> {
        let file_content = fs::read_to_string(file_path)?;
        let config: SimulationConfig = serde_json::from_str(&file_content)?;
        Ok(config)
    }

    pub fn multisig(&self) -> Address {
        self.multisig.parse().expect("Failed to parse multisig")
    }

    pub fn to(&self) -> Address {
        self.to.parse().expect("Failed to parse to")
    }

    pub fn value(&self) -> U256 {
        U256::from(self.value.parse::<U256>().expect("Failed to parse value"))
    }

    pub fn data(&self) -> Bytes {
        Bytes::from(self.data.parse::<Bytes>().expect("Failed to parse data"))
    }
}
