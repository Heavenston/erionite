mod chunk_svo;
use std::sync::Arc;

use ordered_float::OrderedFloat;
use bevy::{ecs::system::EntityCommands, prelude::*, tasks::{block_on, AsyncComputeTaskPool, Task}};
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
            chunks_subdivs_system, chunk_system
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

    /// higher = more subdivs?
    pub chunk_falloff_multiplier: f64,

    pub root_aabb: DAabb,

    pub on_new_chunk: Option<Box<dyn FnMut(EntityCommands) -> () + Send + Sync>>,
}

#[derive(Component)]
pub struct SvoRendererComponent {
    pub options: SvoRendererComponentOptions,

    root_chunk: Entity,
}

impl SvoRendererComponent {
    pub fn new(options: SvoRendererComponentOptions) -> Self {
        Self {
            options,

            root_chunk: Entity::PLACEHOLDER,
        }
    }
}

#[derive(derivative::Derivative, Component)]
#[derivative(Default)]
pub struct ChunkComponent {
    path: svo::CellPath,
    target_subdivs: u32,

    #[derivative(Default(value="Entity::PLACEHOLDER"))]
    renderer: Entity,

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
    fn new(renderer: Entity, path: svo::CellPath) -> Self {
        Self {
            path,
            renderer,

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
    for (renderer_entity, mut renderer) in &mut svo_renders {
        commands.entity(renderer_entity).insert(VisibilityBundle::default());
        let root_chunk_entitiy = commands.spawn((
            ChunkComponent::new(renderer_entity, CellPath::new()),
            TransformBundle::default(),
            VisibilityBundle::default(),
        )).set_parent(renderer_entity).id();
        renderer.root_chunk = root_chunk_entitiy;
        if let Some(on_new_chunk) = &mut renderer.options.on_new_chunk {
            on_new_chunk(commands.entity(root_chunk_entitiy));
        }
    }
}

/// Updates chunks target_subdivs
fn chunks_subdivs_system(
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut chunks: Query<&mut ChunkComponent>,
    svo_renders: Query<(&SvoRendererComponent, &GlobalTransform)>,
) {
    let cameras_poses = cameras.iter()
        .filter(|(c, _)| c.is_active)
        .map(|(_, t)| t.translation())
        .collect::<Vec<_>>();

    for mut chunk in &mut chunks {
        let Ok((SvoRendererComponent { options, .. }, renderer_trans)) =
            svo_renders.get(chunk.renderer)
        else {
            log::warn!("Chunk without proper rendrere !?");
            continue;
        };

        let relative_camera_poses = cameras_poses.iter()
            .map(|&cp| renderer_trans.transform_point(cp))
            .collect::<Vec<_>>();
        let aabb = chunk.path.get_aabb(options.root_aabb);

        let Some(closest_camera_dist_2) = relative_camera_poses.iter()
            .map(Vec3::as_dvec3)
            .map(|campos| aabb.closest_point(campos).distance_squared(campos))
            .min_by_key(|&d| OrderedFloat(d))
        else { continue };
        let closest_camera_dist = closest_camera_dist_2.sqrt();

        let mut total_subdivs = 0u32;
        while total_subdivs < options.max_subdivs  {
            let mut width = options.root_aabb.size / 2f64.powi(total_subdivs as i32);
            width *= options.chunk_falloff_multiplier;
            if closest_camera_dist < width.max_element() {
                total_subdivs += 1;
            }
            else {
                break;
            }
        }

        if total_subdivs < options.min_subdivs {
            total_subdivs = options.min_subdivs;
        }
        
        let subdivs = total_subdivs.saturating_sub(chunk.path.len());
       
        if chunk.target_subdivs != subdivs {
            chunk.should_update_data = true;
            chunk.target_subdivs = subdivs;
        }
    }
}

/// Updates chunk datas, meshes etc.
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
