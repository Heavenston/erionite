use std::sync::{Arc, Mutex};

use bevy::prelude::default;
use bevy::utils::HashSet;
use either::Either;
use svo::StatInt;
use utils::DAabb;

use crate::task_runner::{self, Task};
use crate::generator::Generator;

struct SharedData {
    root_svo: svo::TerrainCell,
    generated: svo::Cell<svo::StatInt<u32>>,
}

pub struct GeneratorSvoProvider<G: Generator> {
    aabb: DAabb,

    generator: Arc<G>,

    svo_data: Arc<Mutex<SharedData>>,
    dirty_chunks: Arc<Mutex<HashSet<svo::CellPath>>>,
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

            svo_data: Arc::new(Mutex::new(SharedData {
                root_svo,
                generated: svo::LeafCell::new(StatInt(init_depth)).into(),
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
                let (found_path, found) = lock.generated.follow_path(path);
                let gen_depth = match found.data() {
                    Either::Left(l) => l.min,
                    Either::Right(r) => r.0,
                };
                gen_depth + found_path.len() < subdivs + path.len()
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

                *lock.generated.follow_internal_path(path) =
                    svo::LeafCell::new(StatInt(subdivs)).into();
                lock.generated.update_on_path(path);

                dirties.lock().unwrap()
                    .extend(
                        path.neighbors().map(|(_, n)| n)
                            .flat_map(|n|
                                n.parents().chain(std::iter::once(n))
                            )
                    );
            }
            else {
                lock = data.lock().unwrap();
            }

            Arc::new(lock.root_svo.clone())
        })
    }

    fn drain_dirty_chunks(&mut self) -> Box<[svo::CellPath]> {
        std::mem::take(&mut *self.dirty_chunks.lock().unwrap())
            .into_iter().collect::<Box<[_]>>()
    }
}
