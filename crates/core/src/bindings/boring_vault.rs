use alloy::sol;

sol! {
    #[sol(rpc)]
    contract BoringVault {
        constructor(address _owner, string memory _name, string memory _symbol, uint8 _decimals);
        function authority() external view returns(address authority);
        function name() external view returns(string);
        function symbol() external view returns(string);
        function decimals() external view returns(uint8);
        function hook() external view returns(address hook);
    }
}
