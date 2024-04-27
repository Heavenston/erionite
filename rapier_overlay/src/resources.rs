use bevy::{prelude::*, utils::HashMap};

use crate::rapier;
use rapier::{
    dynamics::{CCDSolver, ImpulseJointSet, IntegrationParameters, IslandManager, MultibodyJointSet, RigidBodySet},
    geometry::{BroadPhase, ColliderHandle, ColliderSet, NarrowPhase},
    pipeline::{PhysicsPipeline, QueryPipeline}
};

#[derive(Resource, Default)]
pub struct RapierContext {
    pub integration_parameters: IntegrationParameters,
    
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

    pub entities2colliders: HashMap<Entity, ColliderHandle>,
}

impl RapierContext {
}
