use alloy::primitives::{Address, Bytes};
use alloy::sol_types::SolCall;
use serde_json::{Value, json};

use crate::{
    actions::admin_action::AdminAction, bindings::accountant::AccountantWithRateProviders,
};

pub struct SetRateProviderData {
    accountant: Address,
    asset: Address,
    is_pegged_to_base: bool,
    rate_provider: Address,
}

impl SetRateProviderData {
    pub fn new(
        accountant: Address,
        asset: Address,
        is_pegged_to_base: bool,
        rate_provider: Address,
    ) -> Self {
        Self {
            accountant,
            asset,
            is_pegged_to_base,
            rate_provider,
        }
    }
}

impl AdminAction for SetRateProviderData {
    fn target(&self) -> Address {
        self.accountant
    }
    fn data(&self) -> Bytes {
        let bytes_data = AccountantWithRateProviders::setRateProviderDataCall::new((
            self.asset,
            self.is_pegged_to_base,
            self.rate_provider,
        ))
        .abi_encode();
        Bytes::from(bytes_data)
    }
    fn describe(&self) -> Value {
        json!({
            "action": "SetRateProviderData",
            "accountant": self.accountant.to_string(),
            "asset": self.asset.to_string(),
            "is_pegged_to_base": self.is_pegged_to_base,
            "rate_provider" : self.rate_provider,
        })
    }
}
