use crate::{actions::action::Action, bindings::timelock::Timelock};
use alloy::primitives::{Address, B256, Bytes, U256};
use alloy::sol_types::SolCall;
use eyre::Result;
use serde_json::{Value, json};

use super::meta_action::MetaAction;

enum Mode {
    Propose,
    Execute,
}

const DEFAULT_PREDECESSOR: B256 = B256::ZERO;
const DEFAULT_SALT: B256 = B256::ZERO;

pub struct TimelockMetaAction {
    mode: Mode,
    timelock: Address,
    delay: U256,
    actions: Vec<Box<dyn Action>>,
}

impl TimelockMetaAction {
    pub fn new(timelock: Address, delay: U256, actions: Vec<Box<dyn Action>>) -> Result<Self> {
        // Make sures actions is not empty, and that all SenderType's are the same.
        Self::validate(&actions)?;
        let mode = Mode::Propose;
        Ok(Self {
            mode,
            timelock,
            delay,
            actions,
        })
    }

    pub fn toggle_mode(&mut self) {
        match self.mode {
            Mode::Propose => self.mode = Mode::Execute,
            Mode::Execute => self.mode = Mode::Propose,
        }
    }
}

impl MetaAction for TimelockMetaAction {}

impl Action for TimelockMetaAction {
    fn target(&self) -> Address {
        self.timelock
    }

    fn data(&self) -> Bytes {
        let targets = self
            .actions
            .iter()
            .map(|action| action.target())
            .collect::<Vec<_>>();
        let values = self
            .actions
            .iter()
            .map(|action| action.value())
            .collect::<Vec<_>>();
        let data = self
            .actions
            .iter()
            .map(|action| action.data())
            .collect::<Vec<_>>();
        let tx_data = match self.mode {
            Mode::Propose => Timelock::scheduleBatchCall::new((
                targets,
                values,
                data,
                DEFAULT_PREDECESSOR,
                DEFAULT_SALT,
                self.delay,
            ))
            .abi_encode(),
            Mode::Execute => Timelock::executeBatchCall::new((
                targets,
                values,
                data,
                DEFAULT_PREDECESSOR,
                DEFAULT_SALT,
            ))
            .abi_encode(),
        };

        Bytes::from(tx_data)
    }

    fn describe(&self) -> Value {
        match self.mode {
            Mode::Propose => {
                json!({
                    "action": "ProposeBatch",
                    "timelock": self.timelock.to_string(),
                    "inner": self.actions.iter().map(|action| action.describe()).collect::<Vec<_>>()
                })
            }
            Mode::Execute => {
                json!({
                    "action": "ExecuteBatch",
                    "timelock": self.timelock.to_string(),
                    "inner": self.actions.iter().map(|action| action.describe()).collect::<Vec<_>>()
                })
            }
        }
    }
}
