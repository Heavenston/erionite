mod chunk_svo;
use std::{ops::Range, sync::Arc};

use chunk_svo::*;

use ordered_float::OrderedFloat;
use bevy::{ecs::system::EntityCommands, prelude::*, tasks::{block_on, AsyncComputeTaskPool, Task}};
use svo::{mesh_generation::marching_cubes, Cell, CellPath};
use utils::{AabbExt, DAabb, RangeExt};

use crate::svo_provider::SvoProviderComponent;

pub struct SvoRendererPlugin {
    
}

impl Default for SvoRendererPlugin {
    fn default() -> Self {
        Self{}
    }
}

impl Plugin for SvoRendererPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            new_renderer_system,
            chunks_subdivs_system, chunks_splitting_system, chunk_system
        ).chain());
    }
}

#[derive(Bundle)]
pub struct SvoRendererBundle {
    pub transform: TransformBundle,
    pub svo_render: SvoRendererComponent,
    pub svo_provider: SvoProviderComponent,
}

pub struct SvoRendererComponentOptions {
    pub total_subdivs: Range<u32>,
    /// Chunks with or more subdivs are splitted
    pub chunk_split_subdivs: u32,
    /// Chunks with or less subdivs are merged
    pub chunk_merge_subdivs: u32,

    /// start is the camera distance at which the chunk should have max_subdivs
    /// end is the distance for lowest res
    pub chunk_subdiv_distances: Range<f64>,

    pub root_aabb: DAabb,

    pub on_new_chunk: Option<Box<dyn FnMut(EntityCommands) -> () + Send + Sync>>,
}

#[derive(Component)]
pub struct SvoRendererComponent {
    pub options: SvoRendererComponentOptions,

    chunks_svo: svo::Cell<ChunkSvoData>,

    chunks_to_split: Vec<svo::CellPath>,
    chunks_to_merge: Vec<svo::CellPath>,
}

impl SvoRendererComponent {
    pub fn new(options: SvoRendererComponentOptions) -> Self {
        Self {
            options,
            
            chunks_svo: default(),

            chunks_to_split: vec![],
            chunks_to_merge: vec![],
        }
    }
}

#[derive(Component)]
pub struct ChunkComponent {
    pub path: svo::CellPath,
    pub target_subdivs: u32,

    /// Subdivs of the *last requested* data
    data_subdivs: u32,
    data: Option<Arc<svo::TerrainCell>>,
    chunk_request_task: Option<Task<Arc<svo::TerrainCell>>>,
    mesh_task: Option<Task<Mesh>>,

    /// If true the data should be requested
    should_update_data: bool,
    /// If true the mesh should be recomputed
    should_update_mesh: bool,
}

impl ChunkComponent {
    fn new(path: svo::CellPath) -> Self {
        Self {
            path,

            target_subdivs: 0,
            data_subdivs: 0,

            data: None,

            chunk_request_task: None,
            mesh_task: None,

            should_update_data: false,
            should_update_mesh: false,
        }
    }

    fn with_update_data(self) -> Self {
        Self {
            should_update_data: true,
            ..self
        }
    }
}

fn new_renderer_system(
    mut commands: Commands,
    mut svo_renders: Query<(Entity, &mut SvoRendererComponent), Added<SvoRendererComponent>>,
) {
    for (entity, mut renderer) in &mut svo_renders {
        commands.entity(entity).insert(VisibilityBundle::default());
        let root_chunk_entitiy = commands.spawn((
            ChunkComponent::new(CellPath::new()),
            TransformBundle::default(),
            VisibilityBundle::default(),
        )).set_parent(entity).id();
        renderer.chunks_svo = svo::LeafCell {
            data: ChunkSvoData { entity: Some(root_chunk_entitiy) }
        }.into();
        if let Some(on_new_chunk) = &mut renderer.options.on_new_chunk {
            on_new_chunk(commands.entity(root_chunk_entitiy));
        }
    }
}

/// Updates chunks target_subdivs
fn chunks_subdivs_system(
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut chunks: Query<&mut ChunkComponent>,
    mut svo_renders: Query<(&mut SvoRendererComponent, &GlobalTransform)>,
) {
    let cameras_poses = cameras.iter()
        .filter(|(c, _)| c.is_active)
        .map(|(_, t)| t.translation())
        .collect::<Vec<_>>();

    for (mut renderer, &trans) in svo_renders.iter_mut() {
        let relative_camera_poses = cameras_poses.iter()
            .map(|&cp| trans * cp)
            .collect::<Vec<_>>();

        // Later swaped with renderer's version
        let mut chunks_to_split = vec![];
        let mut chunks_to_merge = vec![];

        for svo::SvoIterItem {
            cell: chunkcell, path: chunkpath,
        } in renderer.chunks_svo.iter() {
            let Some(entity) = chunkcell.data.entity
                else { continue; };
            let Ok(mut chunk) = chunks.get_mut(entity)
            else {
                log::warn!("Stored chunk entity does not exist");
                continue;
            };
            let aabb = chunkpath.get_aabb(renderer.options.root_aabb);

            let Some(closest_camera_dist) = relative_camera_poses.iter()
                .map(Vec3::as_dvec3)
                .map(|campos| aabb.closest_point(campos).distance_squared(campos))
                .min_by_key(|&d| OrderedFloat(d))
            else { continue };

            let dists = &renderer.options.chunk_subdiv_distances;
            let subdivs_range = renderer.options.total_subdivs.range_map(|&x| f64::from(x));
            // 0. is minimum 1. is maximum subdivs
            let subdiv_proportion = (
                dists.clamped(closest_camera_dist) - dists.start
            ) / dists.extent();
            let subdivs = subdivs_range.extent() * subdiv_proportion + subdivs_range.start;
            let subdivs = (subdivs.round() as u32).saturating_sub(chunkpath.depth());
           
            if chunk.target_subdivs != subdivs {
                chunk.should_update_data = true;
                chunk.target_subdivs = subdivs;
            }

            if chunk.target_subdivs >= renderer.options.chunk_split_subdivs {
                chunks_to_split.push(chunkpath);
            }
            if chunk.target_subdivs <= renderer.options.chunk_merge_subdivs {
                chunks_to_merge.push(chunkpath);
            }
        }

        std::mem::swap(&mut chunks_to_split, &mut renderer.chunks_to_split);
        std::mem::swap(&mut chunks_to_merge, &mut renderer.chunks_to_merge);
    }
}

/// Splits / merges chunks
fn chunks_splitting_system(
    mut commands: Commands,
    mut chunks: Query<&mut ChunkComponent>,
    mut svo_renders: Query<(Entity, &mut SvoRendererComponent, &mut SvoProviderComponent, &GlobalTransform)>,
) {
    for (renderer_entity, mut renderer, mut provider, &trans) in svo_renders.iter_mut() {
        let chunks_to_split = std::mem::take(&mut renderer.chunks_to_split);
        let chunks_to_merge = std::mem::take(&mut renderer.chunks_to_merge);

        for chunkpath in chunks_to_split {
            let mut on_new_chunk = renderer.options.on_new_chunk.take();
            let cell = renderer.chunks_svo.follow_path_mut(chunkpath).1;
            if let Some(e) = cell.try_leaf_mut().and_then(|leaf| leaf.data.entity.take()) {
                commands.entity(e).despawn();
            }
            cell.split();

            for child in CellPath::components() {
                let child_path = chunkpath.with_push(child);
                let child_cell = cell.as_inner_mut().get_child_mut(child).as_leaf_mut();

                let chunk_entitiy = commands.spawn((
                    ChunkComponent::new(child_path),
                    TransformBundle::default(),
                    VisibilityBundle::default(),
                )).set_parent(renderer_entity).id();
                child_cell.data.entity = Some(chunk_entitiy);
                if let Some(on_new_chunk) = &mut on_new_chunk {
                    on_new_chunk(commands.entity(chunk_entitiy));
                }
            }

            renderer.options.on_new_chunk = on_new_chunk;
        }

        'splits: for chunkpath in chunks_to_merge {
            // check if merging would mean immediately splitting ('overcrowded' chunk)
            for (_, neighbor) in chunkpath.neighbors() {
                let Some(nleaf) =
                    renderer.chunks_svo.follow_path(neighbor).1.try_leaf()
                // not a leaf = either its children will be merged later
                // or merging would create an overcroweded chunk
                else { continue 'splits; };
                // non existent = merge is probably fine ^^
                let Some(nchunk) = nleaf.data.entity.and_then(|e| chunks.get(e).ok())
                else { continue; };
                // is overcrowded
                if nchunk.target_subdivs+1 > renderer.options.chunk_split_subdivs {
                    continue 'splits;
                }
            }
            // TODO: Perform merging
        }

        let dirty = provider.drain_dirty_chunks();
        for &c in &*dirty {
            for chunkcell in renderer.chunks_svo.follow_path(c).1.iter() {
                let Some(entity) = chunkcell.cell.data.entity
                else {continue};
                let Ok(mut chunk) = chunks.get_mut(entity)
                else {continue};
                chunk.should_update_data = true;
            }
        }
    }
}

/// Updates chunk meshes / anything per-chunk
fn chunk_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,

    mut chunks: Query<(Entity, &mut ChunkComponent, &Parent)>,
    mut svo_renders: Query<(&mut SvoRendererComponent, &mut SvoProviderComponent)>,
) {
    for (chunk_entitiy, mut chunk, parent) in chunks.iter_mut() {
        let Ok((renderer, mut provider)) = svo_renders.get_mut(parent.get())
        else { continue; };

        if chunk.should_update_data || chunk.target_subdivs != chunk.data_subdivs {
            chunk.chunk_request_task = Some(
                provider.request_chunk(
                    chunk.path,
                    chunk.target_subdivs
                )
            );
            chunk.data_subdivs = chunk.target_subdivs;
            chunk.should_update_data = false;
        }

        if let Some(task) = chunk.chunk_request_task
            .take_if(|task| task.is_finished())
        {
            chunk.data = Some(block_on(task));
            chunk.should_update_mesh = true;
        }

        if let Some(data) =
            chunk.should_update_mesh.then_some(&chunk.data).cloned().flatten()
        {
            chunk.should_update_mesh = false;

            let chunkpath = chunk.path;
            let root_aabb = renderer.options.root_aabb;
            let subdivs = chunk.target_subdivs;
            chunk.mesh_task = Some(AsyncComputeTaskPool::get().spawn(async move {
                let mut out = marching_cubes::Out::new(false);
                log::debug!("Rendering mesh...");

                marching_cubes::run(
                    &mut out, chunkpath, &*data, root_aabb.into(), subdivs
                );
                log::debug!("Finished mesh");
                out.into_mesh()
            }));
        }

        if let Some(task) = chunk.mesh_task
            .take_if(|task| task.is_finished())
        {
            let new_mesh = meshes.add(block_on(task));
            commands.entity(chunk_entitiy).insert(new_mesh);
        }
    }
}
