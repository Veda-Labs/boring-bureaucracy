// Tracks if stuff is deployed
// Looks for configuration errors/warnings
// Creates building blocks from jsons
// Shared contracts? Like building blocks can
// save addresses in the block manager

use super::building_blocks::{building_block::BuildingBlock, building_blocks::BuildingBlocks};
use super::shared_cache::CacheValue;
use crate::actions::action::Action;
use crate::actions::multisig_meta_action::MultisigMetaAction;
use crate::actions::sender_type::SenderType;
use crate::block_manager::shared_cache::{SharedCache, SharedCacheRef};
use crate::utils::view_request_manager::{ViewRequestManager, ViewRequestManagerRef};
use alloy::primitives::Address;
use alloy::providers::Provider;
use alloy::providers::ProviderBuilder;

use eyre::{Result, eyre};
use serde_json::Value;
use std::time::Duration;

pub struct BlockManager {
    pub blocks: Vec<Box<dyn BuildingBlock>>,
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
        let building_blocks: Vec<BuildingBlocks> = serde_json::from_value(value)?;
        self.blocks = building_blocks
            .into_iter()
            .map(|b| b.into_trait_object())
            .collect();
        Ok(())
    }

    pub fn create_blocks_from_json_str(&mut self, json_str: &str) -> Result<()> {
        let building_blocks: Vec<BuildingBlocks> = serde_json::from_str(json_str)?;
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
                block.resolve_state(&cache, &vrm).await?;
                Ok::<Box<dyn BuildingBlock>, eyre::Error>(block) // Explicitly specify the type
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

    pub async fn assemble_and_aggregate(&mut self) -> Result<Vec<Box<dyn Action>>> {
        let mut actions = Vec::new();

        // Assemble actions from all blocks
        for block in &self.blocks {
            let block_actions = block.assemble(&self.vrm).await?;
            actions.extend(block_actions);
        }

        // Sort by priority first, then by SenderType
        actions.sort_by(|a, b| {
            a.priority()
                .cmp(&b.priority())
                .then_with(|| a.sender().cmp(&b.sender()))
        });

        if actions.is_empty() {
            return Err(eyre!("BlockManager: actions is empty"));
        } else {
            let executor = match self.cache.get_immediate("executor").await? {
                CacheValue::Address(addr) => addr,
                _ => {
                    return Err(eyre!(
                        "BlockManager: executor is not an address in the cache"
                    ));
                }
            };
            let mut meta_actions = Vec::new();
            let mut current_chunk = Vec::new();
            let mut current_sender = actions[0].sender();
            for action in actions.into_iter() {
                match action.sender() {
                    SenderType::EOA(addr) => {
                        if addr != executor {
                            return Err(eyre!("BlockManager: Wrong EOA"));
                        }
                        meta_actions.push(action);
                        current_sender = SenderType::EOA(addr);
                    }
                    SenderType::Signer(multisig) => {
                        // Convert into Approve Hash or Exec Transaction action, and push onto meta_actions
                        let meta_action =
                            MultisigMetaAction::new(multisig, executor, action, None, &self.vrm)
                                .await?;
                        meta_actions.push(Box::new(meta_action));
                        current_sender = SenderType::Signer(multisig);
                    }
                    SenderType::Multisig(multisig) => {
                        // Need to append actions to current_chunk, until it changes, once it changes, we take the current chunk, and make a new meta_action, emptying out the current chunk.
                        if current_sender != SenderType::Multisig(multisig) {
                            // TODO need to create the multisend meta action
                            // Chunk transition.
                            if !current_chunk.is_empty() {
                                // Batch current chunk into a Multisend meta action
                                // reset current chunk to empty
                                // push current action into current chunk
                            } // else nothing to do
                        } else {
                            // Add action to current chunk.
                            current_chunk.push(action);
                        }
                    }
                    SenderType::Timelock(timelock) => {
                        if current_sender != SenderType::Timelock(timelock) {
                            // Chunk transition
                            if !current_chunk.is_empty() {
                                // Batch current chunk into a Timelock meta action
                                // reset current chunk to empty
                            }

                            // Add action to current chunk.
                            current_chunk.push(action);
                        }
                    }
                }
            }
        }

        Ok(actions)
    }
}
