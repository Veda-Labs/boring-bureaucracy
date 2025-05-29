use alloy::sol;

sol! {
    #[sol(rpc)]
    contract Auth {
        function authority() external view returns(address authority);
        function owner() external view returns(address owner);
        function transferOwnership(address newOwner) external;
        function setAuthority(address newOwner) external;
    }
}
