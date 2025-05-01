use alloy::primitives::{Address, Bytes};
use alloy::sol_types::SolCall;
use serde_json::{Value, json};

use crate::{actions::admin_action::AdminAction, bindings::roles_authority::RolesAuthority};

pub struct SetUserRoleAction {
    roles_authority: Address,
    user: Address,
    role: u8,
    enabled: bool,
}

impl SetUserRoleAction {
    pub fn new(roles_authority: Address, user: Address, role: u8, enabled: bool) -> Self {
        Self {
            roles_authority,
            user,
            role,
            enabled,
        }
    }
}

impl AdminAction for SetUserRoleAction {
    fn target(&self) -> Address {
        self.roles_authority
    }
    fn data(&self) -> Bytes {
        let bytes_data =
            RolesAuthority::setUserRoleCall::new((self.user, self.role, self.enabled)).abi_encode();
        Bytes::from(bytes_data)
    }
    fn describe(&self) -> Value {
        json!({
            "action": "SetUserRoleAction",
            "roles_authority": self.roles_authority.to_string(),
            "user": self.user.to_string(),
            "role": self.role.to_string(),
            "enabled": self.enabled.to_string(),
        })
    }
}
