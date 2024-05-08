use crate::*;
use bevy::prelude::*;

#[derive(Default)]
pub struct NBodyPlugin {
    /// Prevents public contruction
    _private: (),
}

impl Plugin for NBodyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, (
            sync_attractor_masses_with_colliders_system,
            compute_gravity_field_system,
            apply_gravity_to_attracted_rigid_bodies_system,
        ).chain().after(doprec::TransformSystems));

        app.init_resource::<GravityConfig>();
    }
}