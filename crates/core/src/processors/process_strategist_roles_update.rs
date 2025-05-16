use crate::{
    actions::{admin_action::AdminAction, set_user_role_action::SetUserRoleAction},
    types::config_wrapper::ConfigWrapper,
};
use alloy::primitives::Address;
use eyre::{eyre, Result};
use serde_json::Value;

// Enum to define the mode of operation
enum StrategistUpdateMode {
    AddRoles,
    RevokeRoles,
}

pub fn process_strategist_roles_update(
    admin_actions: &mut Vec<Box<dyn AdminAction>>,
    cw: &ConfigWrapper,
    product: &str,
    network_id: u32,
    strategist_update_data: &Value, // Renamed for clarity
) -> Result<()> {
    // Extract strategist address
    let strategist_address_str = strategist_update_data
        .get("strategist_address")
        .and_then(|v| v.as_str())
        .ok_or_else(|| eyre!("'strategist_address' must be a string and is required"))?;
    let strategist_addr = strategist_address_str.parse::<Address>()?;

    // Extract operation mode ("add_roles" or "revoke_roles")
    let operation_str = strategist_update_data
        .get("operation")
        .and_then(|v| v.as_str())
        .ok_or_else(|| eyre!("'operation' field (e.g., 'add_roles' or 'revoke_roles') is required"))?;

    let mode = match operation_str {
        "add_roles" => StrategistUpdateMode::AddRoles,
        "revoke_roles" => StrategistUpdateMode::RevokeRoles,
        _ => return Err(eyre!("Invalid 'operation': must be 'add_roles' or 'revoke_roles'")),
    };

    // Extract roles to add/revoke
    let roles_val = strategist_update_data
        .get("roles") // Changed from "roles_to_revoke" to be generic
        .ok_or_else(|| eyre!("'roles' field (array of role IDs) is required"))?;
    
    let roles_array = roles_val
        .as_array()
        .ok_or_else(|| eyre!("'roles' must be an array of role IDs"))?;

    let mut role_ids: Vec<u8> = Vec::new();
    for role_val in roles_array {
        let role_id = role_val
            .as_u64()
            .ok_or_else(|| eyre!("Each role ID in 'roles' must be a number"))?
            as u8;
        role_ids.push(role_id);
    }

    if role_ids.is_empty() {
        return Err(eyre!("'roles' array cannot be empty"));
    }

    // Get roles_authority address for the product
    let roles_authority_addr_str =
        cw.get_product_config_value(product, network_id, "roles_authority_address")?;
    let roles_authority_addr = roles_authority_addr_str.parse::<Address>()?;

    let enabled = match mode {
        StrategistUpdateMode::AddRoles => true,
        StrategistUpdateMode::RevokeRoles => false,
    };

    // Create SetUserRoleAction for each role
    for role_id in role_ids {
        let action = SetUserRoleAction::new(
            roles_authority_addr,
            strategist_addr,
            role_id,
            enabled,
        );
        admin_actions.push(Box::new(action));
    }

    Ok(())
} 