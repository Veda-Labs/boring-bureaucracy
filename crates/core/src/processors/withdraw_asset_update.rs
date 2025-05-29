use crate::{
    actions::{
        admin_action::AdminAction, set_rate_provider_data_action::SetRateProviderData,
        stop_withdraws_in_asset_action::StopWithdrawsInAsset,
        update_withdraw_asset_action::UpdateWithdrawAsset,
    },
    bindings::{accountant::AccountantWithRateProviders, boring_queue::BoringOnChainQueue},
    types::config_wrapper::ConfigWrapper,
};
use alloy::primitives::Address;
use alloy::providers::ProviderBuilder;
use eyre::{Result, eyre};
use serde_json::Value;

pub async fn process_queue_asset_updates(
    admin_actions: &mut Vec<Box<dyn AdminAction>>,
    cw: &ConfigWrapper,
    product: &str,
    network_id: u32,
    asset_data: &Value,
) -> Result<()> {
    // Get addresses from config
    let queue_addr_str = cw.get_product_config_value(product, network_id, "queue_address")?;
    let accountant_addr_str =
        cw.get_product_config_value(product, network_id, "accountant_address")?;

    let queue_addr = queue_addr_str.parse::<Address>()?;
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
    let base_asset = accountant.base().call().await?;
    // Only need to check accountant asset if it is not the base.
    if asset_addr != base_asset.base {
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
    }

    // Query current withdraw asset data
    let queue = BoringOnChainQueue::new(queue_addr, provider);
    let current_withdraw_data = queue.withdrawAssets(asset_addr).call().await?;

    // Check if withdraw settings need updating
    let allow_withdraws = asset_data["allow_withdraws"]
        .as_bool()
        .ok_or_else(|| eyre!("allow_withdraws must be a boolean"))?;
    let seconds_to_maturity = asset_data["seconds_to_maturity"]
        .as_u64()
        .ok_or_else(|| eyre!("seconds_to_maturity must be a number"))?
        as u32;
    let minimum_seconds_to_deadline = asset_data["minimum_seconds_to_deadline"]
        .as_u64()
        .ok_or_else(|| eyre!("minimum_seconds_to_deadline must be a number"))?
        as u32;
    let min_discount = asset_data["min_discount"]
        .as_u64()
        .ok_or_else(|| eyre!("min_discount must be a number"))? as u16;
    let max_discount = asset_data["max_discount"]
        .as_u64()
        .ok_or_else(|| eyre!("max_discount must be a number"))? as u16;
    let minimum_shares = asset_data["minimum_shares"]
        .as_u64()
        .ok_or_else(|| eyre!("minimum_shares must be a number"))? as u128;

    if allow_withdraws != current_withdraw_data.allowWithdraws
        || seconds_to_maturity != current_withdraw_data.secondsToMaturity.to::<u32>()
        || minimum_seconds_to_deadline != current_withdraw_data.minimumSecondsToDeadline.to::<u32>()
        || min_discount != current_withdraw_data.minDiscount
        || max_discount != current_withdraw_data.maxDiscount
        || minimum_shares != current_withdraw_data.minimumShares.to::<u128>()
    {
        // Something is different, so we need to update it.
        if allow_withdraws {
            let seconds_to_maturity = asset_data["seconds_to_maturity"]
                .as_u64()
                .ok_or_else(|| eyre!("seconds_to_maturity must be a number"))?
                as u32;
            let minimum_seconds_to_deadline = asset_data["minimum_seconds_to_deadline"]
                .as_u64()
                .ok_or_else(|| eyre!("minimum_seconds_to_deadline must be a number"))?
                as u32;
            let min_discount = asset_data["min_discount"]
                .as_u64()
                .ok_or_else(|| eyre!("min_discount must be a number"))?
                as u16;
            let max_discount = asset_data["max_discount"]
                .as_u64()
                .ok_or_else(|| eyre!("max_discount must be a number"))?
                as u16;
            let minimum_shares = asset_data["minimum_shares"]
                .as_u64()
                .ok_or_else(|| eyre!("minimum_shares must be a number"))?
                as u128;

            let action = UpdateWithdrawAsset::new(
                queue_addr,
                asset_addr,
                seconds_to_maturity,
                minimum_seconds_to_deadline,
                min_discount,
                max_discount,
                minimum_shares,
            );
            admin_actions.push(Box::new(action));
        } else {
            let action = StopWithdrawsInAsset::new(queue_addr, asset_addr);
            admin_actions.push(Box::new(action));
        }
    }

    Ok(())
}
