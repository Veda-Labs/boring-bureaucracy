use alloy::sol;

sol! {
    #[sol(rpc)]
    contract AccountantWithRateProviders {
    struct RateProviderData {
        bool isPeggedToBase;
        address rateProvider;
    }

        function setRateProviderData(address asset, bool isPeggedToBase, address rateProvider) external;
        function rateProviderData(address asset) external view returns(RateProviderData memory rpd);
        function base() external view returns(address base);
        function updatePerformanceFee(uint16 fee) external;
        function updatePlatformFee(uint16 fee) external;
    }
}
