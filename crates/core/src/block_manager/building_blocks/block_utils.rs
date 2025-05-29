use crate::actions::{
    action::Action, sender_type::SenderType, set_role_capability_action::SetRoleCapabilityAction,
    set_user_role_action::SetUserRoleAction,
};
use crate::bindings::roles_authority::RolesAuthority;
use crate::utils::view_request_manager::ViewRequestManager;
use alloy::primitives::{Address, Bytes, FixedBytes, aliases::B32, keccak256};

use alloy::sol_types::SolCall;
use eyre::Result;

pub async fn grant_roles_capabilities(
    actions: &mut Vec<Box<dyn Action>>,
    roles_authority: Address,
    capabilities: Vec<(u8, Address, &str)>,
    vrm: &ViewRequestManager,
    priority: u32,
    sender: SenderType,
) -> Result<()> {
    // Create futures for checking capabilities
    let mut capability_futures = Vec::new();
    let mut capability_info = Vec::new();

    for (role, target, function_signature) in capabilities {
        let function_selector =
            FixedBytes::<4>::from_slice(&keccak256(function_signature.as_bytes())[..4].to_vec());

        // Create a future for checking the capability
        let future = does_role_have_capability(
            roles_authority,
            role,
            target,
            B32::from(function_selector),
            vrm,
        );

        capability_futures.push(future);
        capability_info.push((role, target, function_signature));
    }

    // Execute all futures concurrently
    let results = futures::future::join_all(capability_futures).await;

    // Process results and add necessary actions
    for (i, has_capability_result) in results.into_iter().enumerate() {
        let (role, target, function_signature) = &capability_info[i];

        // Only add action if the role doesn't already have the capability
        match has_capability_result {
            Ok(has_capability) if !has_capability => {
                let action = SetRoleCapabilityAction::new(
                    roles_authority,
                    *role,
                    *target,
                    function_signature.to_string(),
                    true,
                    priority,
                    sender,
                );
                actions.push(Box::new(action));
            }
            Ok(_) => {
                // Role already has the capability, no action needed
            }
            Err(e) => {
                // Handle error
                return Err(e);
            }
        }
    }

    Ok(())
}

pub async fn grant_users_roles(
    actions: &mut Vec<Box<dyn Action>>,
    roles_authority: Address,
    user_role: Vec<(Address, u8)>,
    vrm: &ViewRequestManager,
    priority: u32,
    sender: SenderType,
) -> Result<()> {
    // Create futures for checking if users have roles
    let mut role_futures = Vec::new();
    let mut role_info = Vec::new();

    for (user, role) in user_role {
        // Create a future for checking if the user has the role
        let future = does_user_have_role(roles_authority, user, role, vrm);

        role_futures.push(future);
        role_info.push((user, role));
    }

    // Execute all futures concurrently
    let results = futures::future::join_all(role_futures).await;

    // Process results and add necessary actions
    for (i, has_role_result) in results.into_iter().enumerate() {
        let (user, role) = role_info[i];

        // Only add action if the user doesn't already have the role
        match has_role_result {
            Ok(has_role) if !has_role => {
                let action =
                    SetUserRoleAction::new(roles_authority, user, role, true, priority, sender);
                actions.push(Box::new(action));
            }
            Ok(_) => {
                // User already has the role, no action needed
            }
            Err(e) => {
                // Handle error
                return Err(e);
            }
        }
    }

    Ok(())
}

async fn does_role_have_capability(
    roles_authority: Address,
    role: u8,
    target: Address,
    function_selector: B32,
    vrm: &ViewRequestManager,
) -> Result<bool> {
    if vrm.request_code(roles_authority).await?.len() > 0 {
        let calldata = Bytes::from(
            RolesAuthority::doesRoleHaveCapabilityCall::new((role, target, function_selector))
                .abi_encode(),
        );
        let result = vrm.request(roles_authority, calldata).await?;
        Ok(RolesAuthority::doesRoleHaveCapabilityCall::abi_decode_returns(&result, true)?._0)
    } else {
        Ok(false)
    }
}

async fn does_user_have_role(
    roles_authority: Address,
    user: Address,
    role: u8,
    vrm: &ViewRequestManager,
) -> Result<bool> {
    if vrm.request_code(roles_authority).await?.len() > 0 {
        let calldata =
            Bytes::from(RolesAuthority::doesUserHaveRoleCall::new((user, role)).abi_encode());
        let result = vrm.request(roles_authority, calldata).await?;
        Ok(RolesAuthority::doesUserHaveRoleCall::abi_decode_returns(&result, true)?._0)
    } else {
        Ok(false)
    }
}
