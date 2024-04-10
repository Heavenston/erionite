use std::sync::{Arc, Mutex};

use bevy::prelude::default;
use bevy::utils::HashSet;
use utils::DAabb;

use crate::task_runner::{self, Task};
use crate::generator::Generator;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GeneratedDepthData(pub i64);

impl Default for GeneratedDepthData {
    fn default() -> Self {
        Self(-1)
    }
}

impl svo::Data for GeneratedDepthData {
    type Internal = GeneratedDepthData;
}

impl svo::InternalData for GeneratedDepthData {}

impl svo::SplittableData for GeneratedDepthData {
    fn split(self) -> (Self::Internal, [Self; 8]) {
        (
            self,
            [Self(self.0-1); 8]
        )
    }
}

impl svo::AggregateData for GeneratedDepthData {
    fn aggregate<'a>(
        children: [svo::EitherDataRef<Self>; 8]
    ) -> Self::Internal {
        Self(children.into_iter().map(|c| c.into_inner().0).min().expect("8") + 1)
    }
}

struct SharedData {
    root_svo: svo::TerrainCell,
    generated: svo::Cell<GeneratedDepthData>,
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
                generated: svo::LeafCell::new(
                    GeneratedDepthData(init_depth.into())
                ).into(),
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
                (found.data().into_inner().0 + i64::from(found_path.len()))
                    <
                From::from(subdivs + path.len())
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
                    svo::LeafCell::new(GeneratedDepthData(subdivs.into())).into();
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
