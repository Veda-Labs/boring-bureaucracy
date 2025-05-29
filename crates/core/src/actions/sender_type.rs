use alloy::primitives::Address;
use std::cmp::Ordering;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum SenderType {
    EOA(Address),      // User address
    Signer(Address),   // Multisig Address
    Multisig(Address), // Multisig Address
    Timelock(Address), // Timelock Address
    Bundler(Address),  // Bundler Address
}

impl Ord for SenderType {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            // Same type, compare addresses
            (SenderType::Bundler(a), SenderType::Bundler(b)) => a.cmp(b),
            (SenderType::Timelock(a), SenderType::Timelock(b)) => a.cmp(b),
            (SenderType::Multisig(a), SenderType::Multisig(b)) => a.cmp(b),
            (SenderType::Signer(a), SenderType::Signer(b)) => a.cmp(b),
            (SenderType::EOA(a), SenderType::EOA(b)) => a.cmp(b),

            // Different types, use enum order
            (SenderType::Bundler(_), _) => Ordering::Less,
            (_, SenderType::Bundler(_)) => Ordering::Greater,
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
