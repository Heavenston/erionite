use bevy::prelude::*;
use crate::*;

#[derive(Default)]
pub struct RapierPlugin {
    // Prevents creation without using Default
    _private: (),
}

impl Plugin for RapierPlugin {
    fn build(&self, app: &mut App) {
        app
            .insert_resource(resources::RapierContext::default())
            .add_systems(PostStartup, (
                rigid_body_init_system,
                collider_init_system,
            ).chain())
            .add_systems(PostUpdate, (
                // update before init because there is no need to update a
                // collider that has just been created
                rigid_body_remove_system,
                rigid_body_update_system,
                rigid_body_init_system,

                collider_remove_system,
                collider_update_system,
                collider_init_system,
            ).chain())
            // .add_systems(FixedUpdate, (
                
            // ))
        ;
    }
}
