use alloy::primitives::{Address, Bytes, FixedBytes, aliases::B32, keccak256};
use alloy::sol_types::SolCall;
use serde_json::{Value, json};

use crate::{actions::admin_action::AdminAction, bindings::roles_authority::RolesAuthority};

pub struct SetPublicCapabilityAction {
    roles_authority: Address,
    target: Address,
    function_signature: String,
    function_selector: B32,
    enabled: bool,
}

impl SetPublicCapabilityAction {
    pub fn new(
        roles_authority: Address,
        target: Address,
        function_signature: String,
        enabled: bool,
    ) -> Self {
        let function_selector =
            FixedBytes::<4>::from_slice(&keccak256(function_signature.as_bytes())[..4].to_vec());
        Self {
            roles_authority,
            target,
            function_signature,
            function_selector,
            enabled,
        }
    }
}

impl AdminAction for SetPublicCapabilityAction {
    fn target(&self) -> Address {
        self.roles_authority
    }
    fn data(&self) -> Bytes {
        let bytes_data = RolesAuthority::setPublicCapabilityCall::new((
            self.target,
            self.function_selector,
            self.enabled,
        ))
        .abi_encode();
        Bytes::from(bytes_data)
    }
    fn describe(&self) -> Value {
        json!({
            "action": "SetPublicCapabilityAction",
            "roles_authority": self.roles_authority.to_string(),
            "target": self.target.to_string(),
            "function_signature": self.function_signature.to_string(),
            "function_selector": self.function_selector.to_string(),
            "enabled": self.enabled.to_string(),
        })
    }
}
