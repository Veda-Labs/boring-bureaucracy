use alloy::sol;

sol! {
    #[sol(rpc)]
    contract Bundler {
        struct Tx {
            address target;
            bytes data;
            uint256 value;
        }
        function bundleTxs(Tx[] calldata txs) external;
    }
}
