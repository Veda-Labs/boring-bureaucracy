// Tracks if stuff is deployed
// Looks for configuration errors/warnings
// Creates building blocks from jsons
// Shared contracts? Like building blocks can
// save addresses in the block manager

use super::building_blocks::building_block::{Actionable, BuildingBlock};
use eyre::Result;
use serde_json::Value;

pub fn process_json_value(value: Value) -> Result<Vec<Box<dyn Actionable>>> {
    let parsed_data: Vec<BuildingBlock> = serde_json::from_value(value)?;
    Ok(parsed_data
        .into_iter()
        .map(|item| item.into_trait_object())
        .collect())
}

pub fn process_json_str(json_str: &str) -> Result<Vec<Box<dyn Actionable>>> {
    let parsed_data: Vec<BuildingBlock> = serde_json::from_str(json_str)?;
    Ok(parsed_data
        .into_iter()
        .map(|item| item.into_trait_object())
        .collect())
}

pub struct BlockManager {
    pub blocks: Vec<BuildingBlock>,
    pub cache: Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_json_str() -> Result<()> {
        let json_data = r#"[
        { "TypeA": { "value_a": "hello" } },
        { "TypeB": { "count": 123 } },
        { "TypeC": { "boolean": true } },
        { "TypeA": { "value_a": "world" } }
    ]"#;
        let processed = process_json_str(json_data)?;
        for p in processed {
            p.to_actions()?;
        }
        Ok(())
    }

    #[test]
    fn test_process_json_value() -> Result<()> {
        let json_data = r#"[
            { "TypeA": { "value_a": "hello" } },
            { "TypeB": { "count": 123 } },
            { "TypeA": { "value_a": "world" } }
        ]"#;
        let value: serde_json::Value = serde_json::from_str(json_data)?;
        let processed = process_json_value(&value)?;

        Ok(())
    }
}
