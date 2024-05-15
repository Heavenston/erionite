use crate::systems;

use bevy::{diagnostic::{Diagnostic, RegisterDiagnostic}, prelude::*};

#[derive(Default)]
pub struct DoprecPlugin {
    /// Prevents creation without using default
    _private: (),
}

impl Plugin for DoprecPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<systems::PropagStart>()
            .register_diagnostic(
                Diagnostic::new(crate::TRANSFORM_SYSTEMS_DURATION_DIAG)
                    .with_suffix(" ms")
            )

            .add_systems(PostStartup, (
                systems::propagate_transforms_system,

                (
                    systems::propagate_start_end_system,
                    systems::propagate_transforms64_system,
                    systems::update_on_floating_origin_system,
                    systems::propagate_start_end_system,
                ).chain(),
            ).in_set(systems::TransformSystems))
            .add_systems(PostUpdate, (
                systems::propagate_transforms_system,

                (
                    systems::propagate_start_end_system,
                    systems::propagate_transforms64_system,
                    systems::update_on_floating_origin_system,
                    systems::propagate_start_end_system,
                ).chain(),
            ).in_set(systems::TransformSystems));
    }
}
