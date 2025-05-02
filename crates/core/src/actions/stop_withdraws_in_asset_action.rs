use alloy::primitives::{Address, Bytes};
use alloy::sol_types::SolCall;
use serde_json::{Value, json};

use crate::{actions::admin_action::AdminAction, bindings::boring_queue::BoringOnChainQueue};

pub struct StopWithdrawsInAsset {
    queue: Address,
    asset: Address,
}

impl StopWithdrawsInAsset {
    pub fn new(queue: Address, asset: Address) -> Self {
        Self { queue, asset }
    }
}

impl AdminAction for StopWithdrawsInAsset {
    fn target(&self) -> Address {
        self.queue
    }
    fn data(&self) -> Bytes {
        let bytes_data =
            BoringOnChainQueue::stopWithdrawsInAssetCall::new((self.asset,)).abi_encode();
        Bytes::from(bytes_data)
    }
    fn describe(&self) -> Value {
        json!({
            "action": "StopWithdrawsInAsset",
            "queue": self.queue.to_string(),
            "asset": self.asset.to_string(),
        })
    }
}
