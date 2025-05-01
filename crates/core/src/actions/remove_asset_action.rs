use alloy::primitives::{Address, Bytes};
use alloy::sol_types::SolCall;
use serde_json::{Value, json};

use crate::{actions::admin_action::AdminAction, bindings::teller::TellerWithMultiAssetSupport};

pub struct RemoveAsset {
    teller: Address,
    asset: Address,
}

impl RemoveAsset {
    pub fn new(teller: Address, asset: Address) -> Self {
        Self { teller, asset }
    }
}

impl AdminAction for RemoveAsset {
    fn target(&self) -> Address {
        self.teller
    }
    fn data(&self) -> Bytes {
        let bytes_data =
            TellerWithMultiAssetSupport::removeAssetCall::new((self.asset,)).abi_encode();
        Bytes::from(bytes_data)
    }
    fn describe(&self) -> Value {
        json!({
            "action": "RemoveAsset",
            "teller": self.teller.to_string(),
            "asset": self.asset.to_string(),
        })
    }
}
