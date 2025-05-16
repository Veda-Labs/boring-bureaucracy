use alloy::primitives::{Address, Bytes};
use alloy::sol_types::SolCall;
use serde_json::{Value, json};

use crate::{
    actions::admin_action::AdminAction, bindings::accountant::AccountantWithRateProviders,
};

pub struct UpdatePerformanceFee {
    accountant: Address,
    fee: u16,
}

impl UpdatePerformanceFee {
    pub fn new(accountant: Address, fee: u16) -> Self {
        Self { accountant, fee }
    }
}

impl AdminAction for UpdatePerformanceFee {
    fn target(&self) -> Address {
        self.accountant
    }
    fn data(&self) -> Bytes {
        let bytes_data =
            AccountantWithRateProviders::updatePerformanceFeeCall::new((self.fee,)).abi_encode();
        Bytes::from(bytes_data)
    }
    fn describe(&self) -> Value {
        json!({
            "action": "UpdatePerformanceFee",
            "accountant": self.accountant.to_string(),
            "fee": self.fee.to_string(),
        })
    }
}
