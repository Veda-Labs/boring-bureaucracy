use alloy::contract::CallBuilder;
use alloy::eips::BlockId;
use alloy::primitives::{Address, Bytes, U256};
use alloy::providers::Provider;
use alloy::rpc::types::BlockNumberOrTag;
use alloy::transports::BoxTransport;
use eyre::Result;
use log::debug;
use log::info;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::{Mutex, oneshot};

// Request type containing all info needed for the RPC call
#[derive(Hash, Eq, PartialEq, Clone)]
struct ViewRequest {
    target: Address,
    calldata: Option<Bytes>,
    read_slot: Option<U256>,
}

// Response type containing the result or error
type ViewResponse = Result<Bytes>;

pub struct ViewRequestManager {
    // Cache of completed requests
    cache: Arc<Mutex<HashMap<ViewRequest, ViewResponse>>>,
    // Channel to send requests to worker pool
    request_tx: Sender<(ViewRequest, oneshot::Sender<ViewResponse>)>,
}

// Create a type alias for the view request manager
pub type ViewRequestManagerRef = Arc<ViewRequestManager>;

impl ViewRequestManager {
    pub fn new<P>(num_workers: usize, provider: P, latest_block_number: u64) -> Self
    where
        P: Provider + Clone + Send + Sync + 'static,
    {
        debug!("Creating ViewRequestManager");
        let (tx, rx) = mpsc::channel(100);
        let cache = Arc::new(Mutex::new(HashMap::new()));

        // Spawn worker pool
        debug!("Spawning {} worker(s)", num_workers);
        Self::spawn_workers(
            provider.clone(),
            latest_block_number,
            num_workers,
            rx,
            Arc::clone(&cache),
        );

        Self {
            cache,
            request_tx: tx,
        }
    }

    async fn _request(&self, request: ViewRequest) -> Result<Bytes> {
        // Check cache first
        if let Some(response) = self.cache.lock().await.get(&request) {
            return match response {
                Ok(bytes) => Ok(bytes.clone()),
                Err(_) => Err(eyre::eyre!("Cached error")),
            };
        }

        // Create oneshot channel for response
        let (response_tx, response_rx) = oneshot::channel();

        // Send request to worker pool
        self.request_tx.send((request.clone(), response_tx)).await?;

        // Wait for response
        let response = response_rx.await?;

        // Cache the response
        match &response {
            Ok(bytes) => self.cache.lock().await.insert(request, Ok(bytes.clone())),
            Err(_) => self
                .cache
                .lock()
                .await
                .insert(request, Err(eyre::eyre!("Cached error"))),
        };

        response
    }

    // TODO this is where I Was at
    pub async fn request_storage_read(&self, target: Address, slot: U256) -> Result<U256> {
        let request = ViewRequest {
            target,
            calldata: None,
            read_slot: Some(slot),
        };

        let bytes_response = self._request(request).await?;
        Ok(U256::from_be_bytes::<32>(
            bytes_response.as_ref().try_into()?,
        ))
    }
    pub async fn request(&self, target: Address, calldata: Bytes) -> Result<Bytes> {
        let request = ViewRequest {
            target,
            calldata: Some(calldata),
            read_slot: None,
        };

        self._request(request).await
    }

    fn spawn_workers<P>(
        provider: P,
        latest_block_number: u64,
        num_workers: usize,
        rx: Receiver<(ViewRequest, oneshot::Sender<ViewResponse>)>,
        cache: Arc<Mutex<HashMap<ViewRequest, ViewResponse>>>,
    ) where
        P: Provider + Clone + Send + Sync + 'static,
    {
        let rx = Arc::new(Mutex::new(rx));
        for _ in 0..num_workers {
            let rx = Arc::clone(&rx);
            let cache = Arc::clone(&cache);
            let provider = provider.clone();
            tokio::spawn(async move {
                loop {
                    let (request, response_tx) = {
                        let mut rx_guard = rx.lock().await;
                        match rx_guard.recv().await {
                            Some(msg) => msg,
                            None => break, // Channel closed, exit worker
                        }
                    };

                    // Make the RPC call
                    let response =
                        Self::make_rpc_call(provider.clone(), &request, latest_block_number).await;

                    // Cache the result
                    match &response {
                        Ok(bytes) => cache.lock().await.insert(request, Ok(bytes.clone())),
                        Err(_) => cache
                            .lock()
                            .await
                            .insert(request, Err(eyre::eyre!("Cached error"))),
                    };

                    // Send response back to requester
                    let _ = response_tx.send(match &response {
                        Ok(bytes) => Ok(bytes.clone()),
                        Err(e) => {
                            info!("RPC request failed");
                            Err(eyre::eyre!("{}", e))
                        }
                    });
                }
            });
        }
    }

    const MAX_RETRIES: u32 = 10;
    const RETRY_DELAY: Duration = Duration::from_secs(1);

    // TODO add logic that checks the length of the request calldata, if it is exactly 32 bytes,
    // then that means we need to make a provider.get_storage_at call instead off a normal call.
    // This should be seamless as all calldata will have function selectors in them
    // TODO OR I just add another arg to the request function so requesters have to specifically say their request is a
    // raw storage read request.
    async fn make_rpc_call<P>(
        provider: P,
        request: &ViewRequest,
        latest_block_number: u64,
    ) -> Result<Bytes>
    where
        P: Provider + Clone + Send + Sync + 'static,
    {
        if request.calldata.is_none() {
            // Making raw storage read
            let mut attempts = 0;
            let slot = request.read_slot.unwrap();

            while attempts < Self::MAX_RETRIES {
                match provider.get_storage_at(request.target, slot).await {
                    Ok(result) => return Ok(Bytes::from(result.to_be_bytes::<32>())),
                    Err(e) => {
                        // If it's not a rate limit error, return the error immediately
                        if !e.to_string().contains("429") && !e.to_string().contains("quota") {
                            return Err(e.into());
                        }

                        info!("Rate limit error: {}", e);
                        attempts += 1;
                        tokio::time::sleep(Self::RETRY_DELAY).await;
                    }
                }
            }

            Err(eyre::eyre!(
                "Failed to make storage read call after {} attempts",
                Self::MAX_RETRIES
            ))
        } else {
            let calldata = request.calldata.clone().unwrap();
            let builder = CallBuilder::<BoxTransport, P, ()>::new_raw(provider, calldata);
            let builder = builder.to(request.target);
            let builder = builder.block(BlockId::Number(BlockNumberOrTag::Number(
                latest_block_number,
            )));
            let mut attempts = 0;

            while attempts < Self::MAX_RETRIES {
                match builder.call().await {
                    Ok(result) => return Ok(result),
                    Err(e) => {
                        // If it's not a rate limit error, return the error immediately
                        if !e.to_string().contains("429") && !e.to_string().contains("quota") {
                            return Err(e.into());
                        }

                        info!("Rate limit error: {}", e);
                        attempts += 1;
                        tokio::time::sleep(Self::RETRY_DELAY).await;
                    }
                }
            }

            Err(eyre::eyre!(
                "Failed to make RPC call after {} attempts",
                Self::MAX_RETRIES
            ))
        }
    }
}

// TODO
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use alloy::primitives::bytes;
//     use alloy::providers::ProviderBuilder;
//     use alloy::sol_types::{sol_data, SolType};
//     use tokio;

//     #[tokio::test]
//     async fn test_requests_greater_than_workers() {
//         // Create provider pointing to mainnet
//         let provider = ProviderBuilder::new()
//             .connect("https://1rpc.io/eth")
//             .await
//             .unwrap();

//         // Create request manager with 3 workers
//         let manager = Arc::new(ViewRequestManager::new(3, provider));

//         // Symbol function selector
//         let symbol_calldata = bytes!("95d89b41");

//         // Create requests for each token
//         let tokens = [
//             ("USDC", "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
//             ("WETH", "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"),
//             ("WBTC", "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599"),
//             ("USDT", "0xdAC17F958D2ee523a2206206994597C13D831ec7"),
//             ("TUSD", "0x0000000000085d4780B73119b644AE5ecd22b376"),
//         ];

//         // Create custom type to decode the result.
//         type SymbolReturnType = sol_data::String;

//         let mut handles = vec![];

//         for (expected_symbol, addr) in tokens {
//             let manager = Arc::clone(&manager);
//             let target = addr.parse().unwrap();
//             let calldata = symbol_calldata.clone();
//             handles.push(async move {
//                 let result = manager.request(target, calldata).await.unwrap().to_vec();
//                 // Decode the result into a SymbolReturnType
//                 let symbol = SymbolReturnType::abi_decode(&result, true).unwrap();
//                 assert_eq!(&symbol, expected_symbol);
//             });
//         }

//         futures::future::join_all(handles).await;
//     }

//     #[tokio::test]
//     async fn test_requests_less_than_workers() {
//         // Create provider pointing to mainnet
//         let provider = ProviderBuilder::new()
//             .with_recommended_fillers()
//             .on_builtin("https://1rpc.io/eth")
//             .await
//             .unwrap();

//         // Create request manager with 3 workers
//         let manager = Arc::new(ViewRequestManager::new(8, provider));

//         // Symbol function selector
//         let symbol_calldata = bytes!("95d89b41");

//         // Create requests for each token
//         let tokens = [
//             ("USDC", "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
//             ("WETH", "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"),
//             ("WBTC", "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599"),
//             ("USDT", "0xdAC17F958D2ee523a2206206994597C13D831ec7"),
//             ("TUSD", "0x0000000000085d4780B73119b644AE5ecd22b376"),
//         ];

//         // Create custom type to decode the result.
//         type SymbolReturnType = sol_data::String;

//         let mut handles = vec![];

//         for (expected_symbol, addr) in tokens {
//             let manager = Arc::clone(&manager);
//             let target = addr.parse().unwrap();
//             let calldata = symbol_calldata.clone();
//             handles.push(async move {
//                 let result = manager.request(target, calldata).await.unwrap().to_vec();
//                 // Decode the result into a SymbolReturnType
//                 let symbol = SymbolReturnType::abi_decode(&result, true).unwrap();
//                 assert_eq!(&symbol, expected_symbol);
//             });
//         }

//         futures::future::join_all(handles).await;
//     }

//     #[tokio::test]
//     async fn test_cache_reuse() {
//         let provider = ProviderBuilder::new()
//             .with_recommended_fillers()
//             .on_builtin("https://1rpc.io/eth")
//             .await
//             .unwrap();

//         let manager = Arc::new(ViewRequestManager::new(1, provider));
//         let symbol_calldata = bytes!("95d89b41");
//         let usdc = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
//             .parse()
//             .unwrap();

//         // First request - should hit RPC
//         let start = std::time::Instant::now();
//         let result1 = manager
//             .request(usdc, symbol_calldata.clone())
//             .await
//             .unwrap();
//         let first_duration = start.elapsed();

//         // Second request - should hit cache
//         let start = std::time::Instant::now();
//         let result2 = manager.request(usdc, symbol_calldata).await.unwrap();
//         let second_duration = start.elapsed();

//         // Verify results match
//         type SymbolType = sol_data::String;
//         let symbol1 = SymbolType::abi_decode(&result1, true).unwrap();
//         let symbol2 = SymbolType::abi_decode(&result2, true).unwrap();
//         assert_eq!(symbol1, symbol2);
//         assert_eq!(symbol1, "USDC");

//         // Second request should be much faster (cached)
//         assert!(second_duration < first_duration);
//         assert!(
//             second_duration.as_micros() < 100,
//             "Cache lookup took too long: {:?}",
//             second_duration
//         );
//     }

//     #[tokio::test]
//     async fn test_concurrent_same_request() {
//         let provider = ProviderBuilder::new()
//             .with_recommended_fillers()
//             .on_builtin("https://1rpc.io/eth")
//             .await
//             .unwrap();

//         let manager = Arc::new(ViewRequestManager::new(2, provider));
//         let symbol_calldata = bytes!("95d89b41");
//         let usdc = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
//             .parse()
//             .unwrap();

//         // Make two identical requests concurrently
//         let (result1, result2) = tokio::join!(
//             manager.request(usdc, symbol_calldata.clone()),
//             manager.request(usdc, symbol_calldata.clone())
//         );

//         // Both should succeed and return the same result
//         type SymbolType = sol_data::String;
//         let symbol1 = SymbolType::abi_decode(&result1.unwrap(), true).unwrap();
//         let symbol2 = SymbolType::abi_decode(&result2.unwrap(), true).unwrap();
//         assert_eq!(symbol1, symbol2);
//         assert_eq!(symbol1, "USDC");
//     }

//     // This test works, but its commented out to make less rpc calls.
//     // #[tokio::test]
//     // async fn test_retry_logic() {
//     //     let provider = ProviderBuilder::new()
//     //         .with_recommended_fillers()
//     //         .on_builtin("https://1rpc.io/eth")
//     //         .await
//     //         .unwrap();

//     //     // Create lots of workers and requests to try to trigger rate limiting
//     //     let manager = Arc::new(ViewRequestManager::new(100, provider));
//     //     let symbol_calldata = bytes!("95d89b41");

//     //     let mut handles = vec![];

//     //     // Make 100 concurrent requests for USDC symbol
//     //     for _ in 0..100 {
//     //         let manager = Arc::clone(&manager);
//     //         let calldata = symbol_calldata.clone();
//     //         handles.push(async move {
//     //             let usdc = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"
//     //                 .parse()
//     //                 .unwrap();
//     //             let result = manager.request(usdc, calldata).await.unwrap();
//     //             type SymbolType = sol_data::String;
//     //             let symbol = SymbolType::abi_decode(&result, true).unwrap();
//     //             assert_eq!(symbol, "USDC");
//     //         });
//     //     }

//     //     futures::future::join_all(handles).await;
//     // }
// }
