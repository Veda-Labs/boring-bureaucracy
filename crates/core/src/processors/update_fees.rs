use crate::{
    actions::{
        admin_action::AdminAction, update_performance_fee_action::UpdatePerformanceFee,
        update_platform_fee_action::UpdatePlatformFee,
    },
    types::config_wrapper::ConfigWrapper,
};
use alloy::primitives::Address;
use eyre::Result;
use serde_json::Value;

// TODO this function could read state to se if it needs to update the fee or not.
pub fn process_fee_updates(
    admin_actions: &mut Vec<Box<dyn AdminAction>>,
    cw: &ConfigWrapper,
    product: &str,
    network_id: u32,
    fee_data: &Value,
) -> Result<()> {
    // Extract fee values from fee_data
    let platform_fee = fee_data.get("platform_fee").and_then(|v| v.as_u64());
    let performance_fee = fee_data.get("performance_fee").and_then(|v| v.as_u64());

    // Ensure at least one fee is present
    if platform_fee.is_none() && performance_fee.is_none() {
        return Err(eyre::eyre!(
            "Either platform_fee or performance_fee must be provided"
        ));
    }

    // Convert to u16 if present
    let platform_fee = platform_fee.map(|v| v as u16);
    let performance_fee = performance_fee.map(|v| v as u16);

    // Get accountant address for the product
    let accountant_addr_str =
        cw.get_product_config_value(product, network_id, "accountant_address")?;
    let accountant_addr = accountant_addr_str.parse::<Address>()?;

    // Add updatePlatformFee action if Some
    if let Some(new_fee) = platform_fee {
        let action = UpdatePlatformFee::new(accountant_addr, new_fee);
        admin_actions.push(Box::new(action));
    }

    // Add updatePerformanceFee action if Some
    if let Some(new_fee) = performance_fee {
        let action = UpdatePerformanceFee::new(accountant_addr, new_fee);
        admin_actions.push(Box::new(action));
    }

    Ok(())
}
