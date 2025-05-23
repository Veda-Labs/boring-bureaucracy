use crate::{
    actions::{
        admin_action::AdminAction,
        set_user_role_action::SetUserRoleAction,
        set_role_capability_action::SetRoleCapabilityAction,
        set_public_capability_action::SetPublicCapabilityAction,
    },
    types::config_wrapper::ConfigWrapper,
};
use alloy::primitives::Address;
use eyre::{eyre, Result};
use serde_json::Value;

pub fn process_roles_updates(
    admin_actions: &mut Vec<Box<dyn AdminAction>>,
    cw: &ConfigWrapper,
    product: &str,
    network_id: u32,
    roles_data: &Value,
) -> Result<()> {
    // Get roles_authority address for the product
    let roles_authority_addr_str =
        cw.get_product_config_value(product, network_id, "roles_authority_address")?;
    let roles_authority_addr = roles_authority_addr_str.parse::<Address>()?;

    // Process each role update in the array
    let roles_array = roles_data
        .as_array()
        .ok_or_else(|| eyre!("new_roles must be an array"))?;

    for role_update in roles_array {
        let action_type = role_update["action_type"]
            .as_str()
            .ok_or_else(|| eyre!("action_type must be a string"))?;

        match action_type {
            "setUserRole" => {
                let user_str = role_update["user"]
                    .as_str()
                    .ok_or_else(|| eyre!("user must be a string"))?;
                let user_addr = user_str.parse::<Address>()?;

                let role_id = role_update["role_id"]
                    .as_u64()
                    .ok_or_else(|| eyre!("role_id must be a number"))?;
                let role_id: u8 = role_id
                    .try_into()
                    .map_err(|_| eyre!("role_id must be between 0 and 255"))?;

                let enabled = role_update["enabled"]
                    .as_bool()
                    .ok_or_else(|| eyre!("enabled must be a boolean"))?;

                let action = SetUserRoleAction::new(
                    roles_authority_addr,
                    user_addr,
                    role_id,
                    enabled,
                );
                admin_actions.push(Box::new(action));
            }
            "setRoleCapability" => {
                let role_id = role_update["role_id"]
                    .as_u64()
                    .ok_or_else(|| eyre!("role_id must be a number"))?;
                let role_id: u8 = role_id
                    .try_into()
                    .map_err(|_| eyre!("role_id must be between 0 and 255"))?;

                let target_str = role_update["target_contract"]
                    .as_str()
                    .ok_or_else(|| eyre!("target_contract must be a string"))?;
                let target_addr = target_str.parse::<Address>()?;

                let function_signature = role_update["function_signature"]
                    .as_str()
                    .ok_or_else(|| eyre!("function_signature must be a string"))?
                    .to_string();

                let enabled = role_update["enabled"]
                    .as_bool()
                    .ok_or_else(|| eyre!("enabled must be a boolean"))?;

                let action = SetRoleCapabilityAction::new(
                    roles_authority_addr,
                    role_id,
                    target_addr,
                    function_signature,
                    enabled,
                );
                admin_actions.push(Box::new(action));
            }
            "setPublicCapability" => {
                let target_str = role_update["target_contract"]
                    .as_str()
                    .ok_or_else(|| eyre!("target_contract must be a string"))?;
                let target_addr = target_str.parse::<Address>()?;

                let function_signature = role_update["function_signature"]
                    .as_str()
                    .ok_or_else(|| eyre!("function_signature must be a string"))?
                    .to_string();

                let enabled = role_update["enabled"]
                    .as_bool()
                    .ok_or_else(|| eyre!("enabled must be a boolean"))?;

                let action = SetPublicCapabilityAction::new(
                    roles_authority_addr,
                    target_addr,
                    function_signature,
                    enabled,
                );
                admin_actions.push(Box::new(action));
            }
            _ => {
                return Err(eyre!(
                    "Unknown action_type: {}. Must be one of: setUserRole, setRoleCapability, setPublicCapability",
                    action_type
                ));
            }
        }
    }

    Ok(())
}