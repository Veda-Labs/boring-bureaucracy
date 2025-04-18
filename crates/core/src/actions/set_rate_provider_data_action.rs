use alloy::primitives::{Address, Bytes};
use alloy::sol_types::SolCall;
use serde_json::{Value, json};

use crate::{
    actions::admin_action::AdminAction, bindings::accountant::AccountantWithRateProviders,
};

pub struct SetRateProviderData {
    accountant: Address,
    asset: Address,
    rate_provider: Option<Address>,
}

impl SetRateProviderData {
    pub fn new(accountant: Address, asset: Address, rate_provider: Option<Address>) -> Self {
        Self {
            accountant,
            asset,
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
            self.rate_provider.is_none(),
            self.rate_provider.unwrap_or_default(),
        ))
        .abi_encode();
        Bytes::from(bytes_data)
    }
    fn describe(&self) -> Value {
        json!({
            "action": "SetRateProviderData",
            "accountant": self.accountant.to_string(),
            "asset": self.asset.to_string(),
            "is_pegged_to_base": self.rate_provider.is_none(),
            "rate_provider" : self.rate_provider.unwrap_or_default(),
        })
    }
}
