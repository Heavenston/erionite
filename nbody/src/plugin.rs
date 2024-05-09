use crate::*;
use bevy::{diagnostic::{Diagnostic, RegisterDiagnostic}, prelude::*};

#[derive(Default)]
pub struct NBodyPlugin {
    /// Prevents public contruction
    _private: (),
}

impl Plugin for NBodyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, (
            #[cfg(feature = "rapier")]
            sync_attractor_masses_with_colliders_system,
            compute_gravity_field_single_threaded_system,
            compute_gravity_field_parallel_system,
            #[cfg(feature = "rapier")]
            apply_gravity_to_attracted_rigid_bodies_system,
        ).chain().after(doprec::TransformSystems));

        app.register_diagnostic(
            Diagnostic::new(GRAVITY_COMPUTE_SYSTEM_DURATION)
                .with_suffix(" ms")
        );
 
        app.init_resource::<GravityConfig>();
    }
}
