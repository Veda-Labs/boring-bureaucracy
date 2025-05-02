use alloy::primitives::{Address, Bytes, Uint};
use alloy::sol_types::SolCall;
use serde_json::{Value, json};

use crate::{actions::admin_action::AdminAction, bindings::boring_queue::BoringOnChainQueue};

pub struct UpdateWithdrawAsset {
    queue: Address,
    asset: Address,
    seconds_to_maturity: Uint<24, 1>,
    minimum_seconds_to_deadline: Uint<24, 1>,
    min_discount: u16,
    max_discount: u16,
    minimum_shares: Uint<96, 2>,
}

impl UpdateWithdrawAsset {
    pub fn new(
        queue: Address,
        asset: Address,
        seconds_to_maturity: u32,
        minimum_seconds_to_deadline: u32,
        min_discount: u16,
        max_discount: u16,
        minimum_shares: u128,
    ) -> Self {
        let seconds_to_maturity = Uint::<24, 1>::from(seconds_to_maturity);
        let minimum_seconds_to_deadline = Uint::<24, 1>::from(minimum_seconds_to_deadline);
        let minimum_shares = Uint::<96, 2>::from(minimum_shares);
        Self {
            queue,
            asset,
            seconds_to_maturity,
            minimum_seconds_to_deadline,
            min_discount,
            max_discount,
            minimum_shares,
        }
    }
}

impl AdminAction for UpdateWithdrawAsset {
    fn target(&self) -> Address {
        self.queue
    }
    fn data(&self) -> Bytes {
        let bytes_data = BoringOnChainQueue::updateWithdrawAssetCall::new((
            self.asset,
            self.seconds_to_maturity,
            self.minimum_seconds_to_deadline,
            self.min_discount,
            self.max_discount,
            self.minimum_shares,
        ))
        .abi_encode();
        Bytes::from(bytes_data)
    }
    fn describe(&self) -> Value {
        json!({
            "action": "UpdateWithdrawAsset",
            "queue": self.queue.to_string(),
            "asset": self.asset.to_string(),
            "seconds_to_maturity": self.seconds_to_maturity.to_string(),
            "minimum_seconds_to_deadline": self.minimum_seconds_to_deadline.to_string(),
            "min_discount": self.min_discount,
            "max_discount": self.max_discount,
            "minimum_shares": self.minimum_shares.to_string(),
        })
    }
}
