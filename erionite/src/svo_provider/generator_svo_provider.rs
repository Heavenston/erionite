use std::sync::{Arc, Mutex};

use bevy::prelude::default;
use bevy::utils::HashSet;
use utils::DAabb;
use itertools::Itertools;

use crate::task_runner::{self, Task, TaskHandle};
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

impl svo::MergeableData for GeneratedDepthData {
    fn can_merge(
        _this: &Self::Internal,
        children: [&Self; 8]
    ) -> bool {
        children.iter().map(|x| x.0).all_equal()
    }

    fn merge(
        _this: Self::Internal,
        children: [Self; 8]
    ) -> Self {
        let v = children.iter().map(|x| x.0).min().unwrap_or_default();
        Self(v + 1)
    }
}

#[derive(Debug, Clone)]
struct GenPromise {
    started: bool,
    handle: TaskHandle<Arc<svo::TerrainCell>>,
    depth: i64,
}

#[derive(Debug, Default, Clone)]
struct GenTaskData {
    promise: Option<GenPromise>,
}

impl GenTaskData {
    pub fn clone_lower(&self) -> Self {
        let Some(task) = self.promise.clone()
        else {
            return Self { promise: None };
        };

        Self {
            promise: Some(GenPromise {
                started: false,
                handle: task.handle,
                depth: task.depth - 1,
            })
        }
    }
}

impl svo::Data for GenTaskData {
    type Internal = Self;
}

impl svo::InternalData for GenTaskData {
}

impl svo::SplittableData for GenTaskData {
    fn split(self) -> (Self::Internal, [Self; 8]) {
        let lower = self.clone_lower();
        (self, [
            lower.clone(), lower.clone(), lower.clone(), lower.clone(),
            lower.clone(), lower.clone(), lower.clone(), lower,
        ])
    }
}

impl svo::MergeableData for GenTaskData {
    fn can_merge(
        _this: &Self::Internal,
        children: [&Self; 8]
    ) -> bool {
        children.iter().map(|x| x.promise.as_ref()).all(|x|
            x.is_none() || x.is_some_and(|x| x.handle.canceled() || x.handle.finished())
        )
    }

    fn merge(
        this: Self::Internal,
        _children: [Self; 8]
    ) -> Self {
        this
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

    gen_target: svo::BoxCell<GenTaskData>,
}

impl<G: Generator + 'static> GeneratorSvoProvider<G> {
    pub fn new(generator: impl Into<Arc<G>>, aabb: DAabb) -> Self{
        let generator = generator.into();

        let init_depth = 6;
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

            gen_target: default(),
        }
    }

    pub fn start_promise(
        &self,
        path: svo::CellPath,
        subdivs: u32,
        handle: TaskHandle<Arc<svo::TerrainCell>>,
    ) {
        let generator = Arc::clone(&self.generator);
        let aabb = self.aabb;

        let data = self.svo_data.clone();
        let dirties = self.dirty_chunks.clone();

        let handle2 = handle.clone();
        let task = task_runner::spawn::<(), _>(move || {
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

                if handle.canceled() {
                    return;
                }
                
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

            if handle.canceled() {
                return;
            }
            handle.finish(Arc::new(lock.root_svo.clone()));
        });

        handle2.add_parent(task);
    }
}

impl<G: Generator + 'static> super::SvoProvider for GeneratorSvoProvider<G> {
    fn update(&mut self) {
        let mut todo = Vec::new();
        todo.push(svo::CellPath::new());

        let mut gen_target = std::mem::take(&mut self.gen_target);
        gen_target.simplify();

        while let Some(path) = todo.pop() {
            let (found_path, cell) = gen_target.follow_path_mut(path);
            debug_assert_eq!(found_path, path);

            let is_task_started = 'is_task_started: {
                let data = cell.data_mut().into_inner();

                let Some(promise) = &mut data.promise
                else { break 'is_task_started false; };

                if promise.handle.finished() || promise.handle.canceled() {
                    break 'is_task_started false;
                }

                if promise.started {
                    true
                }
                else {
                    promise.started = true;

                    self.start_promise(
                        path, promise.depth as u32, promise.handle.clone()
                    );

                    true
                }
            };

            // We cannot generate children if the parent has not finished
            if is_task_started {
                continue;
            }

            if cell.has_children() {
                todo.extend(path.children());
            }
        }

        self.gen_target = gen_target;
    }

    fn request_chunk(
        &mut self,
        path: svo::CellPath,
        subdivs: u32,
    ) -> Task<Arc<svo::TerrainCell>> {
        let isubdivs = i64::from(subdivs);
        let cell = self.gen_target.follow_internal_path(path);
        
        if let Some(promise) = &cell.data().into_inner().promise {
            if promise.depth >= isubdivs {
                if let Some(task) = promise.handle.upgrade() {
                    return task;
                }
            }
        }

        let task = Task::new();

        *cell = svo::LeafCell {
            data: GenTaskData {
                promise: Some(GenPromise {
                    handle: task.handle(),
                    depth: isubdivs,
                    started: false,
                }),
            },
        }.into();

        task
    }

    fn drain_dirty_chunks(&mut self) -> Box<[svo::CellPath]> {
        std::mem::take(&mut *self.dirty_chunks.lock().unwrap())
            .into_iter().collect::<Box<[_]>>()
    }
}
