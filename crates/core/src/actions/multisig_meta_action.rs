use super::meta_action::MetaAction;
use super::sender_type::SenderType;
use crate::utils::view_request_manager::ViewRequestManager;
use crate::{actions::action::Action, bindings::multisig::GnosisSafe};
use alloy::primitives::{Address, B256, Bytes, U256};
use alloy::sol_types::SolCall;
use eyre::{Result, eyre};
use serde_json::{Value, json};

enum Mode {
    ApproveHash,
    ExecTransaction,
}

const SAFE_TX_GAS: U256 = U256::ZERO;
const BASE_GAS: U256 = U256::ZERO;
const GAS_PRICE: U256 = U256::ZERO;
const GAS_TOKEN: Address = Address::ZERO;
const REFUND_RECEIVER: Address = Address::ZERO;
// This will either make an approveHash or an execTransaction call
pub struct MultisigMetaAction {
    multisig: Address,
    action: Box<dyn Action>,
    nonce: U256,
    safe_hash: B256,
    signer: SenderType,
    mode: Mode,
    signature: Option<Bytes>,
}

impl MultisigMetaAction {
    pub async fn new(
        multisig: Address,
        signer: Address,
        action: Box<dyn Action>,
        action_nonce: Option<U256>,
        vrm: &ViewRequestManager,
    ) -> Result<Self> {
        // Make sure actions is not empty, and that all SenderType's are the same.
        if action.sender() != SenderType::Signer(multisig) {
            return Err(eyre!("MultisigMetaAction: Wrong SenderType"));
        }

        // Check that the signer is an owner.
        let calldata = Bytes::from(GnosisSafe::getOwnersCall::new(()).abi_encode());
        let result = vrm.request(multisig, calldata).await?;
        let owners = GnosisSafe::getOwnersCall::abi_decode_returns(&result, true)?.owners;
        if !owners.contains(&signer) {
            return Err(eyre!("MultisigMetaAction: Signer is not an owner"));
        }
        // Either use provided nonce or read from safe.
        let nonce = if let Some(nonce) = action_nonce {
            nonce
        } else {
            // Read nonce from safe.
            let calldata = Bytes::from(GnosisSafe::nonceCall::new(()).abi_encode());
            let result = vrm.request(multisig, calldata).await?;
            GnosisSafe::nonceCall::abi_decode_returns(&result, true)?.nonce
        };
        // Get safe hash.
        let calldata = Bytes::from(
            GnosisSafe::getTransactionHashCall::new((
                action.target(),
                action.value(),
                action.data(),
                action.operation(),
                SAFE_TX_GAS,
                BASE_GAS,
                GAS_PRICE,
                GAS_TOKEN,
                REFUND_RECEIVER,
                nonce,
            ))
            .abi_encode(),
        );
        let result = vrm.request(multisig, calldata).await?;
        let safe_hash = GnosisSafe::getTransactionHashCall::abi_decode_returns(&result, true)?._0;

        // Determine mode by seeing what owners have approved hash.
        let mut owner_statuses = Vec::new();
        for owner in owners {
            let calldata =
                Bytes::from(GnosisSafe::approvedHashesCall::new((owner, safe_hash)).abi_encode());
            let result = vrm.request(multisig, calldata).await?;
            let signed = match GnosisSafe::approvedHashesCall::abi_decode_returns(&result, true)?._0
            {
                U256::ZERO => false,
                _ => true,
            };
            owner_statuses.push((owner, signed));
        }

        // Get safe threshold.
        let calldata = Bytes::from(GnosisSafe::getThresholdCall::new(()).abi_encode());
        let result = vrm.request(multisig, calldata).await?;
        let threshold = GnosisSafe::getThresholdCall::abi_decode_returns(&result, true)?
            .threshold
            .try_into()
            .unwrap();

        let mut signer_signed = false;
        let mut signature_count = 0;
        for status in &owner_statuses {
            if status.1 {
                signature_count += 1;
                if status.0 == signer {
                    signer_signed = true;
                }
            }
        }

        let mode = if signature_count >= threshold {
            // We have enough signatures.
            Mode::ExecTransaction
        } else if (signature_count == (threshold - 1)) && !signer_signed {
            // Missing one signature but signer can sign.
            Mode::ExecTransaction
        } else if !signer_signed {
            // Signer has not signed.
            Mode::ApproveHash
        } else {
            return Err(eyre!(
                "MultisigMetaAction: Signer already approved, but not enough signers."
            ));
        };
        // Create signature.
        let signature = match mode {
            Mode::ApproveHash => None,
            Mode::ExecTransaction => {
                let mut sig = Vec::new();
                owner_statuses.sort();
                let mut count = 0;
                for status in owner_statuses {
                    if status.1 {
                        // r
                        sig.extend_from_slice(status.0.into_word().as_slice());
                        // s
                        sig.extend_from_slice(&[0u8; 32]);
                        // v
                        sig.push(1);
                        count += 1;
                        if count == threshold {
                            break;
                        }
                    }
                }
                Some(Bytes::from(sig))
            }
        };
        Ok(Self {
            multisig,
            action,
            signer: SenderType::EOA(signer),
            nonce,
            safe_hash,
            mode,
            signature,
        })
    }
}

impl MetaAction for MultisigMetaAction {}

impl Action for MultisigMetaAction {
    fn target(&self) -> Address {
        self.multisig
    }

    fn data(&self) -> Bytes {
        let tx_data = match self.mode {
            Mode::ApproveHash => GnosisSafe::approveHashCall::new((self.safe_hash,)).abi_encode(),
            Mode::ExecTransaction => {
                GnosisSafe::execTransactionCall::new((
                    self.action.target(),
                    self.action.value(),
                    self.action.data(),
                    self.action.operation(),
                    SAFE_TX_GAS,
                    BASE_GAS,
                    GAS_PRICE,
                    GAS_TOKEN,
                    REFUND_RECEIVER,
                    self.signature.as_ref().unwrap().clone(),
                ))
            }
            .abi_encode(),
        };

        Bytes::from(tx_data)
    }

    fn sender(&self) -> SenderType {
        self.signer
    }

    fn describe(&self) -> Value {
        match self.mode {
            Mode::ApproveHash => {
                json!({
                    "action": "ApproveHash",
                    "multisig": self.multisig.to_string(),
                    "nonce": self.nonce,
                    "safe_hash": self.safe_hash,
                    "inner": self.action.describe()
                })
            }
            Mode::ExecTransaction => {
                json!({
                    "action": "ExecTransaction",
                    "multisig": self.multisig.to_string(),
                    "nonce": self.nonce,
                    "safe_hash": self.safe_hash,
                    "inner": self.action.describe()
                })
            }
        }
    }
}
