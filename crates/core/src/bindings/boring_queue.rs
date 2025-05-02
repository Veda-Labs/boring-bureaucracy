use alloy::sol;

sol! {
    #[sol(rpc)]
    contract BoringOnChainQueue {
    struct WithdrawAsset {
        bool allowWithdraws;
        uint24 secondsToMaturity;
        uint24 minimumSecondsToDeadline;
        uint16 minDiscount;
        uint16 maxDiscount;
        uint96 minimumShares;
        uint256 withdrawCapacity;
    }
    function updateWithdrawAsset(
        address assetOut,
        uint24 secondsToMaturity,
        uint24 minimumSecondsToDeadline,
        uint16 minDiscount,
        uint16 maxDiscount,
        uint96 minimumShares
    ) external;
    function stopWithdrawsInAsset(address assetOut) external;
    function withdrawAssets(address asset) external view returns(bool allowWithdraws, uint24 secondsToMaturity, uint24 minimumSecondsToDeadline, uint16 minDiscount, uint16 maxDiscount, uint96 minimumShares);
    }
}
