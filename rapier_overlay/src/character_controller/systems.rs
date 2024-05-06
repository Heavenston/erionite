use bevy::prelude::*;
use crate::*;

use rapier::control::KinematicCharacterController;

pub fn characher_controllers_physics_step_system(
    time: Res<Time<Fixed>>,
    mut context: ResMut<RapierContext>,

    mut characters: Query<(
        &CharacterControllerComp, &CharacterNextTranslationComp,
        &mut CharacterResultsComp,

        &ColliderHandleComp,
        Option<&RigidBodyHandleComp>,
    )>,
) {
    let dt = time.delta_seconds_f64();

    let RapierContext {
        rigid_body_set, collider_set, query_pipeline, ..
    } = &mut *context;

    for (
        controller, next_translation,
        mut results,

        collider_handle_comp,
        rigid_body_comp,
    ) in &mut characters {
        let rapier_controller: KinematicCharacterController = controller.into();

        let Some(collider) = collider_set.get(collider_handle_comp.handle())
        else { continue; };
        let mut collisions = vec![];

        let moved = rapier_controller.move_shape(
            dt,
            &rigid_body_set,
            &collider_set,
            &query_pipeline,
            collider.shape(),
            collider.position(),
            next_translation.next_translation.to_rapier(),
            default(),
            |c| {
                collisions.push(c);
            },
        );

        for collision in &collisions {
            rapier_controller.solve_character_collision_impulses(
                dt,
                rigid_body_set,
                collider_set,
                query_pipeline,
                collider.shape(),
                collider.mass(),
                collision,
                default(),
            );
        }

        results.on_ground = moved.grounded;
        results.is_sliding = moved.is_sliding_down_slope;

        if let Some(rb) = rigid_body_comp
            .and_then(|rb| rigid_body_set.get_mut(rb.handle))
        {
            use rapier::dynamics::RigidBodyType as Kind;
            match rb.body_type() {
                Kind::KinematicPositionBased => {
                    let new_translation = rb.translation() + moved.translation;
                    rb.set_next_kinematic_translation(new_translation);
                },
                Kind::KinematicVelocityBased => {
                    rb.set_linvel(moved.translation / dt, false);
                },
                Kind::Dynamic | Kind::Fixed => {
                    log::warn!("Unsupported rigid body type for kinematic character controller: {:?}", rb.body_type());
                },
            }
        }
        else {
            let collider = collider_set.get_mut(collider_handle_comp.handle())
                .expect("checked before");
            let new_translation = collider.translation() + moved.translation;
            collider.set_translation(new_translation);
        }
    }
}
