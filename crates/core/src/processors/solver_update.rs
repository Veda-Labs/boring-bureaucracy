use crate::{
    actions::{
        admin_action::AdminAction, set_public_capability_action::SetPublicCapabilityAction,
        set_role_capability_action::SetRoleCapabilityAction,
        set_user_role_action::SetUserRoleAction,
    },
    types::config_wrapper::ConfigWrapper,
};
use alloy::primitives::Address;
use eyre::{Result, eyre};
use serde_json::Value;

enum Mode {
    Setup,
    TearDown,
}
// TODO this function could read state to se if it needs to update the fee or not.
// TODO old products like liquid eth have different functions so this will fail
pub fn process_solver_update(
    admin_actions: &mut Vec<Box<dyn AdminAction>>,
    cw: &ConfigWrapper,
    product: &str,
    network_id: u32,
    solver_data: &Value,
) -> Result<()> {
    // Extract setup or tear_down.
    let setup = solver_data.get("setup");
    let tear_down = solver_data.get("tear_down");
    let mode;

    if setup.is_some() && tear_down.is_some() {
        // Make sure one is false.
        let setup = match setup.unwrap().as_bool() {
            Some(val) => val,
            None => return Err(eyre::eyre!("setup must be a bool")),
        };
        let tear_down = match tear_down.unwrap().as_bool() {
            Some(val) => val,
            None => return Err(eyre::eyre!("tear_down must be a bool")),
        };
        if setup == tear_down {
            return Err(eyre::eyre!(
                "Exactly one of 'setup' or 'tear_down' must be true while the other must be false, or not provided"
            ));
        }
        if setup {
            mode = Mode::Setup
        } else {
            mode = Mode::TearDown;
        }
    } else if let Some(setup) = setup {
        let setup = match setup.as_bool() {
            Some(val) => val,
            None => return Err(eyre::eyre!("setup must be a bool")),
        };
        if !setup {
            return Err(eyre!("Only setup defined but it is false"));
        }
        mode = Mode::Setup;
    } else if let Some(tear_down) = tear_down {
        let tear_down = match tear_down.as_bool() {
            Some(val) => val,
            None => return Err(eyre::eyre!("tear_down must be a bool")),
        };
        if !tear_down {
            return Err(eyre!("Only tear_down defined but it is false"));
        }
        mode = Mode::TearDown;
    } else {
        // setup and tear_down were not provided so error.
        return Err(eyre!("'setup' or 'tear_down' must be specified"));
    }

    // Extract solver contract and allow self solves.
    let solver_contract = solver_data
        .get("solver_contract")
        .unwrap()
        .as_str()
        .unwrap();
    let solver_addr = solver_contract.parse::<Address>()?;
    let allow_self_solves = solver_data.get("allow_self_solves");
    let allow_self_solves = match allow_self_solves {
        Some(v) => match v.as_bool() {
            Some(b) => b,
            None => return Err(eyre::eyre!("allow_self_solves must be a bool")),
        },
        None => false,
    };

    // Get roles_authority address for the product
    let roles_authority_addr_str =
        cw.get_product_config_value(product, network_id, "roles_authority_address")?;
    let roles_authority_addr = roles_authority_addr_str.parse::<Address>()?;

    let enabled = match mode {
        Mode::Setup => true,
        Mode::TearDown => false,
    };

    // Add action so boring queue can call boringSolve
    let action = SetRoleCapabilityAction::new(
        roles_authority_addr,
        32,
        solver_addr,
        "boringSolve(address,address,address,uint256,uint256,bytes)".to_string(),
        enabled,
    );
    admin_actions.push(Box::new(action));
    // Allow solver eoa to call solve functions.
    let action = SetRoleCapabilityAction::new(
                roles_authority_addr,
                33,
                solver_addr,
                "boringRedeemSolve((uint96,address,address,uint128,uint128,uint40,uint24,uint24)[],address,bool)".to_string(),
                enabled,
            );
    admin_actions.push(Box::new(action));
    let action = SetRoleCapabilityAction::new(
                roles_authority_addr,
                33,
                solver_addr,
                "boringRedeemMintSolve((uint96,address,address,uint128,uint128,uint40,uint24,uint24)[],address,address,address,bool)".to_string(),
                enabled,
            );
    admin_actions.push(Box::new(action));
    // Grant required roles to new solver contract
    let action = SetUserRoleAction::new(roles_authority_addr, solver_addr, 31, enabled);
    admin_actions.push(Box::new(action));
    let action = SetUserRoleAction::new(roles_authority_addr, solver_addr, 12, enabled);
    admin_actions.push(Box::new(action));

    if !enabled && allow_self_solves {
        return Err(eyre!(
            "Tearing down a solver and enabling self solves does not make sense"
        ));
    }

    // Make self solve functions public if need be.
    if allow_self_solves {
        let action = SetPublicCapabilityAction::new(roles_authority_addr, solver_addr, "boringRedeemSelfSolve((uint96,address,address,uint128,uint128,uint40,uint24,uint24),address)".to_string(), true);
        admin_actions.push(Box::new(action));
        let action = SetPublicCapabilityAction::new(roles_authority_addr, solver_addr, "boringRedeemMintSelfSolve((uint96,address,address,uint128,uint128,uint40,uint24,uint24),address,address,address)".to_string(), true);
        admin_actions.push(Box::new(action));
    }

    Ok(())
}
