use alloy::sol;

sol! {
    #[sol(rpc)]
    contract TellerWithMultiAssetSupport {
        struct Asset {
            bool allowDeposits;
            bool allowWithdraws;
            uint16 sharePremium;
        }

        function updateAssetData(address asset, bool allowDeposits, bool allowWithdraws, uint16 sharePremium) external;
        function assetData(address asset) external view returns(Asset memory asset);
    }
}
