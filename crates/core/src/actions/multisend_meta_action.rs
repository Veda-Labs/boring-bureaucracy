use super::meta_action::MetaAction;
use super::sender_type::SenderType;
use crate::{actions::action::Action, bindings::multisend::MutliSendCallOnly};
use alloy::primitives::{Address, Bytes, U256};
use alloy::sol_types::SolCall;
use eyre::{Result, eyre};
use serde_json::{Value, json};

pub struct MultisendMetaAction {
    multisend: Option<Address>,
    actions: Vec<Box<dyn Action>>,
    signer: SenderType,
}

// TODO if action length 1, just make direct call, so use action.target(), etc.
impl MultisendMetaAction {
    pub fn new(
        multisig: Address,
        multisend: Option<Address>,
        actions: Vec<Box<dyn Action>>,
    ) -> Result<Self> {
        // Validate all actions have the same sender type and address
        Self::validate(&actions)?;
        // We know the action sender type is multisig with the appropriate multisig, from the match arm in block_manager
        // let multisig = match actions[0].sender() {
        //     SenderType::Multisig(addr) => addr,
        //     _ => {
        //         return Err(eyre!("MultisendMetaAction: Wrong SenderType"));
        //     }
        // };
        if actions.len() > 1 {
            // Use multisend to make multiple calls.
            if multisend.is_none() {
                return Err(eyre!(
                    "MultisendMetaAction: multisend is not defined and action length is {}",
                    actions.len()
                ));
            }
        }
        Ok(Self {
            multisend,
            actions,
            signer: SenderType::Signer(multisig),
        })
    }
}

impl MetaAction for MultisendMetaAction {}

impl Action for MultisendMetaAction {
    fn target(&self) -> Address {
        if self.actions.len() > 1 {
            self.multisend.unwrap()
        } else {
            self.actions[0].target()
        }
    }

    fn data(&self) -> Bytes {
        if self.actions.len() > 1 {
            let mut encoded_transactions = Vec::new();
            for action in &self.actions {
                // operation (0 for Call) - 1 byte
                encoded_transactions.push(0u8);

                // to address - 20 bytes
                encoded_transactions.extend_from_slice(&action.target().as_slice());

                // value - 32 bytes
                encoded_transactions.extend_from_slice(&action.value().to_be_bytes::<32>());

                // data length - 32 bytes
                let data_len = U256::from(action.data().len());
                encoded_transactions.extend_from_slice(&data_len.to_be_bytes::<32>());

                // data - dynamic length
                encoded_transactions.extend_from_slice(&action.data());
            }

            let multisend_data =
                MutliSendCallOnly::multiSendCall::new((Bytes::from(encoded_transactions),))
                    .abi_encode();

            Bytes::from(multisend_data)
        } else {
            self.actions[0].data()
        }
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
        if self.actions.len() > 1 {
            json!({
                "action": "MultiSend",
                "multisend": self.multisend.unwrap().to_string(),
                "inner": self.actions.iter().map(|action| action.describe()).collect::<Vec<_>>()
            })
        } else {
            json!({
                "action": "DirectCall",
                "inner": self.actions.iter().map(|action| action.describe()).collect::<Vec<_>>()
            })
        }
    }
}
