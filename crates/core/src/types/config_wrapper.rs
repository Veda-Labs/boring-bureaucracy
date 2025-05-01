use eyre::{Result, eyre};
use std::{env, fs};
use toml::Value;

pub struct ConfigWrapper {
    raw_config: Value,
}

impl ConfigWrapper {
    pub fn new(raw_config: Value) -> Self {
        Self { raw_config }
    }

    pub fn from_file(path: Option<&str>) -> Result<Self> {
        let config_content = if let Some(p) = path {
            fs::read_to_string(p)?
        } else {
            fs::read_to_string("config.toml")?
        };
        let raw_config: toml::Value = config_content.parse::<Value>()?;

        Ok(Self { raw_config })
    }

    pub fn get_product_config_value(
        &self,
        product: &str,
        network_id: u32,
        key: &str,
    ) -> Result<String> {
        // Try network specific value first
        let value = self
            .raw_config
            .get("product")
            .and_then(|p| p.get(product))
            .and_then(|p| p.get(&network_id.to_string()))
            .and_then(|p| p.get(key))
            .or_else(|| {
                // Fallback to default if network specific not found
                self.raw_config
                    .get("product")
                    .and_then(|p| p.get(product))
                    .and_then(|p| p.get("default"))
                    .and_then(|p| p.get(key))
            })
            .and_then(|v| v.as_str())
            .ok_or_else(|| eyre!("{} not found for product: {}", key, product))?;

        Ok(value.to_string())
    }

    pub fn get_product_config_value_or_default(
        &self,
        product: &str,
        network_id: u32,
        key: &str,
    ) -> String {
        let val = self.get_product_config_value(product, network_id, key);

        match val {
            Ok(v) => v,
            Err(_) => "0x0000000000000000000000000000000000000000".to_string(),
        }
    }

    pub fn get_product_strategists(&self, product: &str, network_id: u32) -> Result<Vec<String>> {
        // Try network specific value first
        let strategists = self
            .raw_config
            .get("product")
            .and_then(|p| p.get(product))
            .and_then(|p| p.get(&network_id.to_string()))
            .and_then(|p| p.get("strategists"))
            .or_else(|| {
                // Fallback to default if network specific not found
                self.raw_config
                    .get("product")
                    .and_then(|p| p.get(product))
                    .and_then(|p| p.get("default"))
                    .and_then(|p| p.get("strategists"))
            })
            .and_then(|s| s.as_array())
            .ok_or_else(|| eyre!("strategists not found for product: {}", product))?;

        let result = strategists
            .iter()
            .filter_map(|s| s.as_str().map(|s| s.to_string()))
            .collect();

        Ok(result)
    }

    pub fn get_rpc_url(&self, network_id: u32) -> Result<String> {
        let url_value = &self.raw_config["rpc_endpoints"][&network_id.to_string()];
        let url_str = url_value
            .as_str()
            .ok_or_else(|| eyre::eyre!("URL not found for network_id: {}", network_id))?;

        if url_str.starts_with("env:") {
            let env_var = &url_str[4..];
            env::var(env_var).map_err(|_| eyre::eyre!("Environment variable {} not set", env_var))
        } else {
            Ok(url_str.to_string())
        }
    }

    pub fn get_block_explorer_url(&self, network_id: u32) -> Result<String> {
        let url_value = &self.raw_config["block_explorers"][&network_id.to_string()];
        let url_str = url_value.as_str().ok_or_else(|| {
            eyre::eyre!(
                "Block explorer URL not found for network_id: {}",
                network_id
            )
        })?;

        Ok(url_str.trim_end_matches('/').to_string())
    }

    pub fn get_multisend_address(&self, network_id: u32) -> Result<String> {
        // Try network specific value first
        let network_value = self
            .raw_config
            .get("multi_send_address")
            .and_then(|m| m.get(&network_id.to_string()))
            .and_then(|m| m.as_str());

        // Fallback to default if network specific not found
        let default_value = self
            .raw_config
            .get("multi_send_address")
            .and_then(|m| m.get("default"))
            .and_then(|m| m.as_str());

        let address_str = network_value.or(default_value).ok_or_else(|| {
            eyre::eyre!("Multisend address not found for network_id: {}", network_id)
        })?;

        Ok(address_str.to_string())
    }
}
