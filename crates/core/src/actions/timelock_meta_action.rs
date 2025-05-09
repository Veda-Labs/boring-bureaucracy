use super::sender_type::SenderType;
use crate::utils::view_request_manager::ViewRequestManager;
use crate::{actions::action::Action, bindings::timelock::Timelock};
use alloy::primitives::{Address, B256, Bytes, U256};
use alloy::sol_types::SolCall;
use eyre::{Result, eyre};
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
    sender: SenderType,
}

// TODO if action length 1, can use non batch functions
impl TimelockMetaAction {
    pub async fn new(
        delay: Option<U256>,
        actions: Vec<Box<dyn Action>>,
        vrm: &ViewRequestManager,
        timelock_admin: SenderType,
    ) -> Result<Self> {
        // Make sure actions is not empty, and that all SenderType's are the same.
        Self::validate(&actions)?;

        let timelock = match actions[0].sender() {
            SenderType::Timelock(addr) => addr,
            _ => {
                return Err(eyre!("TimelockMetaAction: Wrong SenderType"));
            }
        };

        // TODO verify that timelock_admin does have the proposer and executor role
        // Verify timelock_admin.
        // Should be of type EOA or multisig.
        match timelock_admin {
            SenderType::EOA(_) => {}
            SenderType::Multisig(_) => {}
            _ => {
                return Err(eyre!(
                    "TimelockMetaAction: Timelock admin must be EOA or Multisig"
                ));
            }
        }

        // Handle delay.
        let calldata = Bytes::from(Timelock::getMinDelayCall::new(()).abi_encode());
        let result = vrm.request(timelock, calldata).await?;
        let min_delay = Timelock::getMinDelayCall::abi_decode_returns(&result, true)?.delay;
        let delay = if let Some(delay) = delay {
            if delay < min_delay {
                return Err(eyre!(
                    "TimelockMetaAction: Provided delay does not meet minimum"
                ));
            }
            delay
        } else {
            min_delay
        };

        let targets = actions
            .iter()
            .map(|action| action.target())
            .collect::<Vec<_>>();
        let values = actions
            .iter()
            .map(|action| action.value())
            .collect::<Vec<_>>();
        let data = actions
            .iter()
            .map(|action| action.data())
            .collect::<Vec<_>>();

        // Get operation hash.
        let calldata = Bytes::from(
            Timelock::hashOperationBatchCall::new((
                targets,
                values,
                data,
                DEFAULT_PREDECESSOR,
                DEFAULT_SALT,
            ))
            .abi_encode(),
        );
        let result = vrm.request(timelock, calldata).await?;
        let id = Timelock::hashOperationBatchCall::abi_decode_returns(&result, true)?.id;

        // Check if operation is ready.
        let calldata = Bytes::from(Timelock::isOperationReadyCall::new((id,)).abi_encode());
        let result = vrm.request(timelock, calldata).await?;
        let is_ready = Timelock::isOperationReadyCall::abi_decode_returns(&result, true)?._0;
        let mode = if is_ready {
            Mode::Execute
        } else {
            // Check if it needs to be queued.
            let calldata = Bytes::from(Timelock::isOperationPendingCall::new((id,)).abi_encode());
            let result = vrm.request(timelock, calldata).await?;
            let is_pending =
                Timelock::isOperationPendingCall::abi_decode_returns(&result, true)?._0;
            if is_pending {
                return Err(eyre!("TimelockMetaAction: Operation is still pending"));
            } else {
                Mode::Propose
            }
        };
        Ok(Self {
            mode,
            timelock,
            delay,
            actions,
            sender: timelock_admin,
        })
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

    fn sender(&self) -> SenderType {
        self.sender
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
