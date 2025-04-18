use alloy::sol;

sol! {
    #[sol(rpc)]
    contract ManagerWithMerkleVerification {
        function setManageRoot(address strategist, bytes32 root) external;
    }
}
