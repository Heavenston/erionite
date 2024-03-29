use std::sync::Arc;

use bevy::{ecs::component::Component, tasks::Task};

pub mod generator_svo_provider;

pub trait SvoProvider {
    fn request_chunk(
        &mut self,
        path: svo::CellPath,
        subdivs: u32,
    ) -> Task<Arc<svo::TerrainCell>>;

    /// Gets and resets a accumulated list of chunks that changed since last
    /// call to this function
    fn drain_dirty_chunks(&mut self) -> Box<[svo::CellPath]>;
}

#[derive(Component)]
pub struct SvoProviderComponent(pub Box<dyn SvoProvider + Send + Sync>);

impl<T> From<T> for SvoProviderComponent
where T: SvoProvider + Send + Sync + 'static
{
    fn from(value: T) -> Self {
        Self(Box::new(value) as Box<_>)
    }
}

impl std::ops::Deref for SvoProviderComponent {
    type Target = dyn SvoProvider;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl std::ops::DerefMut for SvoProviderComponent {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}
