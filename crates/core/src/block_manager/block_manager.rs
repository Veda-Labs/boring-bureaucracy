// Tracks if stuff is deployed
// Looks for configuration errors/warnings
// Creates building blocks from jsons
// Shared contracts? Like building blocks can
// save addresses in the block manager

use super::building_blocks::building_block::{Actionable, BuildingBlock, MissingCacheValuesError};
use eyre::Result;
use serde_json::Value;

pub struct BlockManager {
    pub blocks: Vec<Box<dyn Actionable>>,
    pub cache: Value,
}

impl BlockManager {
    pub fn new() -> Self {
        Self {
            blocks: vec![],
            cache: Value::Null,
        }
    }

    pub fn from_value(value: Value) -> Result<Self> {
        let mut s = Self::new();
        s.create_blocks_from_json_value(value)?;
        Ok(s)
    }

    pub fn from_str(json_str: &str) -> Result<Self> {
        let mut s = Self::new();
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

    pub async fn populate_cache(&mut self) -> Result<()> {
        const MAX_PASSES: usize = 5;
        let num_blocks = self.blocks.len();
        let mut results: Vec<Option<Result<Value>>> = Vec::with_capacity(num_blocks);
        results.resize_with(num_blocks, || None);

        for _ in 0..MAX_PASSES {
            let mut changed = false;

            for (i, block) in self.blocks.iter_mut().enumerate() {
                if let Some(Ok(Value::Null)) = &results[i] {
                    // Skip if result is Some(Ok(Null)) as it is done.
                    continue;
                }
                let res = block.resolve_and_contribute(&self.cache).await;
                match &res {
                    Ok(Value::Null) => results[i] = Some(res),
                    Ok(Value::Object(_)) => results[i] = Some(res),
                    Ok(_) => return Err(eyre::eyre!("Block returned unexpected JSON value")),
                    Err(e) => {
                        if let Some(_missing) = e.downcast_ref::<MissingCacheValuesError>() {
                            // TODO can print a debug with the missing values maybe?
                            // You can use missing.missing here if you want
                            // Not ready, will retry in next pass
                        } else {
                            return Err(eyre::eyre!("Block error: {}", e));
                        }
                    }
                }
            }

            // Merge Ok(Value::Object(_)) into cache, set to Ok(Value::Null) after merging
            for res in results.iter_mut() {
                if let Some(Ok(Value::Object(obj))) = res {
                    if !self.cache.is_object() {
                        self.cache = Value::Object(serde_json::Map::new());
                    }
                    if let Some(cache_map) = self.cache.as_object_mut() {
                        for (k, v) in obj.iter() {
                            if let Some(existing) = cache_map.get(k) {
                                if existing != v {
                                    return Err(eyre::eyre!(
                                        "Attempted to override existing cache key '{}' with a different value (old: {}, new: {})",
                                        k,
                                        existing,
                                        v
                                    ));
                                }
                                // else: values are the same, do nothing
                            } else {
                                cache_map.insert(k.clone(), v.clone());
                            }
                        }
                    }
                    *res = Some(Ok(Value::Null));
                    changed = true;
                }
            }

            // Stop if all are Ok(Value::Null) or Err
            if results
                .iter()
                .all(|r| matches!(r, Some(Ok(Value::Null)) | Some(Err(_))))
            {
                break;
            }

            // If nothing changed, break to avoid infinite loop
            if !changed {
                break;
            }
        }

        // Optionally, return an error if not all blocks resolved after MAX_PASSES
        if results
            .iter()
            .any(|r| !matches!(r, Some(Ok(Value::Null)) | Some(Err(_))))
        {
            eyre::bail!("Not all blocks could resolve after {} passes", MAX_PASSES);
        }

        Ok(())
    }
}
