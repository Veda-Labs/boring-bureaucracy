use super::meta_action::MetaAction;
use super::sender_type::SenderType;
use crate::{actions::action::Action, bindings::bundler::Bundler};
use alloy::primitives::{Address, Bytes};
use alloy::sol_types::SolCall;
use eyre::Result;
use serde_json::{Value, json};

pub struct BundlerMetaAction {
    bundler: Address,
    actions: Vec<Box<dyn Action>>,
    signer: SenderType,
}

impl BundlerMetaAction {
    pub fn new(executor: Address, bundler: Address, actions: Vec<Box<dyn Action>>) -> Result<Self> {
        // Validate all actions have the same sender type and address
        Self::validate(&actions)?;
        Ok(Self {
            bundler,
            actions,
            signer: SenderType::EOA(executor),
        })
    }
}

impl MetaAction for BundlerMetaAction {}

impl Action for BundlerMetaAction {
    fn target(&self) -> Address {
        self.bundler
    }

    fn data(&self) -> Bytes {
        let mut txs: Vec<Bundler::Tx> = Vec::new();
        for action in &self.actions {
            txs.push(Bundler::Tx::from((
                action.target(),
                action.data(),
                action.value(),
            )));
        }
        Bytes::from(Bundler::bundleTxsCall::new((txs,)).abi_encode())
    }

    fn priority(&self) -> u32 {
        1
    }

    fn sender(&self) -> SenderType {
        self.signer
    }

    fn operation(&self) -> u8 {
        1
    }

    fn describe(&self) -> Value {
        json!({
            "action": "Bundler",
            "bundler": self.bundler.to_string(),
            "inner": self.actions.iter().map(|action| action.describe()).collect::<Vec<_>>()
        })
    }
}
