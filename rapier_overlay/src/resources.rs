use bevy::{math::DVec3, prelude::*, utils::HashMap};
use doprec::Transform64;

use crate::*;
use rapier::{
    dynamics::{CCDSolver, ImpulseJointSet, IslandManager, MultibodyJointSet, RigidBodyHandle, RigidBodySet},
    geometry::{BroadPhase, ColliderHandle, ColliderSet, NarrowPhase},
    pipeline::{PhysicsPipeline, QueryPipeline}
};

#[derive(Resource, Default)]
pub struct RapierContext {
    pub rigid_body_set: RigidBodySet,
    pub collider_set: ColliderSet,
    pub physics_pipeline: PhysicsPipeline,
    pub island_manager: IslandManager,
    pub broad_phase: BroadPhase,
    pub narrow_phase: NarrowPhase,
    pub impulse_joint_set: ImpulseJointSet,
    pub multibody_joint_set: MultibodyJointSet,
    pub ccd_solver: CCDSolver,
    pub query_pipeline: QueryPipeline,

    /// used for deletion as bevy forgets the component before we can read it
    pub entities2colliders: HashMap<Entity, ColliderHandle>,
    /// used for deletion as bevy forgets the component before we can read it
    pub entities2rigidbodies: HashMap<Entity, RigidBodyHandle>,

    pub entities_last_set_transform: HashMap<Entity, Transform64>,
}

impl RapierContext {
}

#[derive(Resource)]
pub struct RapierConfig {
    pub gravity: Vector3,
}

impl Default for RapierConfig {
    fn default() -> Self {
        Self {
            gravity: DVec3::new(0., -9.8, 0.),
        }
    }
}
