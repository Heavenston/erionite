use std::sync::Arc;
use std::time::Duration;

use bevy::time::common_conditions::on_timer;
use doprec::{GlobalTransform64, Transform64Bundle};
use ordered_float::OrderedFloat;
use bevy::{ecs::system::EntityCommands, prelude::*, render::primitives::Aabb};
use svo::{mesh_generation::marching_cubes, CellPath};
use utils::{AabbExt, DAabb};

use crate::task_runner::{self, OptionTaskExt, Task};
use crate::svo_provider::SvoProviderComponent;

// Shim for physics collider
type Collider = ();

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
            dirty_chunks_drainer_system
                .run_if(on_timer(Duration::from_millis(500))),
            chunks_subdivs_system,
            chunk_split_merge_system,
            chunk_system,
            provider_updates_system,
        ).chain());
    }
}

#[derive(Bundle)]
pub struct SvoRendererBundle {
    pub transform: Transform64Bundle,
    pub svo_render: SvoRendererComponent,
    pub svo_provider: SvoProviderComponent,
}

#[derive(derivative::Derivative)]
#[derivative(Default)]
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

    #[derivative(Default(value="true"))]
    pub enable_subdivs_update: bool,
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

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
enum ChunkMergeState {
    #[default]
    Merge,
    Split,

    ParentMerging,
}

impl ChunkMergeState {
    pub fn is_merge(self) -> bool {
        self == Self::Merge
    }

    pub fn is_split(self) -> bool {
        self == Self::Split
    }
}

#[derive(derivative::Derivative, Component, Debug)]
#[derivative(Default)]
pub struct ChunkComponent {
    path: svo::CellPath,
    target_subdivs: u32,
    target_state: ChunkMergeState,

    #[derivative(Default(value="Entity::PLACEHOLDER"))]
    renderer: Entity,

    /// The Children component is still populated as well but having it here
    /// allows for the chunk to have other chidren without getting confused
    /// which one are chunks
    ///
    /// Full describes wether the chunk is splitted (Some) or merged (None)
    chunk_children: Option<[Entity; 8]>,

    /// set to true on creation and set to false by the 
    /// subdivs system
    /// used to know if the chunk is waiting for a subdiv 'assignment'
    waiting_for_subdivs: bool,

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

            waiting_for_subdivs: true,

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

    pub fn is_busy(&self) -> bool {
        if self.waiting_for_subdivs {
            return true;
        }

        if self.target_state != ChunkMergeState::Merge {
            return false;
        }

        self.is_generating() ||
        self.is_generating_mesh() ||
        self.is_generating_collider() ||
        self.should_update_data ||
        self.should_update_mesh ||
        self.should_update_collider
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
            Transform64Bundle::default(),
            VisibilityBundle::default(),
        )).set_parent(renderer_entity).id();
        renderer.root_chunk = root_chunk_entitiy;
        if let Some(on_new_chunk) = &mut renderer.options.on_new_chunk {
            on_new_chunk(commands.entity(root_chunk_entitiy));
        }
    }
}

fn dirty_chunks_drainer_system(
    mut providers: Query<(Entity, &mut SvoProviderComponent), With<SvoRendererComponent>>,
    mut chunks: Query<&mut ChunkComponent>,
) {
    for (entity, mut provider) in &mut providers {
        let dirties = provider.drain_dirty_chunks();
        if dirties.len() == 0 {
            continue;
        }

        // FIXME: May be too slow if there are lots of chunks
        for mut chunk in chunks.iter_mut()
            .filter(|chunk| chunk.renderer == entity)
            .filter(|chunk| dirties.contains(&chunk.path))
        {
            chunk.should_update_data = true;
        }
    }
}

fn provider_updates_system(
    mut providers: Query<&mut SvoProviderComponent, With<SvoRendererComponent>>,
) {
    for mut provider in &mut providers {
        provider.update();
    }
}

/// Updates chunks target_subdivs
fn chunks_subdivs_system(
    cameras: Query<(&Camera, &GlobalTransform64)>,
    mut chunks: Query<&mut ChunkComponent>,
    svo_renders: Query<(&SvoRendererComponent, &GlobalTransform64)>,
) {
    let cameras_poses = cameras.iter()
        .filter(|(c, _)| c.is_active)
        .map(|(_, t)| t.translation())
        .collect::<Vec<_>>();

    for mut chunk in &mut chunks {
        let Ok((SvoRendererComponent { options, .. }, &renderer_trans)) =
            svo_renders.get(chunk.renderer)
        else {
            log::warn!("Chunk without proper rendrere !?");
            continue;
        };

        if !options.enable_subdivs_update {
            continue;
        }

        let chunk_aabb = chunk.path.get_aabb(options.root_aabb);
        let renderer_translation = renderer_trans.translation();

        let Some(closest_camera_dist_2) = cameras_poses.iter()
            .map(|&cp| cp - renderer_translation)
            .map(|campos| chunk_aabb.closest_point(campos).distance_squared(campos))
            .min_by_key(|&d| OrderedFloat(d))
        else { continue };
        let closest_camera_dist = closest_camera_dist_2.sqrt();

        let mut total_subdivs = options.max_subdivs;
        while total_subdivs > options.min_subdivs &&
            closest_camera_dist >
                (chunk_aabb.size /
                    2f64.powi(total_subdivs.saturating_sub(chunk.path.depth()) as i32)
                ).length() * options.chunk_falloff_multiplier
        {
            total_subdivs -= 1;
        }

        let subdivs = total_subdivs.saturating_sub(chunk.path.depth());
        if chunk.waiting_for_subdivs || chunk.target_subdivs != subdivs {
            chunk.waiting_for_subdivs = false;
            chunk.should_update_data = true;
            chunk.target_subdivs = subdivs;
        }

        if chunk.target_state == ChunkMergeState::Split &&
            chunk.target_subdivs < options.chunk_merge_subdivs {
            chunk.target_state = ChunkMergeState::Merge;
        }
        if chunk.target_state == ChunkMergeState::Merge &&
            chunk.target_subdivs > options.chunk_split_subdivs {
            chunk.target_state = ChunkMergeState::Split;
        }
    }
}

fn chunk_split_merge_system(
    mut commands: Commands,
    mut chunk_entities: Query<Entity, (With<ChunkComponent>, With<Visibility>)>,
    mut chunks: Query<&mut ChunkComponent>,
    mut chunk_complementaries: Query<Option<&Handle<Mesh>>>,
    mut svo_renders: Query<&mut SvoRendererComponent>,
) {
    'chunks_iter: for chunk_entity in &mut chunk_entities {
        let mut chunk = chunks.get_mut(chunk_entity).expect("Query is filtered");
        let mesh = chunk_complementaries
            .get_mut(chunk_entity).expect("Query is filtered");

        let Ok(mut renderer) =
            svo_renders.get_mut(chunk.renderer)
        else {
            log::warn!("Chunk without proper rendrere !?");
            continue 'chunks_iter;
        };
        let options = &mut renderer.options;

        // must split
        if chunk.chunk_children.is_none() && chunk.target_state.is_split() {
            let n_children = CellPath::components().map(|child| {
                let child_path = chunk.path.clone().with_push(child);

                let child_chunk_entitiy = commands.spawn((
                    ChunkComponent::new(chunk.renderer, child_path.clone()),
                    Transform64Bundle::default(),
                    VisibilityBundle::default(),
                    Into::<Aabb>::into(child_path.get_aabb(options.root_aabb)),
                )).set_parent(chunk_entity).id();

                if let Some(on_new_chunk) = &mut options.on_new_chunk {
                    on_new_chunk(commands.entity(child_chunk_entitiy));
                }

                child_chunk_entitiy
            });

            chunk.chunk_children = Some(n_children);

            // The chunk is in an semi-invalid state as the newly created children
            // will only exist when commands is executed so we stop here
            continue 'chunks_iter;
        }

        // weird trick to get access to both children and chunk
        let children = if let Some(children_entities) = chunk.chunk_children {
            let [chunk_, children_ @ ..] = chunks.get_many_mut::<9>(utils::join_arrays(
                [chunk_entity],
                children_entities,
            ).into()).expect("All children and chunks should exist");
            chunk = chunk_;
            Some(children_)
        }
        else {
            None
        };

        // Rest of the loop is only for chunks with children
        let Some(children_entities) = chunk.chunk_children
        else { continue 'chunks_iter; };
        let Some(mut children) = children
        else { continue 'chunks_iter; };

        if chunk.target_state.is_split() {
            for child in &mut children {
                if child.target_state == ChunkMergeState::ParentMerging {
                    child.target_state = ChunkMergeState::Merge;
                }
            }
            
            let can_hide = children.iter().all(|child| !child.is_busy());

            if mesh.is_some() && can_hide {
                chunk.mesh_subdivs = 0;
                chunk.collider_subdivs = 0;
                commands.entity(chunk_entity).remove::<(Collider, Handle<Mesh>)>();
            }
        }

        if chunk.target_state.is_merge() {
            let can_destroy_children = !chunk.is_busy();

            if can_destroy_children {
                for childe in children_entities {
                    commands.entity(childe).despawn_recursive();
                }

                chunk.chunk_children = None;
            }
            else {
                for child in &mut children {
                    child.target_state = ChunkMergeState::ParentMerging;
                }
            }
        }
    }
}

/// Updates chunk datas, meshes etc.
fn chunk_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,

    mut chunks: Query<(Entity, &mut ChunkComponent, Option<&Handle<Mesh>>)>,
    mut svo_renders: Query<(&mut SvoRendererComponent, &mut SvoProviderComponent)>,
) {
    for (chunk_entitiy, mut chunk, mesh) in chunks.iter_mut() {
        let Ok((renderer, mut provider)) = svo_renders.get_mut(chunk.renderer)
        else { continue; };

        let actual_subdivs = renderer.options.chunk_split_subdivs
            .min(chunk.target_subdivs);
        // If the mesh is changed during this system this variable is updated
        // as the actual entitie's component isn't updated until next run
        let mut current_mesh = mesh.cloned();

        if chunk.target_state.is_merge() && chunk.should_update_data {
            chunk.should_update_data = false;

            chunk.data_task = Some(provider.request_chunk(
                &chunk.path,
                actual_subdivs
            ));
            chunk.data_subdivs = actual_subdivs;
        }

        if let Some(data) = chunk.data_task.take_if_finished() {
            chunk.data = Some(data);
            chunk.should_update_mesh = true;
        }

        if let Some(data) = (chunk.target_state.is_merge() && chunk.should_update_mesh)
            .then_some(&chunk.data).cloned().flatten()
        {
            chunk.should_update_mesh = false;
            chunk.mesh_subdivs = chunk.data_subdivs;

            let chunkpath = chunk.path.clone();
            let root_aabb = renderer.options.root_aabb;
            let subdivs = actual_subdivs;
            chunk.mesh_task = Some(task_runner::spawn(move || {
                let mut out = marching_cubes::Out::new(true, false);
                marching_cubes::run(
                    &mut out, chunkpath, &*data, root_aabb.into(), subdivs
                );

                if out.vertices.len() == 0 {
                    return None;
                }
                
                let m = out.into_mesh();

                Some(m)
            }));
        }

        if let Some(maybe_new_mesh) = chunk.mesh_task.take_if_finished() {
            if let Some(new_mesh) = maybe_new_mesh {
                let new_mesh = meshes.add(new_mesh);
                commands.entity(chunk_entitiy).insert(new_mesh.clone());
                current_mesh = Some(new_mesh);
                
                chunk.should_update_collider = true;
            }
            else {
                commands.entity(chunk_entitiy).remove::<Handle<Mesh>>();
            }
        }

        if let Some(_mesh_for_collider) = (chunk.target_state.is_merge() && chunk.should_update_collider)
            .then_some(current_mesh).flatten()
            .and_then(|handle| meshes.get(handle)).cloned()
        {
            chunk.should_update_collider = false;
            chunk.collider_subdivs = chunk.data_subdivs;

            let subdivs = chunk.mesh_subdivs + chunk.path.len();
            let target = renderer.options.max_subdivs;
            chunk.collider_task = Some(task_runner::spawn(move || {
                if subdivs != target {
                    return None;
                }
                // Collider::from_bevy_mesh(
                //     &mesh_for_collider, &ComputedColliderShape::TriMesh
                // )
                Some(())
            }));
        }

        if let Some(maybe_collider) = chunk.collider_task.take_if_finished() {
            if let Some(collider) = maybe_collider {
                commands.entity(chunk_entitiy).insert(collider);
            }
            else {
                commands.entity(chunk_entitiy).remove::<Collider>();
            }
        }
    }
}
