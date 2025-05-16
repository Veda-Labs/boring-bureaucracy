use alloy::primitives::{Address, Bytes};
use alloy::sol_types::SolCall;
use serde_json::{Value, json};

use crate::{
    actions::admin_action::AdminAction, bindings::accountant::AccountantWithRateProviders,
};

pub struct UpdatePlatformFee {
    accountant: Address,
    fee: u16,
}

impl UpdatePlatformFee {
    pub fn new(accountant: Address, fee: u16) -> Self {
        Self { accountant, fee }
    }
}

impl AdminAction for UpdatePlatformFee {
    fn target(&self) -> Address {
        self.accountant
    }
    fn data(&self) -> Bytes {
        let bytes_data =
            AccountantWithRateProviders::updatePlatformFeeCall::new((self.fee,)).abi_encode();
        Bytes::from(bytes_data)
    }
    fn describe(&self) -> Value {
        json!({
            "action": "UpdatePlatformFee",
            "accountant": self.accountant.to_string(),
            "fee": self.fee.to_string(),
        })
    }
}
