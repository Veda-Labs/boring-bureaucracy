use alloy::primitives::Address;
use std::cmp::Ordering;

// TODO think I am missing a sender type, for the deployer tx bundler
// Which really each of these should correspond to a meta action that knows how to convert actions into ones with a sender that is a higher level up.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum SenderType {
    EOA(Address),      // User address
    Signer(Address),   // Multisig Address
    Multisig(Address), // Multisig Address
    Timelock(Address), // Timelock Address
}

impl Ord for SenderType {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            // Same type, compare addresses
            (SenderType::Timelock(a), SenderType::Timelock(b)) => a.cmp(b),
            (SenderType::Multisig(a), SenderType::Multisig(b)) => a.cmp(b),
            (SenderType::Signer(a), SenderType::Signer(b)) => a.cmp(b),
            (SenderType::EOA(a), SenderType::EOA(b)) => a.cmp(b),

            // Different types, use enum order
            (SenderType::Timelock(_), _) => Ordering::Less,
            (_, SenderType::Timelock(_)) => Ordering::Greater,
            (SenderType::Multisig(_), _) => Ordering::Less,
            (_, SenderType::Multisig(_)) => Ordering::Greater,
            (SenderType::Signer(_), _) => Ordering::Less,
            (_, SenderType::Signer(_)) => Ordering::Greater,
            // Already handled all permutations.
            // (SenderType::EOA(_), _) => Ordering::Less,
            // (_, SenderType::EOA(_)) => Ordering::Greater,
        }
    }
}

impl PartialOrd for SenderType {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
