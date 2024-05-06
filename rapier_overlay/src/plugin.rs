use bevy::prelude::*;
use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub struct PhysicsStepSystems;

#[derive(Default)]
pub struct RapierPlugin {
    // Prevents creation without using Default
    _private: (),
}

impl Plugin for RapierPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(RapierContext::default())
            .add_systems(PostStartup, (
                rigid_body_init_system,
                collider_init_system,
            ).chain().after(doprec::TransformSystems))
            .add_systems(PostUpdate, (
                // update before init because there is no need to update a
                // collider that has just been created
                rigid_body_remove_system,
                rigid_body_update_system,
                rigid_body_init_system,

                collider_remove_system,
                collider_update_system,
                collider_init_system,
            ).chain().after(doprec::TransformSystems))
            .add_systems(FixedUpdate, (
                characher_controllers_physics_step_system,
                physics_step_system,
                physics_rapier2bevy_sync_system,
            ).in_set(PhysicsStepSystems).chain())
        ;
    }
}
