use alloy::primitives::{Address, Bytes, U256};
use alloy::sol_types::SolCall;
use serde_json::{Value, json};

use crate::{actions::action::Action, bindings::deployer::Deployer};

use super::sender_type::SenderType;

pub struct DeployContract {
    deployer: Address,
    name: String,
    creation_code: Bytes,
    constructor_args: Bytes,
    value: U256,
    priority: u32,
    sender: SenderType,
}

impl DeployContract {
    pub fn new(
        deployer: Address,
        name: String,
        creation_code: Bytes,
        constructor_args: Bytes,
        value: U256,
        priority: u32,
        sender: SenderType,
    ) -> Self {
        Self {
            deployer,
            name,
            creation_code,
            constructor_args,
            value,
            priority,
            sender,
        }
    }
}

impl Action for DeployContract {
    fn target(&self) -> Address {
        self.deployer
    }
    fn data(&self) -> Bytes {
        let bytes_data = Deployer::deployContractCall::new((
            self.name.clone(),
            self.creation_code.clone(),
            self.constructor_args.clone(),
            self.value,
        ))
        .abi_encode();
        Bytes::from(bytes_data)
    }

    fn priority(&self) -> u32 {
        self.priority
    }

    fn sender(&self) -> SenderType {
        self.sender
    }
    fn describe(&self) -> Value {
        json!({
            "action": "DeployContract",
            "deployer": self.deployer.to_string(),
            "name": self.name.to_string(),
        })
    }
}
