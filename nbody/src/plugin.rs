use crate::*;
use bevy::{diagnostic::{Diagnostic, RegisterDiagnostic}, prelude::*};

#[derive(Default)]
pub struct NBodyPlugin {
    /// Prevents public contruction
    _private: (),
}

impl Plugin for NBodyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, (
            #[cfg(feature = "rapier")]
            sync_attractor_masses_with_colliders_system,
            compute_gravity_field_system,
            #[cfg(feature = "rapier")]
            apply_gravity_to_attracted_rigid_bodies_system,
        ).chain().in_set(GravitySystems));

        app.register_diagnostic(
            Diagnostic::new(GRAVITY_COMPUTE_SYSTEM_DURATION)
                .with_suffix(" ms")
        );
 
        app.init_resource::<GravityConfig>();
    }
}
