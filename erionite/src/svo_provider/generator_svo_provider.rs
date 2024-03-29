use std::sync::Arc;

use bevy::{prelude::default, tasks::AsyncComputeTaskPool};
use utils::DAabb;

use crate::generator::Generator;

pub struct GeneratorSvoProvider<G: Generator> {
    aabb: DAabb,

    generator: Arc<G>,

    root_svo: svo::TerrainCell,

    dirty_chunks: Vec<svo::CellPath>,
}

impl<G: Generator + 'static> GeneratorSvoProvider<G> {
    pub fn new(generator: impl Into<Arc<G>>, aabb: DAabb) -> Self{
        Self {
            aabb,
            generator: generator.into(),

            root_svo: default(),
            dirty_chunks: vec![],
        }
    }
}

impl<G: Generator + 'static> super::SvoProvider for GeneratorSvoProvider<G> {
    fn request_chunk(
        &mut self,
        path: svo::CellPath,
        subdivs: u32,
    ) -> bevy::tasks::Task<std::sync::Arc<svo::TerrainCell>> {
        let generator = Arc::clone(&self.generator);
        let aabb = self.aabb;
        AsyncComputeTaskPool::get().spawn(async move {
            log::debug!("Generating {path:?}@{subdivs}...");
            let result = generator.generate_chunk(
                aabb,
                path,
                subdivs,
            );
            log::debug!("Finished {path:?}@{subdivs}...");
            Arc::new(result)
        })
    }

    fn drain_dirty_chunks(&mut self) -> Box<[svo::CellPath]> {
        std::mem::take(&mut self.dirty_chunks).into_boxed_slice()
    }
}
