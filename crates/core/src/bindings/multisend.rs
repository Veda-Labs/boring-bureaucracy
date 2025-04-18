use alloy::sol;

sol! {
    #[sol(rpc)]
    contract MutliSendCallOnly {
        function multiSend(bytes memory transactions) external;
    }
}
