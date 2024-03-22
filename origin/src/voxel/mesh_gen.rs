use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};

use crate::generator::{Generator, TryIntoGenerator};

use crate::singletones::GetSingletonEx;
use crate::svo::{TerrainCellKind, CellPath};
use crate::unsafe_send::UnsafeSend;
use crate::{svo, marching_cubes, every_cubes::every_cubes};

use cached::proc_macro::cached;
use godot::engine::character_body_3d::MotionMode;
use godot::engine::geometry_instance_3d::ShadowCastingSetting;
use godot::engine::global::Key;
use godot::engine::input::MouseMode;
use godot::engine::{
    mesh, ConcavePolygonShape3D, CollisionShape3D, SurfaceTool, NoiseTexture3D,
    FastNoiseLite, Material, Shape3D
};
use godot::prelude::*;
use godot::engine::{
    CharacterBody3D, ICharacterBody3D, InputEvent, InputEventMouseMotion,
    PhysicsServer3D, RigidBody3D, IRigidBody3D, CollisionPolygon3D,
    MeshInstance3D, Mesh, ArrayMesh
};
use itertools::Itertools as _;
use ordered_float::OrderedFloat;
use rand::prelude::*;
use arbitrary_int::*;
use noise::NoiseFn;
use rayon::prelude::*;

use super::VoxelProvider;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ChunkMeshGenSettings {
    pub subdivs: u32,
    pub collisions: bool,
}

#[derive(Debug, Default, Clone)]
pub struct ChunkSvoData {
    pub requesting: bool,
    pub last_request_subdivs: Option<u32>,
    pub force_remesh: bool,
    pub mesh_generating: bool,
    pub last_mesh_gen_settings: Option<ChunkMeshGenSettings>,

    pub mesh_instance: Option<Gd<MeshInstance3D>>,
    pub collision_shape: Option<Gd<CollisionShape3D>>,
}

impl svo::InternalData for ChunkSvoData {}

impl Drop for ChunkSvoData {
    fn drop(&mut self) {
        if let Some(mi) = self.mesh_instance.take() {
            if mi.is_instance_valid() {
                mi.free();
            }
        }
        if let Some(cs) = self.collision_shape.take() {
            if cs.is_instance_valid() {
                cs.free();
            }
        }
    }
}

impl ChunkSvoData {
    pub fn busy(&self) -> bool {
        self.mesh_generating | self.requesting
    }
}

impl svo::Data for ChunkSvoData {
    type Internal = Self;
}

impl svo::AggregateData for ChunkSvoData {
    fn aggregate<'a>(
        _children: [svo::EitherDataRef<Self>; 8]
    ) -> Self::Internal {
        Self::default()
    }
}

type TaskFn = Box<dyn FnOnce(&mut Voxel) -> () + Send>;

#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct Voxel {
    #[export]
    aabb: Aabb,

    #[export]
    /// What subdivs should the fullres use
    max_total_subdivs: u32,

    #[export]
    /// What subdivs should the complete low-res use
    lod_min_total_subdivs: u32,

    #[export]
    /// What subdivs a chunk should contain at maximum
    chunk_max_subdivs: u32,

    #[export]
    fullres_cam_distance: f64,

    #[export]
    fullres_cam_falloff: f64,

    #[export]
    material: Option<Gd<Material>>,

    #[export]
    provider: Option<Gd<VoxelProvider>>,

    local_svo: svo::TerrainCell,
    chunk_svo: svo::Cell<ChunkSvoData>,

    pending_chunk_updates: HashMap<svo::CellPath, u32>,
    running_tasks: Arc<Mutex<Vec<TaskFn>>>,

    /// Cache for expected_additional_subdivs
    ex_cache: RefCell<HashMap<([OrderedFloat<f64>; 3], svo::CellPath), u32>>,

    camera_pos: Vector3,

    #[base]
    base: Base<Node3D>,
}

#[godot_api]
impl Voxel {
    fn schedule_chunk_update(&mut self, path: svo::CellPath) {
        self.pending_chunk_updates.entry(path).or_insert(0);
    }

    // Very hot function, maybe caching is worth it
    fn expected_additional_subdivs(&self, path: svo::CellPath) -> u32 {
        let cp = [
            self.camera_pos.x.into(),
            self.camera_pos.y.into(),
            self.camera_pos.z.into(),
        ];

        if let Some(x) = self.ex_cache.borrow().get(&(cp, path)) {
            return *x;
        }

        // Player "view" sphere to chunk aabb collision
        let aabb = path.get_aabb(self.aabb);
        let closest_point = Vector3::new(
            self.camera_pos.x.clamp(aabb.position.x, aabb.end().x),
            self.camera_pos.y.clamp(aabb.position.y, aabb.end().y),
            self.camera_pos.z.clamp(aabb.position.z, aabb.end().z),
        );
        let distance = (self.camera_pos - closest_point).length();
        // Kinda arbitrary calculation for amount of subdivs required
        // the idea is, fullres_cam_falloff times further = 1 more subdiv
        let subdiv_sub =
            (distance.max(self.fullres_cam_distance) / self.fullres_cam_distance)
            .log(self.fullres_cam_falloff).ceil() as u32;
        let expected_subdiv = (self.max_total_subdivs.saturating_sub(subdiv_sub))
            .max(self.lod_min_total_subdivs);
        
        let v = expected_subdiv.saturating_sub(path.len());
        self.ex_cache.borrow_mut().insert((cp, path), v);
        v
    }

    fn expected_chunk_mesh_settings(
        &self, path: svo::CellPath
    ) -> ChunkMeshGenSettings {
        ChunkMeshGenSettings {
            subdivs: self.expected_additional_subdivs(path),
            collisions: true,
        }
    }

    fn split_chunk(&mut self, path: svo::CellPath) {
        let (_, chunk) = self.chunk_svo.follow_path_mut(path);
        if chunk.is_inner() {
            godot_error!("could not split an internal cell");
        }

        let previous_data = std::mem::take(&mut chunk.as_leaf_mut().data);
        chunk.split();
        chunk.as_inner_mut().data = previous_data;

        for child in path.children() {
            self.schedule_chunk_update(child);
        }
    }

    fn merge_chunk(&mut self, merging_path: svo::CellPath) {
        let (_, merging_chunk) = self.chunk_svo.follow_path_mut(merging_path);

        *merging_chunk = svo::LeafCell::new(
            std::mem::take(&mut merging_chunk.as_inner_mut().data)
        ).into();

        self.schedule_chunk_update(merging_path);
    }

    /// Called when the terrain changed in a chunk
    /// updates all neighbors and queue them to update
    fn chunk_updated(&mut self, chunk_path: svo::CellPath) {
        self.chunk_svo.follow_path_mut(chunk_path).
            1.as_inner_mut().data.force_remesh = true;
        self.schedule_chunk_update(chunk_path);

        for (_, neighbor) in chunk_path.neighbors() {
            let neighbor_cell = self.chunk_svo.follow_path_mut(neighbor).1;

            let mut to_update = vec![];
            
            // Neighbors could be more devided so we need to find its children,
            // the actual neighbor chunks
            let children = neighbor_cell.iter().map(|x| x.path).collect_vec();
            for child_path in children {
                let neighbor_chunk = neighbor_cell.follow_path_mut(child_path).1;
                neighbor_chunk.as_leaf_mut().data.force_remesh = true;
                to_update.push(neighbor.extended(child_path));
            }

            for t in to_update {
                self.schedule_chunk_update(t);
            }
        }
    }

    fn start_requesting_chunk(
        &mut self,
        chunk_path: svo::CellPath,
        subdivs: u32,
    ) {
        self.chunk_svo.follow_path_mut(chunk_path).1.as_leaf_mut()
            .data.requesting = true;

        let mut provider = self.provider.clone().unwrap();

        provider.bind_mut().set_subscription_rpc(
            chunk_path,
            subdivs,
            true
        );
    }

    #[func]
    /// Event handler for the svo_update event of [VoxelProvider]
    fn on_svo_update(
        &mut self,
        svo_path: svo::CellPath, svo: svo::TerrainCell, subdivs: u32
    ) {
        let (chunk_path, chunk) = self.chunk_svo.follow_path_mut(svo_path);
        let Some(chunk) = chunk.try_leaf_mut()
        else {
            godot_error!(
                "received chunk data on chunk {chunk_path:?},
                 but chunk not valid anymore"
            );
            return;
        };

        // subdivs of the received svo from the perspective of the chunk
        let chunk_subdivs = subdivs + svo_path.depth() - chunk_path.depth();

        if chunk.data.last_request_subdivs.unwrap_or(0) > chunk_subdivs {
            godot_warn!("received subdivs lower than requested (ignored update)");
            return;
        }

        // could already be false for spontanious updates
        chunk.data.requesting = false;

        *self.local_svo.follow_path_mut(svo_path).1 = svo;
        self.local_svo.updated_child(svo_path);

        self.chunk_updated(chunk_path);
    }

    fn start_generate_chunk_mesh(
        &mut self,
        chunk_path: svo::CellPath,
        settings: ChunkMeshGenSettings,
    ) {
        let rt = Arc::clone(&self.running_tasks);

        let mesh_instance;
        let collision_shape;
        {
            let chunk = self.chunk_svo.follow_path_mut(chunk_path)
                .1.as_leaf_mut();
            chunk.data.mesh_generating = true;
            chunk.data.force_remesh = false;
            mesh_instance = chunk.data.mesh_instance.clone();
            collision_shape = chunk.data.collision_shape.clone();
        }

        let root_aabb = self.aabb;
        // cheap as svo::Cell is copy-on-write
        let root_svo = self.local_svo.clone();
        // let start = Instant::now();

        let material = UnsafeSend::new(self.material.clone());
        let mesh_instance = UnsafeSend::new(mesh_instance);
        let collision_shape = UnsafeSend::new(collision_shape);

        rayon::spawn(move || {
            // godot_print!("[START] Generating chunk mesh {chunk_path:?} -> {settings:?}");
            let material = unsafe { material.into_inner() };
            let mut mesh_instance = unsafe { mesh_instance.into_inner() };
            let mut collision_shape = unsafe { collision_shape.into_inner() };

            let mut out = marching_cubes::Out::default();
            marching_cubes::run(
                &mut out,
                chunk_path, &root_svo,
                root_aabb, settings.subdivs
            );

            if out.vertices.len() == 0 {
                if let Some(mi) = mesh_instance.as_mut() {
                    mi.call_thread_safe(
                        "hide".into(),
                        &[]
                    );
                }
                if let Some(cs) = collision_shape.as_mut() {
                    cs.call_thread_safe(
                        "set_disabled".into(),
                        &[Variant::from(true)]
                    );
                }
                rt.lock().unwrap().push(Box::new(move |this: &mut Self| {
                    let chunk = this.chunk_svo.follow_path_mut(chunk_path).1;
                    let Some(chunk) = chunk.try_leaf_mut()
                    else {
                        godot_error!("[ERROR] Generating chunk mesh {chunk_path:?}, chunk not valid anymore");
                        return;
                    };
                    chunk.data.mesh_generating = false;
                    chunk.data.last_mesh_gen_settings = Some(settings);
                }));
                return;
            }

            let mesh = {
                let mut mesh = ArrayMesh::new();
                mesh.add_surface_from_arrays(
                    mesh::PrimitiveType::PRIMITIVE_TRIANGLES, out.to_array()
                );
                if let Some(mat) = material {
                    mesh.surface_set_material(0, mat);
                }
                mesh
            };
            let collision = settings.collisions
                .then(|| mesh.create_trimesh_shape()).flatten();

            let mut mesh_instance = mesh_instance
                .unwrap_or_else(MeshInstance3D::new_alloc);
            mesh_instance.call_thread_safe(
                "show".into(),
                &[]
            );
            mesh_instance.call_thread_safe(
                "set_mesh".into(),
                &[Variant::from(mesh.upcast::<Mesh>())]
            );

            let mut collision_shape = collision_shape
                .unwrap_or_else(CollisionShape3D::new_alloc);
            if let Some(shape) = collision {
                collision_shape.call_thread_safe(
                    "set_shape".into(),
                    &[Variant::from(shape.upcast::<Shape3D>())],
                );
                collision_shape.call_thread_safe(
                    "set_disabled".into(),
                    &[Variant::from(false)],
                );
            }
            else {
                collision_shape.call_thread_safe(
                    "set_disabled".into(),
                    &[Variant::from(true)],
                );
            }

            let mesh_instance = UnsafeSend::new(mesh_instance);
            let collision_shape = UnsafeSend::new(collision_shape);

            // let vertex_count = out.vertices.len();
            rt.lock().unwrap().push(Box::new(move |this: &mut Self| {
                let mesh_instance = unsafe { mesh_instance.into_inner() };
                let collision_shape = unsafe { collision_shape.into_inner() };

                let chunk = this.chunk_svo.follow_path_mut(chunk_path).1;
                let Some(chunk) = chunk.try_leaf_mut()
                else {
                    godot_error!("[ERROR] Generating chunk mesh {chunk_path:?}, chunk not valid anymore");
                    return;
                };
                chunk.data.mesh_generating = false;
                chunk.data.last_mesh_gen_settings = Some(settings);

                if !mesh_instance.is_inside_tree() {
                    this.base.add_sibling(mesh_instance.clone().upcast());
                }
                chunk.data.mesh_instance = Some(mesh_instance);

                if !collision_shape.is_inside_tree() {
                    this.base.add_sibling(collision_shape.clone().upcast());
                }
                chunk.data.collision_shape = Some(collision_shape);

                // update parent to check if it can be hidden
                for parent in chunk_path.parents() {
                    this.schedule_chunk_update(parent);
                }

                // let took = start.elapsed();
                // godot_print!(
                //     "[FINISH] Generated chunk mesh {chunk_path:?}, took {took:?}, generated {vertex_count} vertices",
                // );
            }) as TaskFn);
        });
    }

    fn update_chunk(&mut self, path: svo::CellPath) {
        let (found_path, chunk) = self.chunk_svo.follow_path_mut(path);
        if found_path != path {
            // Chunk isn't valid anymore, not an error and can happen very often
            return;
        }

        if chunk.is_inner() {
            // let internal = chunk.as_inner_mut();
            let can_hide = chunk.iter()
                .all(|x| x.cell.data.last_mesh_gen_settings.is_some());
            if can_hide {
                if let Some(x) = 
                    chunk.data_mut().into_inner().mesh_instance.as_mut()
                {
                    x.hide();
                }
                if let Some(x) = 
                    chunk.data_mut().into_inner().collision_shape.as_mut()
                {
                    x.set_disabled(true);
                }
            }
            return;
        }

        let expected_subdivs = self.expected_additional_subdivs(path);
        let expected_settings = self.expected_chunk_mesh_settings(path);

        let can_merge_parent = if let Some(parent) = path.parent() {
            self.expected_additional_subdivs(parent) == 0 &&
            parent.children().into_iter().all(|child|
                self.chunk_svo.follow_path(child).1.try_leaf()
                    .is_some_and(|leaf| !leaf.data.busy())
            )
        }
        else {
            false
        };

        let (_, chunk) = self.chunk_svo.follow_path_mut(path);
        let leaf = chunk.as_leaf_mut();

        let must_split = can_merge_parent
            || expected_subdivs > self.chunk_max_subdivs;

        let must_update = must_split
            || leaf.data.last_request_subdivs
                .map(|subdivs| subdivs < expected_subdivs)
                .unwrap_or(true);

        let must_update_mesh = must_update
            || leaf.data.force_remesh
            || leaf.data.last_mesh_gen_settings.as_ref()
                .map(|x| x != &expected_settings)
                .unwrap_or(true);

        if must_update_mesh && leaf.data.busy() {
            self.schedule_chunk_update(path);
            return;
        }

        if can_merge_parent {
            self.merge_chunk(path.parent().unwrap());
        }
        else if must_split {
            self.split_chunk(path);
        }
        else if must_update {
            self.start_requesting_chunk(path, expected_subdivs);
        }
        else if must_update_mesh {
            self.start_generate_chunk_mesh(path, expected_settings);
        }
    }

    fn on_camera_move(&mut self, pos: Vector3) {
        let local_pos = self.base.get_global_transform().affine_inverse() * pos;

        if (self.camera_pos - local_pos).length() < 1. {
            return;
        }

        *self.ex_cache.borrow_mut() = HashMap::new();
        self.camera_pos = local_pos;
        
        for path in self.chunk_svo.into_iter().map(|x| x.path).collect_vec() {
            self.schedule_chunk_update(path);
            for p in path.parents() {
                self.schedule_chunk_update(p);
            }
        }
    }

    fn get_and_divide_at(&mut self, local_pos: Vector3) -> Option<CellPath> {
        loop {
            let Some((path, cell)) = self.local_svo.sample_mut(
                (local_pos - self.aabb.position) / self.aabb.size, u32::MAX
            ) else { return None; };

            if path.len() < self.max_total_subdivs {
                cell.split();
                continue;
            }

            return Some(path);
        }
    }

    fn get_path(&mut self, path: CellPath) -> f64 {
        self.local_svo.follow_path(path).1.data().distance
    }

    fn set_path(&mut self, path: CellPath, val: f64) {
        let (_, cell) = self.local_svo.follow_path_mut(path);

        let leaf = cell.as_leaf_mut();
        leaf.data.distance = val;
        if leaf.data.distance > 0. && leaf.data.kind != TerrainCellKind::Air {
            leaf.data.kind = TerrainCellKind::Air;
        }
        if leaf.data.distance < 0. && leaf.data.kind == TerrainCellKind::Air {
            leaf.data.kind = TerrainCellKind::Stone;
        }
        let (chunk_path, chunk) = self.chunk_svo.follow_path_mut(path);
        chunk.data_mut().into_inner().force_remesh = true;

        for n in chunk_path.neighbors().map(|x| x.1).chain([chunk_path]) {
            self.schedule_chunk_update(n);
            // without this chunks aren't all remeshed, not sure why rn
            for p in n.parents() {
                self.schedule_chunk_update(p);
            }
        }
    }

    pub fn remove_sphere(&mut self, pos: Vector3, radius: f64) {
        let local_pos = self.base.get_global_transform().affine_inverse() * pos;
        
        let effect_aabb = Aabb {
            position: local_pos - Vector3::ONE * radius * 2.,
            size: Vector3::ONE * radius * 4.,
        };
        let cube_size = self.aabb.size / 2f64.powi(self.max_total_subdivs as i32);

        for cube in every_cubes(effect_aabb, cube_size) {
            let Some(path) = self.get_and_divide_at(cube)
            else { continue };

            let amount = radius - (cube - local_pos).length();
            let val = self.get_path(path);
            self.set_path(path, val.max(amount));
        }
    }
}

#[godot_api]
impl INode3D for Voxel {
    fn init(base: Base<Node3D>) -> Self {
        use svo::TerrainCellKind::*;

        Self {
            aabb: Aabb {
                position: Vector3::new(-5., -5., -5.),
                size: Vector3::new(10., 10., 10.),
            },

            max_total_subdivs: 6,
            lod_min_total_subdivs: 3,
            chunk_max_subdivs: 3,
            fullres_cam_distance: 10.,
            fullres_cam_falloff: 1.25,

            chunk_svo: svo::Cell::default(),
            local_svo: svo::TerrainCellKind::default().into(),

            running_tasks: Default::default(),
            pending_chunk_updates: Default::default(),

            material: None,
            provider: None,

            ex_cache: Default::default(),

            camera_pos: Vector3::ZERO,

            base,
        }
    }

    fn ready(&mut self) {
        self.chunk_svo = svo::LeafCell::new(ChunkSvoData::default()).into();
        self.schedule_chunk_update(svo::CellPath::new());

        assert!(
            self.provider.is_none()
            || self.provider.as_ref().unwrap().is_instance_valid()
        );
    }

    fn process(&mut self, _delta: f64) {
        if let Some(vp) = self.base.get_viewport() {
            if let Some(cam) = vp.get_camera_3d() {
                self.on_camera_move(cam.get_global_position());
            }
        }

        {
            // let start = Instant::now();
            let mut old = vec![];
            if let Ok(mut l) = self.running_tasks.try_lock() {
                std::mem::swap(&mut old, &mut l);
            }
            if old.len() > 0 {
                old.into_iter().for_each(|f| { f(self); });
                // godot_print!("[tasks] {:?}", start.elapsed());
            }
        }

        {
            let start = Instant::now();
            // let mut i = 0;
            while start.elapsed() < Duration::MILLISECOND * 15 {
                // i += 1;
                let Some(&t) = self.pending_chunk_updates.keys().next()
                else { break };
                self.pending_chunk_updates.remove(&t);
                self.update_chunk(t);
            }
            // if i > 0 {
            //     godot_print!("[updates][{i}] {:?} ({:?})", start.elapsed(), start.elapsed() / i);
            // }
        }

        if Input::singleton().is_action_just_pressed("refresh".into()) {
            let mut todo = vec![];
            for x in self.chunk_svo.iter() {
                todo.push(x.path);
            }
            for c in todo {
                let chunk = self.chunk_svo.follow_path_mut(c).1;
                chunk.as_leaf_mut().data.force_remesh = true;
                self.schedule_chunk_update(c);
                for p in c.parents() {
                        self.schedule_chunk_update(p);
                }
            }
            godot_print!(
                "[REFRESH] Now {} chunks scheduled for rechunk",
                self.pending_chunk_updates.len()
            );
        }

        // println!("mesh still generating: {}", self.chunk_mesh_generating.len());
        // println!("still generating: {}", self.chunk_generating.len());
    }
}
