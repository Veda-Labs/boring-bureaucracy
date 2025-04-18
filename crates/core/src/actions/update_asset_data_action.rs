use alloy::primitives::{Address, Bytes};
use alloy::sol_types::SolCall;
use serde_json::{Value, json};

use crate::{actions::admin_action::AdminAction, bindings::teller::TellerWithMultiAssetSupport};

pub struct UpdateAssetData {
    teller: Address,
    asset: Address,
    allow_deposits: bool,
    allow_withdraws: bool,
    share_premium: u16,
}

impl UpdateAssetData {
    pub fn new(
        teller: Address,
        asset: Address,
        allow_deposits: bool,
        allow_withdraws: bool,
        share_premium: u16,
    ) -> Self {
        Self {
            teller,
            asset,
            allow_deposits,
            allow_withdraws,
            share_premium,
        }
    }
}

impl AdminAction for UpdateAssetData {
    fn target(&self) -> Address {
        self.teller
    }
    fn data(&self) -> Bytes {
        let bytes_data = TellerWithMultiAssetSupport::updateAssetDataCall::new((
            self.asset,
            self.allow_deposits,
            self.allow_withdraws,
            self.share_premium,
        ))
        .abi_encode();
        Bytes::from(bytes_data)
    }
    fn describe(&self) -> Value {
        json!({
            "action": "UpdateAssetData",
            "teller": self.teller.to_string(),
            "asset": self.asset.to_string(),
            "allow_deposits": self.allow_deposits,
            "allow_withdraws" : self.allow_withdraws,
            "share_premium" : self.share_premium,
        })
    }
}
