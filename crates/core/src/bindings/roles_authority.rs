use alloy::sol;

sol! {
    #[sol(rpc)]
    contract RolesAuthority {
        function doesUserHaveRole(address user, uint8 role) public view virtual returns (bool);
        function doesRoleHaveCapability(
            uint8 role,
            address target,
            bytes4 functionSig
        ) public view virtual returns (bool);
        function setPublicCapability(
            address target,
            bytes4 functionSig,
            bool enabled
        ) external;
        function setRoleCapability(
            uint8 role,
            address target,
            bytes4 functionSig,
            bool enabled
        ) external;
        function setUserRole(
            address user,
            uint8 role,
            bool enabled
        ) external;
    }
}
