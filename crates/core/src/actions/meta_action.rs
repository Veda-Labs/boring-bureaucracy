use super::sender_type::SenderType;
use crate::actions::action::Action;
use alloy::primitives::{Address, Bytes, U256};
use eyre::{Result, eyre};
use serde_json::Value;

pub trait MetaAction: Send + Sync {
    fn validate(actions: &Vec<Box<dyn Action>>) -> Result<()> {
        if actions.is_empty() {
            return Err(eyre!("MetaAction: Empty actions"));
        } else {
            let expected_sender = actions[0].sender();
            if !actions
                .iter()
                .all(|action| action.sender() == expected_sender)
            {
                return Err(eyre!(
                    "MetaAction: All actions in MetaAction must have the same sender type"
                ));
            }
        }

        Ok(())
    }
    fn is_meta() -> bool {
        true
    }
}
