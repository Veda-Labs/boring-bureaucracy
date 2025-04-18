use alloy::primitives::{Address, B256, Bytes, U256};
use alloy::sol_types::SolCall;
use serde_json::{Value, json};

use crate::{actions::admin_action::AdminAction, bindings::timelock::Timelock};

enum Mode {
    Propose,
    Execute,
}

const DEFAULT_PREDECESSOR: B256 = B256::ZERO;
const DEFAULT_SALT: B256 = B256::ZERO;

pub struct TimelockAction {
    mode: Mode,
    timelock: Address,
    delay: U256,
    actions: Vec<Box<dyn AdminAction>>,
}

impl TimelockAction {
    pub fn new(timelock: Address, delay: U256, actions: Vec<Box<dyn AdminAction>>) -> Self {
        let mode = Mode::Propose;
        Self {
            mode,
            timelock,
            delay,
            actions,
        }
    }

    pub fn toggle_mode(&mut self) {
        match self.mode {
            Mode::Propose => self.mode = Mode::Execute,
            Mode::Execute => self.mode = Mode::Propose,
        }
    }
}

impl AdminAction for TimelockAction {
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
