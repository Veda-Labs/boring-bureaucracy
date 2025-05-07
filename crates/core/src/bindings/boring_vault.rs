use alloy::sol;

sol! {
    #[sol(rpc)]
    contract BoringVault {
        function authority() external view returns(address authority);
        function name() external view returns(string);
        function symbol() external view returns(string);
        function decimals() external view returns(uint8);
        function hook() external view returns(address hook);
    }
}
