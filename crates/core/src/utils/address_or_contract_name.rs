use alloy::dyn_abi::SolType;
use alloy::primitives::{Address, b256, keccak256};
use alloy::sol_types::sol_data;
use eyre::{Result, eyre};
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

impl AddressOrContractName {
    pub fn resolve_to_address(&self, deployer: Option<Address>) -> Result<Address> {
        match self {
            AddressOrContractName::Address(addr) => Ok(*addr),
            AddressOrContractName::ContractName(s) => {
                if let Some(deployer_addr) = deployer {
                    Ok(derive_contract_address(&s, deployer_addr))
                } else {
                    Err(eyre!(
                        "block_utils: Cannot resolve contract name to address without deployer"
                    ))
                }
            }
        }
    }
}

pub fn derive_contract_address(name: &str, deployer: Address) -> Address {
    // Step 1: Convert name to bytes32 (keccak256 of the name)
    type SolString = sol_data::String;
    let encoded = SolString::abi_encode(&name);

    let salt = keccak256(encoded);

    let proxy_bytecode_hash =
        b256!("0x21c35dbe1b344a2488cf3321d6ce542f8e9f305544ff09e4993a62319a497c1f");

    // Step 2: Get the proxy address
    let proxy = keccak256(
        &[
            &[0xFF], // Prefix
            deployer.as_slice(),
            salt.as_slice(),
            proxy_bytecode_hash.as_slice(),
        ]
        .concat(),
    );

    // Step 3: Get the final contract address
    let contract = keccak256(
        &[
            &[0xd6, 0x94], // RLP prefix
            &proxy[12..],  // Last 20 bytes of proxy
            &[0x01],       // Nonce
        ]
        .concat(),
    );

    Address::from_slice(&contract[12..])
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::address;

    #[test]
    fn test_address_derivation() {
        let name = "MyContract";
        let deployer = address!("0x5F2F11ad8656439d5C14d9B351f8b09cDaC2A02d");

        let derived = derive_contract_address(name, deployer);
        let expected = address!("0x640c33CB461cD8ec1934a36c6335294AcB0ADc13");
        assert_eq!(derived, expected, "Derived should match expected");
    }
}
