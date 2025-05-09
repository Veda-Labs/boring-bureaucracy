use crate::actions::action::Action;
use eyre::{Result, eyre};

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
                    "MetaAction: All actions in MetaAction must have the same sender type and address"
                ));
            }
        }

        Ok(())
    }
    fn is_meta() -> bool {
        true
    }
}
