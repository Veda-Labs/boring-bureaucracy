use crate::{
    actions::{
        add_asset_action::AddAsset, admin_action::AdminAction, remove_asset_action::RemoveAsset,
        set_rate_provider_data_action::SetRateProviderData,
        update_asset_data_action::UpdateAssetData,
    },
    bindings::{accountant::AccountantWithRateProviders, teller::TellerWithMultiAssetSupport},
    types::config_wrapper::ConfigWrapper,
};
use alloy::primitives::Address;
use alloy::providers::ProviderBuilder;
use eyre::{Result, eyre};
use serde_json::Value;

// TODO I guess this should handle withdraws too?
pub async fn process_asset_updates(
    admin_actions: &mut Vec<Box<dyn AdminAction>>,
    cw: &ConfigWrapper,
    product: &str,
    network_id: u32,
    asset_data: &Value,
) -> Result<()> {
    // Get addresses from config
    let teller_addr_str = cw.get_product_config_value(product, network_id, "teller_address")?;
    let accountant_addr_str =
        cw.get_product_config_value(product, network_id, "accountant_address")?;

    let teller_addr = teller_addr_str.parse::<Address>()?;
    let accountant_addr = accountant_addr_str.parse::<Address>()?;
    let asset_addr = asset_data["asset"]
        .as_str()
        .ok_or_else(|| eyre!("asset must be a string"))?
        .parse::<Address>()?;

    // Query current rate provider data
    let provider = ProviderBuilder::new()
        .on_builtin(&cw.get_rpc_url(network_id)?)
        .await?;
    let accountant = AccountantWithRateProviders::new(accountant_addr, provider.clone());
    let current_rate_data = accountant.rateProviderData(asset_addr).call().await?;

    // Check if rate provider data needs updating
    let new_is_pegged = asset_data["is_pegged_to_base"]
        .as_bool()
        .ok_or_else(|| eyre!("is_pegged_to_base must be a boolean"))?;
    let new_rate_provider: Address = asset_data["rate_provider"]
        .as_str()
        .ok_or_else(|| eyre!("rate_provider must be a string"))?
        .parse()?;

    if (new_is_pegged && new_rate_provider != Address::ZERO)
        || (!new_is_pegged && new_rate_provider == Address::ZERO)
    {
        return Err(eyre!(
            "Accountant asset must either be pegged to base, or rate provider is non zero"
        ));
    }

    if current_rate_data.rpd.isPeggedToBase != new_is_pegged
        || current_rate_data.rpd.rateProvider != new_rate_provider
    {
        let action = SetRateProviderData::new(
            accountant_addr,
            asset_addr,
            new_is_pegged,
            new_rate_provider,
        );
        admin_actions.push(Box::new(action));
    }
    // Check if asset data needs updating
    let new_allow_deposits = asset_data["allow_deposits"]
        .as_bool()
        .ok_or_else(|| eyre!("allow_deposits must be a boolean"))?;
    let new_allow_withdraws = asset_data["allow_withdraws"]
        .as_bool()
        .ok_or_else(|| eyre!("allow_withdraws must be a boolean"))?;
    let new_share_premium = asset_data["share_premium"]
        .as_u64()
        .ok_or_else(|| eyre!("share_premium must be a number"))? as u16;

    // Query current asset data
    let teller = TellerWithMultiAssetSupport::new(teller_addr, provider);
    let result = teller.assetData(asset_addr).call().await;
    match result {
        Ok(current_asset_data) => {
            if current_asset_data.asset.allowDeposits != new_allow_deposits
                || current_asset_data.asset.allowWithdraws != new_allow_withdraws
                || current_asset_data.asset.sharePremium != new_share_premium
            {
                let action = UpdateAssetData::new(
                    teller_addr,
                    asset_addr,
                    new_allow_deposits,
                    new_allow_withdraws,
                    new_share_premium,
                );
                admin_actions.push(Box::new(action));
            }
        }
        Err(_) => {
            // Call failed try using legacy teller interface.
            if new_allow_deposits != new_allow_withdraws {
                return Err(eyre!(
                    "Legacy teller interface requires allow_deposits and allow_withdraws to be the same value"
                ));
            }
            let is_supported = teller.isSupported(asset_addr).call().await?;
            if !is_supported.supported && new_allow_deposits {
                // Create addAsset action
                let action = AddAsset::new(teller_addr, asset_addr);
                admin_actions.push(Box::new(action));
            } else if is_supported.supported && !new_allow_deposits {
                // Create removeAsset call
                let action = RemoveAsset::new(teller_addr, asset_addr);
                admin_actions.push(Box::new(action));
            }
        }
    }

    Ok(())
}
