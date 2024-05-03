use bevy::{math::DVec3, prelude::*, utils::HashMap};
use doprec::GlobalTransform64;

use crate::*;
use rapier::{
    dynamics::{CCDSolver, ImpulseJointSet, IslandManager, MultibodyJointSet, RigidBodyHandle, RigidBodySet},
    geometry::{BroadPhase, Collider, ColliderHandle, ColliderSet, InteractionGroups, NarrowPhase, Ray},
    pipeline::{PhysicsPipeline, QueryFilter as RapierQFilter, QueryFilterFlags, QueryPipeline}
};

/// See [rapier::pipeline::QueryFilter]
#[derive(Copy, Clone, Default)]
pub struct QueryFilter<'a> {
    pub flags: QueryFilterFlags,
    pub groups: Option<InteractionGroups>,
    pub exclude_collider: Option<ColliderHandle>,
    pub exclude_rigid_body: Option<RigidBodyHandle>,
    pub predicate: Option<&'a dyn Fn(Entity, &Collider) -> bool>,
}

#[derive(Resource, Default)]
pub struct RapierContext {
    // Note: If needed outside the crate a util wrapper function should be
    //       created instead
    pub(crate) rigid_body_set: RigidBodySet,
    pub(crate) collider_set: ColliderSet,
    pub(crate) physics_pipeline: PhysicsPipeline,
    pub(crate) island_manager: IslandManager,
    pub(crate) broad_phase: BroadPhase,
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

        let mapped_predicate = filter.predicate.map(|pred| {
            move |handle: ColliderHandle, collider: &Collider| -> bool {
                let Some(&entity) = self.entities2colliders.get_by_right(&handle)
                else {
                    log::warn!("Collider has no registered entity");
                    return false;
                };

                pred(entity, collider)
            }
        });

        let (handle, dist) = self.query_pipeline.cast_ray(
            &self.rigid_body_set,
            &self.collider_set,
            &ray,
            max_toi,
            solid,
            RapierQFilter {
                flags: filter.flags,
                groups: filter.groups,
                exclude_collider: filter.exclude_collider,
                exclude_rigid_body: filter.exclude_rigid_body,
                predicate: mapped_predicate.as_ref().map(|f| f as _),
            },
        )?;
        let Some(&entity) = self.entities2colliders.get_by_right(&handle)
        else {
            log::warn!("Collider has no registered entity");
            return None;
        };

        Some((entity, dist))
    }
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
