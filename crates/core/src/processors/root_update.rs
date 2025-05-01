use crate::{
    actions::{admin_action::AdminAction, set_merkle_root_action::SetMerkleRoot},
    types::config_wrapper::ConfigWrapper,
};
use alloy::primitives::{Address, FixedBytes};
use eyre::Result;

// Process merkle root update action
pub fn process_merkle_root_update(
    admin_actions: &mut Vec<Box<dyn AdminAction>>,
    cw: &ConfigWrapper,
    product: &str,
    network_id: u32,
    root_str: &str,
) -> Result<()> {
    // Remove "0x" prefix if present and convert to bytes
    let root_str = root_str.trim_start_matches("0x");
    let root_bytes = hex::decode(root_str)?;
    let root = FixedBytes::<32>::from_slice(&root_bytes);

    // Get manager address for the product
    let manager_addr_str = cw.get_product_config_value(product, network_id, "manager_address")?;
    let manager_addr = manager_addr_str.parse::<Address>()?;

    // Get strategists
    let strategists = cw.get_product_strategists(product, network_id)?;

    // Add a SetMerkleRootAction for each strategist
    for strategist_str in strategists {
        let strategist_addr = strategist_str.parse::<Address>()?;

        let action = SetMerkleRoot::new(manager_addr, strategist_addr, root);

        admin_actions.push(Box::new(action));
    }

    Ok(())
}
