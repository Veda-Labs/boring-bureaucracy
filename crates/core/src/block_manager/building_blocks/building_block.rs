use crate::actions::action::Action;
use crate::block_manager::shared_cache::SharedCache;
use crate::utils::view_request_manager::ViewRequestManager;
use async_trait::async_trait;
use eyre::Result;

#[async_trait]
pub trait BuildingBlock: Send + Sync {
    async fn assemble(
        &self,
        cache: &SharedCache,
        vrm: &ViewRequestManager,
    ) -> Result<Vec<Box<dyn Action>>>;
    async fn resolve_state(
        &mut self,
        _cache: &SharedCache,
        _vrm: &ViewRequestManager,
    ) -> Result<()> {
        Ok(())
    }

    async fn resolve_values(&self, _cache: &SharedCache) -> Result<()> {
        Ok(()) // Default implementation does nothing
    }

    async fn report_missing_values(&self, _cache: &SharedCache) -> Result<Vec<(String, bool)>> {
        Ok(Vec::new()) // Default implementation returns empty vector
    }

    async fn derive_value(
        &self,
        _key: &str,
        _cache: &SharedCache,
        _vrm: &ViewRequestManager,
    ) -> Result<bool> {
        Ok(false) // Default implementation cannot resolve any values
    }
}
