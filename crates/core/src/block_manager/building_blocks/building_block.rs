use crate::actions::action::Action;
use crate::block_manager::shared_cache::SharedCache;
use crate::utils::view_request_manager::ViewRequestManager;
use async_trait::async_trait;
use eyre::Result;

#[async_trait]
pub trait BuildingBlock: Send + Sync {
    async fn assemble(&self, vrm: &ViewRequestManager) -> Result<Vec<Box<dyn Action>>>;
    async fn resolve_state(&mut self, cache: &SharedCache, vrm: &ViewRequestManager) -> Result<()>;
}
