// Tracks if stuff is deployed
// Looks for configuration errors/warnings
// Creates building blocks from jsons
// Shared contracts? Like building blocks can
// save addresses in the block manager

use eyre::{Result, eyre};
use serde::{Deserialize, Serialize};

// Define the common trait that all your structs will implement
trait Process {
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

// Define a generic struct to hold the different types based on the "type" field
#[derive(Serialize, Deserialize, Debug)]
enum Data {
    TypeA(TypeA),
    TypeB(TypeB),
}

macro_rules! impl_into_trait_object {
    ($enum_name:ident, $trait_name:ident, $($variant:ident),*) => {
        impl $enum_name {
            pub fn into_trait_object(self) -> Box<dyn $trait_name> {
                match self {
                    $(
                        $enum_name::$variant(data) => Box::new(data) as Box<dyn $trait_name>,
                    )*
                }
            }
        }
    };
}

impl_into_trait_object!(Data, Process, TypeA, TypeB);

// impl Data {
//     pub fn into_trait_object(self) -> Box<dyn Process> {
//         match self {
//             Data::TypeA(d) => Box::new(d),
//             Data::TypeB(d) => Box::new(d),
//         }
//     }
// }

fn process_json_value(value: &serde_json::Value) -> Result<Vec<Box<dyn Process>>> {
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

fn process_json_str(json_str: &str) -> Result<Vec<Box<dyn Process>>> {
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
        { "TypeA": { "value_a": "world" } }
    ]"#;
        let processed = process_json_str(json_data)?;
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
