use alloy::sol;

sol! {
    #[sol(rpc)]
    contract Timelock {
        function scheduleBatch(address[] memory targets, uint256[] memory values, bytes[] memory payloads, bytes32 predecessor, bytes32 salt, uint256 delay) external;
        function executeBatch(address[] memory targets, uint256[] memory values, bytes[] memory payloads, bytes32 predecessor, bytes32 salt) external;
        function getMinDelay() external view returns(uint256 delay);
    }
}
