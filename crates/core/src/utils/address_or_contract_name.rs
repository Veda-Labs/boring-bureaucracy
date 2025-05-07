use alloy::primitives::Address;
use serde::{Deserialize, Deserializer, de::Error};

#[derive(Debug, PartialEq)]
pub enum AddressOrContractName {
    Address(Address),
    ContractName(String),
}

impl<'de> Deserialize<'de> for AddressOrContractName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        // Try to parse as Address first
        if let Ok(addr) = s.parse::<Address>() {
            Ok(AddressOrContractName::Address(addr))
        } else if s.starts_with("0x") {
            return Err(D::Error::custom(
                "Invalid address: string starts with '0x' but is not a valid address (possible typo or wrong length)",
            ));
        } else {
            Ok(AddressOrContractName::ContractName(s))
        }
    }
}
