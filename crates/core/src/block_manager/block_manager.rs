// Tracks if stuff is deployed
// Looks for configuration errors/warnings
// Creates building blocks from jsons
// Shared contracts? Like building blocks can
// save addresses in the block manager

use super::building_blocks::building_block::{Actionable, BuildingBlock};
use crate::block_manager::shared_cache::{SharedCache, SharedCacheRef};
use crate::utils::view_request_manager::{ViewRequestManager, ViewRequestManagerRef};
use alloy::providers::Provider;
use alloy::providers::ProviderBuilder;

use eyre::Result;
use serde_json::Value;
use std::time::Duration;

pub struct BlockManager {
    pub blocks: Vec<Box<dyn Actionable>>,
    pub cache: SharedCacheRef,
    vrm: ViewRequestManagerRef,
}

impl BlockManager {
    const WORKER_COUNT: usize = 8;

    pub async fn new(rpc_url: String) -> Result<Self> {
        let provider = ProviderBuilder::new().on_builtin(&rpc_url).await?;
        let latest_block_number = provider.get_block_number().await?;
        let vrm = ViewRequestManager::new(Self::WORKER_COUNT, provider, latest_block_number).into();
        Ok(Self {
            blocks: vec![],
            cache: SharedCache::new(Duration::from_secs(10)).into(),
            vrm: vrm,
        })
    }

    pub async fn from_value(rpc_url: String, value: Value) -> Result<Self> {
        let mut s = Self::new(rpc_url).await?;
        s.create_blocks_from_json_value(value)?;
        Ok(s)
    }

    pub async fn from_str(rpc_url: String, json_str: &str) -> Result<Self> {
        let mut s = Self::new(rpc_url).await?;
        s.create_blocks_from_json_str(json_str)?;
        Ok(s)
    }

    pub fn create_blocks_from_json_value(&mut self, value: Value) -> Result<()> {
        let building_blocks: Vec<BuildingBlock> = serde_json::from_value(value)?;
        self.blocks = building_blocks
            .into_iter()
            .map(|b| b.into_trait_object())
            .collect();
        Ok(())
    }

    pub fn create_blocks_from_json_str(&mut self, json_str: &str) -> Result<()> {
        let building_blocks: Vec<BuildingBlock> = serde_json::from_str(json_str)?;
        self.blocks = building_blocks
            .into_iter()
            .map(|b| b.into_trait_object())
            .collect();
        Ok(())
    }

    pub async fn propogate_shared_data(&mut self) -> Result<()> {
        let mut handles = Vec::new();
        let blocks = std::mem::take(&mut self.blocks);

        for mut block in blocks {
            let cache = self.cache.clone();
            let vrm = self.vrm.clone();
            let handle = tokio::spawn(async move {
                block.resolve_and_contribute(&cache, &vrm).await?;
                Ok::<Box<dyn Actionable>, eyre::Error>(block) // Explicitly specify the type
            });
            handles.push(handle);
        }

        // Wait for all blocks to complete
        let mut processed_blocks = Vec::new();
        for handle in handles {
            let block = handle.await??; // First ? for JoinError, second ? for our Result
            processed_blocks.push(block);
        }

        // Restore blocks to self
        self.blocks = processed_blocks;

        Ok(())
    }
}
