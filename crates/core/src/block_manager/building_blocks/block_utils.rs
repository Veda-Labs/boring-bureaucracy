use crate::bindings::roles_authority::RolesAuthority;
use crate::utils::view_request_manager::ViewRequestManager;
use alloy::{
    primitives::{Address, Bytes, aliases::B32},
    sol_types::SolCall,
};
use eyre::Result;

pub async fn does_role_have_capability(
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

pub async fn does_user_have_role(
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
