use alloy::sol;

sol! {
    #[sol(rpc)]
    contract GnosisSafe {
        event ApproveHash(bytes32 indexed approvedHash, address indexed owner);
        function execTransactionFromModule(address to, uint256 value, bytes memory data, uint8 operation);
        function getTransactionHash(
            address to,
            uint256 value,
            bytes memory data,
            uint8 operation,
            uint256 safeTxGas,
            uint256 baseGas,
            uint256 gasPrice,
            address gasToken,
            address refundReceiver,
            uint256 _nonce
        ) public view returns (bytes32);
        function enableModule(address module) external;
        function approveHash(bytes32 safeHash) external;
        function getOwners() external view returns(address[] memory owners);
        function getThreshold() external view returns(uint256 threshold);
        function nonce() external view returns(uint256 nonce);
        function execTransaction(
            address to,
            uint256 value,
            bytes calldata data,
            uint8 operation,
            uint256 safeTxGas,
            uint256 baseGas,
            uint256 gasPrice,
            address gasToken,
            address payable refundReceiver,
            bytes memory signatures
        ) external;
        function approvedHashes(address owner, bytes32 safeHash) external view returns(uint256);
    }
}
