use alloy::primitives::{Address, B256, Bytes};
use alloy::sol_types::SolCall;
use serde_json::{Value, json};

use crate::{actions::admin_action::AdminAction, bindings::manager::ManagerWithMerkleVerification};

pub struct SetMerkleRoot {
    manager: Address,
    strategist: Address,
    new_root: B256,
}

impl SetMerkleRoot {
    pub fn new(manager: Address, strategist: Address, new_root: B256) -> Self {
        Self {
            manager,
            strategist,
            new_root,
        }
    }
}

impl AdminAction for SetMerkleRoot {
    fn target(&self) -> Address {
        self.manager
    }
    fn data(&self) -> Bytes {
        let bytes_data =
            ManagerWithMerkleVerification::setManageRootCall::new((self.strategist, self.new_root))
                .abi_encode();
        Bytes::from(bytes_data)
    }
    fn describe(&self) -> Value {
        json!({
            "action": "SetMerkleRoot",
            "manager": self.manager.to_string(),
            "strategist": self.strategist.to_string(),
            "root": self.new_root.to_string(),
        })
    }
}
