// Tracks if stuff is deployed
// Looks for configuration errors/warnings
// Creates building blocks from jsons
// Shared contracts? Like building blocks can
// save addresses in the block manager

use eyre::{Result, eyre};
use into_trait::IntoTraitObject;
use serde::{Deserialize, Serialize};

// Define the common trait that all your structs will implement
pub trait Process {
    fn process(&self);
}

// Define your different structs
#[derive(Serialize, Deserialize, Debug)]
struct TypeA {
    value_a: String,
}

impl Process for TypeA {
    fn process(&self) {
        println!("Processing TypeA with value: {}", self.value_a);
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct TypeB {
    count: i32,
}

impl Process for TypeB {
    fn process(&self) {
        println!("Processing TypeB with count: {}", self.count);
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct TypeC {
    boolean: bool,
}

impl Process for TypeC {
    fn process(&self) {
        println!("Processing TypeC with boolean: {}", self.boolean);
    }
}

// Define a generic struct to hold the different types based on the "type" field
#[derive(Serialize, Deserialize, Debug, IntoTraitObject)]
#[trait_name(Process)]
enum Data {
    TypeA(TypeA),
    TypeB(TypeB),
    TypeC(TypeC),
}

pub fn process_json_value(value: &serde_json::Value) -> Result<Vec<Box<dyn Process>>> {
    let array = value.as_array().ok_or(eyre!("Expected a JSON array"))?;
    let mut parsed_data: Vec<Data> = Vec::new();

    for item in array {
        println!("DEBUG: {:?}", item);
        let data: Data = serde_json::from_value(item.clone())?;
        parsed_data.push(data);
    }

    let processed_items: Vec<Box<dyn Process>> = parsed_data
        .into_iter()
        .map(|item| item.into_trait_object())
        .collect();

    Ok(processed_items)
}

pub fn process_json_str(json_str: &str) -> Result<Vec<Box<dyn Process>>> {
    let parsed_data: Vec<Data> = serde_json::from_str(json_str)?;
    Ok(parsed_data
        .into_iter()
        .map(|item| item.into_trait_object())
        .collect())
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
            p.process();
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
        let _processed = process_json_value(&value)?;

        Ok(())
    }
}
