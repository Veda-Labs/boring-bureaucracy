use alloy::primitives::{Address, Bytes, U256};
use alloy::sol_types::SolCall;
use serde_json::{Value, json};

use crate::{actions::admin_action::AdminAction, bindings::deployer::Deployer};

use super::admin_action::CallerType;

pub struct DeployContract {
    deployer: Address,
    name: String,
    creation_code: Bytes,
    constructor_args: Bytes,
    value: U256,
    priority: u32,
    caller: CallerType,
}

impl DeployContract {
    pub fn new(
        deployer: Address,
        name: String,
        creation_code: Bytes,
        constructor_args: Bytes,
        value: U256,
        priority: u32,
        caller: CallerType,
    ) -> Self {
        Self {
            deployer,
            name,
            creation_code,
            constructor_args,
            value,
            priority,
            caller,
        }
    }
}

impl AdminAction for DeployContract {
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

    fn get_priority(&self) -> u32 {
        self.priority
    }

    fn get_caller(&self) -> CallerType {
        self.caller
    }
    fn describe(&self) -> Value {
        json!({
            "action": "DeployContract",
            "deployer": self.deployer.to_string(),
            "name": self.name.to_string(),
        })
    }
}
