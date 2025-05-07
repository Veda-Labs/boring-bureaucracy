use alloy::sol;

sol! {
    #[sol(rpc)]
    contract Deployer {
        struct Tx {
            address target;
            bytes data;
            uint256 value;
        }

        function deployContract(
            string calldata name,
            bytes memory creationCode,
            bytes calldata constructorArgs,
            uint256 value
        ) external;
        function bundleTxs(Tx[] calldata txs) external;
}
}
