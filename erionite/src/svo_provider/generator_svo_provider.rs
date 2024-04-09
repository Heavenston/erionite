use std::sync::{Arc, Mutex};

use bevy::prelude::default;
use either::Either;
use utils::DAabb;

use crate::task_runner::{self, Task};
use crate::generator::Generator;

struct SvoData {
    root_svo: svo::TerrainCell,
    generated: svo::Cell<svo::StatBool>,
}

pub struct GeneratorSvoProvider<G: Generator> {
    aabb: DAabb,

    generator: Arc<G>,

    svo_data: Arc<Mutex<SvoData>>,
    dirty_chunks: Arc<Mutex<Vec<svo::CellPath>>>,
}

impl<G: Generator + 'static> GeneratorSvoProvider<G> {
    pub fn new(generator: impl Into<Arc<G>>, aabb: DAabb) -> Self{
        let generator = generator.into();

        let init_depth = 3;
        let root_svo = generator.generate_chunk(
            aabb, svo::CellPath::new(), init_depth
        );
        Self {
            aabb,
            generator,

            svo_data: Arc::new(Mutex::new(SvoData {
                root_svo,
                generated: svo::Cell::new_with_depth(init_depth, svo::StatBool(true)),
            })),
            dirty_chunks: default(),
        }
    }
}

impl<G: Generator + 'static> super::SvoProvider for GeneratorSvoProvider<G> {
    fn request_chunk(
        &mut self,
        path: svo::CellPath,
        subdivs: u32,
    ) -> Task<std::sync::Arc<svo::TerrainCell>> {
        let generator = Arc::clone(&self.generator);
        let aabb = self.aabb;

        let data = self.svo_data.clone();
        let dirties = self.dirty_chunks.clone();

        task_runner::spawn(move || {
            let must_regen = {
                let lock = data.lock().unwrap();
                let (fpath, cell) = lock.generated.follow_path(path);
                let already_gen = fpath == path && match cell.data() {
                    Either::Left(l) => l.all,
                    Either::Right(r) => r.0,
                } && cell.depth() >= subdivs;
                !already_gen
            };
            let mut lock;
            if must_regen {
                let result = generator.generate_chunk(
                    aabb,
                    path,
                    subdivs,
                );
                lock = data.lock().unwrap();
                *lock.root_svo.follow_internal_path(path) = result;
                lock.root_svo.update_on_path(path);

                *lock.generated.follow_path_mut(path).1 = svo::LeafCell {
                    data: svo::StatBool(false),
                }.into();
                *lock.generated.follow_internal_path(path) = svo::Cell::new_with_depth(
                    subdivs,
                    svo::StatBool(true)
                );
                dirties.lock().unwrap().extend(path.neighbors().map(|(_, n)| n));
            }
            else {
                lock = data.lock().unwrap();
            }

            Arc::new(lock.root_svo.clone())
        })
    }

    fn drain_dirty_chunks(&mut self) -> Box<[svo::CellPath]> {
        std::mem::take(&mut *self.dirty_chunks.lock().unwrap()).into_boxed_slice()
    }
}
