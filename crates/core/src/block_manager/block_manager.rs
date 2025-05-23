// Tracks if stuff is deployed
// Looks for configuration errors/warnings
// Creates building blocks from jsons
// Shared contracts? Like building blocks can
// save addresses in the block manager

use super::building_blocks::{building_block::BuildingBlock, building_blocks::BuildingBlocks};
use crate::actions::action::Action;
use crate::actions::sender_type::SenderType;
use crate::actions::{
    multisend_meta_action::MultisendMetaAction, multisig_meta_action::MultisigMetaAction,
    timelock_meta_action::TimelockMetaAction,
};
use crate::block_manager::shared_cache::{SharedCache, SharedCacheRef};
use crate::utils::view_request_manager::{ViewRequestManager, ViewRequestManagerRef};
use alloy::providers::Provider;
use alloy::providers::ProviderBuilder;

use eyre::{Result, eyre};
use serde_json::Value;
use std::collections::HashMap;

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
            cache: SharedCache::new().into(),
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

    // TODO this should do the global block first, THEN all the other ones so that deployer is defined.
    pub async fn propogate_shared_data(&mut self) -> Result<()> {
        // Step 1: Resolve all provided values
        for block in &self.blocks {
            block.resolve_values(&self.cache).await?;
        }

        // Step 2: Collect missing requirements and conflicts
        let mut missing_requirements = HashMap::new();
        let mut conflicts = Vec::new();

        for (i, block) in self.blocks.iter().enumerate() {
            let block_missing = block.report_missing_values(&self.cache).await?;
            for req in block_missing {
                let (key, can_derive) = (req.0, req.1);

                // Check for conflicts (multiple blocks trying to derive the same value)
                if can_derive {
                    if let Some(Some(existing_idx)) = missing_requirements.get(&key) {
                        conflicts.push((key.clone(), i, *existing_idx));
                        continue;
                    }
                }

                // No conflict detected, update the map
                let entry = missing_requirements.entry(key).or_insert(if can_derive {
                    Some(i)
                } else {
                    None
                });

                // Update entry if this block can derive but previous one couldn't
                if can_derive && entry.is_none() {
                    *entry = Some(i);
                }
            }
        }

        // After collecting, handle conflicts
        if !conflicts.is_empty() {
            let first_conflict = &conflicts[0];
            return Err(eyre!(
                "Multiple blocks trying to derive value '{}': block {} and block {}",
                first_conflict.0,
                first_conflict.1,
                first_conflict.2
            ));
        }

        // Step 3: Try to resolve all derivable values
        let mut made_progress = true;

        while made_progress {
            made_progress = false;

            // Make a copy to avoid borrowing issues during iteration
            let requirements_to_process: Vec<(String, Option<usize>)> = missing_requirements
                .iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect();

            for (key, maybe_block_idx) in requirements_to_process {
                if let Some(block_idx) = maybe_block_idx {
                    let block = &self.blocks[block_idx];
                    if block.derive_value(&key, &self.cache, &self.vrm).await? {
                        // Value was resolved, remove from missing
                        missing_requirements.remove(&key);
                        made_progress = true;
                    }
                }
            }

            if !made_progress && !missing_requirements.is_empty() {
                // We tried but could not resolve everything
                break;
            }
        }

        Ok(())
    }

    // TODO maybe use log in here to print debug and info, etc
    pub async fn assemble_and_aggregate(&mut self) -> Result<Vec<Box<dyn Action>>> {
        let mut actions = Vec::new();

        // Assemble actions from all blocks
        for block in &self.blocks {
            let block_actions = block.assemble(&self.cache, &self.vrm).await?;
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
            let executor = self
                .cache
                .get_address("executor")
                .await
                .expect("BlockManager: Expected executor to be defined and to be an address");

            // Bound while loop to safe maximum.
            // MAX_ITERATIONS flow:
            //  Timelock + Multisend + Signer(which then goes to EOA) = 3.
            const MAX_ITERATIONS: u8 = 3;
            let mut meta_actions = Vec::new();
            let mut current_chunk = Vec::new();
            let mut non_eoa_action_senders = false;
            let mut iterations = 0;

            while !non_eoa_action_senders {
                if iterations == MAX_ITERATIONS {
                    return Err(eyre!(
                        "BlockManager: Failed to aggregate actions in {}",
                        MAX_ITERATIONS
                    ));
                }
                let mut action_iter = actions.into_iter().peekable();
                while let Some(action) = action_iter.next() {
                    let current_sender = action.sender();
                    match current_sender {
                        SenderType::EOA(addr) => {
                            if addr != executor {
                                return Err(eyre!("BlockManager: Wrong EOA"));
                            }
                            meta_actions.push(action);
                        }
                        SenderType::Signer(multisig) => {
                            // Convert into Approve Hash or Exec Transaction action, and push onto meta_actions
                            let meta_action = MultisigMetaAction::new(
                                multisig, executor, action, None, &self.vrm,
                            )
                            .await?;
                            meta_actions.push(Box::new(meta_action));
                        }
                        _ => {
                            // We reached this match arm so the final meta_actions will have NON EOA senders in it.
                            non_eoa_action_senders = true;

                            // Get next action as following arms can bundle multiple actions together.
                            let next_action = action_iter.peek();

                            // Add action to current chunk.
                            current_chunk.push(action);

                            let chunk_transition = match next_action {
                                Some(a) => {
                                    // If next_sender does not equal current sender, then we are at a chunk_transition
                                    let next_sender = a.sender();
                                    next_sender != current_sender
                                }
                                None => {
                                    // Always return true as we are at the end of the actions
                                    true
                                }
                            };
                            match current_sender {
                                SenderType::Multisig(multisig) => {
                                    if chunk_transition {
                                        // Batch current chunk into a Multisend meta action
                                        let multisend = self.cache.get_address("multisend").await;
                                        let meta_action = MultisendMetaAction::new(
                                            multisig,
                                            multisend,
                                            std::mem::take(&mut current_chunk),
                                        )?;
                                        meta_actions.push(Box::new(meta_action));
                                    }
                                }
                                SenderType::Timelock(timelock) => {
                                    if chunk_transition {
                                        // Batch current chunk into a Timelock meta action
                                        let timelock_admin = self.cache.get_address("timelock_admin").await.expect("BlockManager: Expected timelock_admin to be defined and an address");

                                        // TODO delay could be a value optionally read from the cache.
                                        // TODO timelock_admin can technically have sender type EOA too, so maybe write that into the cache too?
                                        // like the type of timelock_admin?
                                        let meta_action = TimelockMetaAction::new(
                                            timelock,
                                            None,
                                            std::mem::take(&mut current_chunk),
                                            &self.vrm,
                                            SenderType::Multisig(timelock_admin),
                                        )
                                        .await?;
                                        meta_actions.push(Box::new(meta_action));
                                    }
                                }
                                _ => unreachable!(), // This should never happen with match arm layout
                            }
                        }
                    }
                }
                // Copy meta_actions into actions.
                actions = std::mem::take(&mut meta_actions);
                // Increment iterations.
                iterations += 1;
            }

            Ok(actions)
        }
    }
}
