use bevy::{prelude::*, utils::HashMap};
use doprec::GlobalTransform64;

use crate::*;
use rapier::{
    dynamics::{CCDSolver, ImpulseJointSet, IslandManager, MultibodyJointSet, RigidBodyHandle, RigidBodySet},
    geometry::{BroadPhaseMultiSap, Collider, ColliderHandle, ColliderSet, NarrowPhase, Ray},
    pipeline::{PhysicsPipeline, QueryFilter as RapierQFilter, QueryPipeline}
};

#[derive(Resource, Default)]
pub struct RapierContext {
    // Note: If needed outside the crate a util wrapper function should be
    //       created instead
    pub(crate) rigid_body_set: RigidBodySet,
    pub(crate) collider_set: ColliderSet,
    pub(crate) physics_pipeline: PhysicsPipeline,
    pub(crate) island_manager: IslandManager,
    pub(crate) broad_phase: BroadPhaseMultiSap,
    pub(crate) narrow_phase: NarrowPhase,
    pub(crate) impulse_joint_set: ImpulseJointSet,
    pub(crate) multibody_joint_set: MultibodyJointSet,
    pub(crate) ccd_solver: CCDSolver,
    pub(crate) query_pipeline: QueryPipeline,

    /// used for deletion as bevy forgets the component before we can read it
    pub(crate) entities2colliders: utils::BiHashMap<Entity, ColliderHandle>,
    /// used for deletion as bevy forgets the component before we can read it
    pub(crate) entities2rigidbodies: utils::BiHashMap<Entity, RigidBodyHandle>,

    pub(crate) entities_last_set_transform: HashMap<Entity, GlobalTransform64>,
}

impl RapierContext {
    /// See [QueryPipeline::cast_ray]
    pub fn cast_ray(
        &self,
        origin: Vector3,
        direction: Vector3,
        max_toi: Float,
        solid: bool,
        filter: QueryFilter,
    ) -> Option<(Entity, Float)> {
        let ray = Ray {
            origin: origin.to_rapier().into(),
            dir: direction.to_rapier(),
        };

        to_rapier_query!(rapier_filter = filter, self);

        let (handle, dist) = self.query_pipeline.cast_ray(
            &self.rigid_body_set,
            &self.collider_set,
            &ray,
            max_toi,
            solid,
            rapier_filter,
        )?;
        let Some(&entity) = self.entities2colliders.get_by_right(&handle)
        else {
            log::warn!("Collider has no registered entity");
            return None;
        };

        Some((entity, dist))
    }
}
