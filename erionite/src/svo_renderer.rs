mod chunk_svo;
use std::sync::Arc;

use chunk_svo::*;

use ordered_float::OrderedFloat;
use bevy::{ecs::system::EntityCommands, prelude::*, render::primitives::Aabb, tasks::{block_on, AsyncComputeTaskPool, Task}};
use bevy_rapier3d::prelude::*;
use svo::{mesh_generation::marching_cubes, CellPath};
use utils::{AabbExt, DAabb};

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
    pub max_subdivs: u32,
    pub min_subdivs: u32,

    /// Chunks with more subdivs are splitted
    pub chunk_split_subdivs: u32,
    /// Chunks with less subdivs are merged
    pub chunk_merge_subdivs: u32,

    pub chunk_subdiv_half_life: f64,

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

#[derive(Default, Component)]
pub struct ChunkComponent {
    pub path: svo::CellPath,
    pub target_subdivs: u32,

    should_update_data: bool,
    data_subdivs: u32,
    data_task: Option<Task<Arc<svo::TerrainCell>>>,
    data: Option<Arc<svo::TerrainCell>>,

    should_update_mesh: bool,
    mesh_subdivs: u32,
    mesh_task: Option<Task<Option<Mesh>>>,

    should_update_collider: bool,
    collider_subdivs: u32,
    collider_task: Option<Task<Option<Collider>>>,
}

impl ChunkComponent {
    fn new(path: svo::CellPath) -> Self {
        Self {
            path,

            ..default()
        }
    }

    pub fn is_generating(&self) -> bool {
        self.data_task.is_some()
    }

    pub fn is_generating_mesh(&self) -> bool {
        self.mesh_task.is_some()
    }

    pub fn is_generating_collider(&self) -> bool {
        self.collider_task.is_some()
    }
}

fn new_renderer_system(
    mut commands: Commands,
    mut svo_renders: Query<(Entity, &mut SvoRendererComponent), Added<SvoRendererComponent>>,
) {
    for (entity, mut renderer) in &mut svo_renders {
        let svo::Cell::Leaf(chunk_leaf) = &renderer.chunks_svo
        else { continue; };
        if chunk_leaf.data.entity != Entity::PLACEHOLDER {
            continue;
        }

        commands.entity(entity).insert(VisibilityBundle::default());
        let root_chunk_entitiy = commands.spawn((
            ChunkComponent::new(CellPath::new()),
            TransformBundle::default(),
            VisibilityBundle::default(),
        )).set_parent(entity).id();
        renderer.chunks_svo = svo::LeafCell {
            data: ChunkSvoData { entity: root_chunk_entitiy }
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
            data: chunkdata, path: chunkpath,
        } in renderer.chunks_svo.iter() {
            let Ok(mut chunk) = chunks.get_mut(chunkdata.entity)
            else {
                log::warn!("Stored chunk entity does not exist");
                continue;
            };
            let aabb = chunkpath.get_aabb(renderer.options.root_aabb);

            let Some(closest_camera_dist_2) = relative_camera_poses.iter()
                .map(Vec3::as_dvec3)
                .map(|campos| aabb.closest_point(campos).distance_squared(campos))
                .min_by_key(|&d| OrderedFloat(d))
            else { continue };
            let closest_camera_dist = closest_camera_dist_2.sqrt();

            let subdiv_reduce =
                (closest_camera_dist / renderer.options.chunk_subdiv_half_life)
                .log2().floor() as u32;
            let mut total_subdivs = renderer.options.max_subdivs.saturating_sub(subdiv_reduce);

            if total_subdivs < renderer.options.min_subdivs {
                total_subdivs = renderer.options.min_subdivs;
            }
            
            let subdivs = total_subdivs.saturating_sub(chunkpath.len());
           
            if chunk.target_subdivs != subdivs {
                chunk.should_update_data = true;
                chunk.target_subdivs = subdivs;
            }

            if chunk.target_subdivs > renderer.options.chunk_split_subdivs {
                chunks_to_split.push(chunkpath);
            }
            if chunk.target_subdivs < renderer.options.chunk_merge_subdivs {
                // don't merge root
                if chunkpath.len() > 0 {
                    chunks_to_merge.push(chunkpath);
                }
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
    mut svo_renders: Query<(Entity, &mut SvoRendererComponent, &mut SvoProviderComponent)>,
) {
    for (renderer_entity, mut renderer, mut provider) in svo_renders.iter_mut() {
        let root_aabb = renderer.options.root_aabb;

        let chunks_to_split = std::mem::take(&mut renderer.chunks_to_split);
        let chunks_to_merge = std::mem::take(&mut renderer.chunks_to_merge);

        // Splitting chunks
        for chunkpath in chunks_to_split {
            log::debug!("split {chunkpath:?}");

            let mut on_new_chunk = renderer.options.on_new_chunk.take();
            let cell = renderer.chunks_svo.follow_path_mut(chunkpath).1;
            if let svo::Cell::Leaf(leaf) = cell {
                commands.entity(leaf.data.entity).despawn();
            }
            cell.split();

            for child in CellPath::components() {
                let child_path = chunkpath.with_push(child);
                let child_cell = match cell {
                    svo::Cell::Internal(i) => i,
                    _ => unreachable!("Just splitted and chunk svo should never have packed cells"),
                }.get_child_mut(child);

                let chunk_entitiy = commands.spawn((
                    ChunkComponent::new(child_path),
                    TransformBundle::default(),
                    VisibilityBundle::default(),
                    Into::<Aabb>::into(child_path.get_aabb(root_aabb)),
                )).set_parent(renderer_entity).id();
                child_cell.data_mut().entity = chunk_entitiy;
                if let Some(on_new_chunk) = &mut on_new_chunk {
                    on_new_chunk(commands.entity(chunk_entitiy));
                }
            }

            renderer.options.on_new_chunk = on_new_chunk;
        }

        // merging chunks
        'merges: for chunkpath in chunks_to_merge {
            let Some(new_chunk_path) = chunkpath.parent()
            else {
                log::warn!("root path have been set for merging");
                continue;
            };
            let mut children_entities = [Entity::PLACEHOLDER; 8];

            // If chunk isn't a leaf node at that position it may have already been
            // merged in previous iterations
            {
                let (foundpath, cell) = renderer.chunks_svo.follow_path(chunkpath);
                if foundpath != chunkpath || matches!(cell, svo::Cell::Leaf(_)) {
                    continue;
                }
            }

            // check if merging would mean immediately splitting ('overcrowded' chunk)
            for (i, child) in new_chunk_path.children().into_iter().enumerate() {
                let svo::Cell::Leaf(cleaf) =
                    renderer.chunks_svo.follow_path(child).1
                // not a leaf = either its children will be merged later
                // or merging would create an overcroweded chunk
                else { continue 'merges; };
                let Some(cchunk) = chunks.get(cleaf.data.entity).ok()
                // non existent = merge is probably fine ^^
                else { continue; };
                // is overcrowded
                if cchunk.target_subdivs+1 > renderer.options.chunk_split_subdivs {
                    continue 'merges;
                }

                children_entities[i] = cleaf.data.entity;
            }

            log::debug!("merge into {new_chunk_path:?}");

            for &nentity in &children_entities {
                let Some(mut c) = commands.get_entity(nentity)
                else { continue; };
                c.despawn();
            }

            let new_chunk_entity = commands.spawn((
                ChunkComponent::new(new_chunk_path),
                TransformBundle::default(),
                VisibilityBundle::default(),
                Into::<Aabb>::into(new_chunk_path.get_aabb(root_aabb)),
            )).set_parent(renderer_entity).id();
            *renderer.chunks_svo.follow_path_mut(new_chunk_path).1 = svo::LeafCell {
                data: ChunkSvoData { entity: new_chunk_entity, }
            }.into();
            if let Some(on_new_chunk) = &mut renderer.options.on_new_chunk {
                on_new_chunk(commands.entity(new_chunk_entity));
            }
        }

        let dirty = provider.drain_dirty_chunks();
        for &c in &*dirty {
            for chunkcell in renderer.chunks_svo.follow_path(c).1.iter() {
                let Ok(mut chunk) = chunks.get_mut(chunkcell.data.entity)
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

    mut chunks: Query<(Entity, &mut ChunkComponent, Option<&Handle<Mesh>>, &Parent)>,
    mut svo_renders: Query<(&mut SvoRendererComponent, &mut SvoProviderComponent)>,
) {
    let task_pool = AsyncComputeTaskPool::get();
    for (chunk_entitiy, mut chunk, mesh, parent) in chunks.iter_mut() {
        let Ok((renderer, mut provider)) = svo_renders.get_mut(parent.get())
        else { continue; };

        let actual_subdivs = renderer.options.chunk_split_subdivs
            .min(chunk.target_subdivs);
        // If the mesh is changed during this system this variable is updated
        // as the actual entitie's component isn't updated until next run
        let mut current_mesh = mesh.cloned();

        if chunk.should_update_data {
            chunk.should_update_data = false;

            chunk.data_task = Some(provider.request_chunk(
                chunk.path,
                actual_subdivs
            ));
            chunk.data_subdivs = actual_subdivs;
        }

        if let Some(task) = chunk.data_task
            .take_if(|task| task.is_finished())
        {
            chunk.data = Some(block_on(task));
            chunk.should_update_mesh = true;
        }

        if let Some(data) =
            chunk.should_update_mesh.then_some(&chunk.data).cloned().flatten()
        {
            chunk.should_update_mesh = false;
            chunk.mesh_subdivs = chunk.data_subdivs;

            let chunkpath = chunk.path;
            let root_aabb = renderer.options.root_aabb;
            let subdivs = actual_subdivs;
            chunk.mesh_task = Some(task_pool.spawn(async move {
                let mut out = marching_cubes::Out::new(true, false);
                log::trace!("Rendering mesh...");

                marching_cubes::run(
                    &mut out, chunkpath, &*data, root_aabb.into(), subdivs
                );

                if out.vertices.len() == 0 {
                    log::trace!("Empty mesh");
                    return None;
                }
                
                let m = out.into_mesh();
                log::trace!("Finished mesh");

                Some(m)
            }));
        }

        if let Some(task) = chunk.mesh_task
            .take_if(|task| task.is_finished())
        {
            if let Some(new_mesh) = block_on(task) {
                let new_mesh = meshes.add(new_mesh);
                commands.entity(chunk_entitiy).insert(new_mesh.clone());
                current_mesh = Some(new_mesh);
                
                chunk.should_update_collider = true;
            }
            else {
                commands.entity(chunk_entitiy).remove::<Handle<Mesh>>();
            }
        }

        if let Some(mesh_for_collider) = chunk.should_update_collider
            .then_some(current_mesh).flatten()
            .and_then(|handle| meshes.get(handle)).cloned()
        {
            chunk.collider_subdivs = chunk.mesh_subdivs;
            let subdivs = chunk.mesh_subdivs + chunk.path.len();
            let target = renderer.options.max_subdivs;
            chunk.collider_task = Some(task_pool.spawn(async move {
                if subdivs != target {
                    return None;
                }
                Collider::from_bevy_mesh(
                    &mesh_for_collider, &ComputedColliderShape::TriMesh
                )
            }));
        }

        if let Some(collider) = chunk.collider_task
            .take_if(|task| task.is_finished()) {
            if let Some(collider) = block_on(collider) {
                commands.entity(chunk_entitiy).insert(collider);
            }
            else {
                commands.entity(chunk_entitiy).remove::<Collider>();
            }
        }
    }
}
