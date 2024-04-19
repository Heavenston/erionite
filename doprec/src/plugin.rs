use crate::systems;

use bevy::prelude::*;

#[derive(Default)]
pub struct DoprecPlugin {
    /// Prevents creation without using default
    _private: (),
}

impl Plugin for DoprecPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, (
            systems::propagate_transforms_system,
            systems::update_on_floating_origin_system,
        ).chain());
    }
}
