use bevy::{math::DVec3, prelude::*};
use doprec::Transform64;
use rapier::dynamics::IntegrationParameters;

use crate::*;

pub fn physics_bevy2rapier_sync_system(
    mut context: ResMut<RapierContext>,

    rigid_bodies_query: Query<(
        &RigidBodyHandleComp,
        &Transform64,
    ), Changed<Transform64>>,
) {
    for (handle_comp, transform_comp) in rigid_bodies_query.iter() {
        let Some(rigid_body) = context.rigid_body_set.get_mut(handle_comp.handle())
        else { continue; };

        // let mut new_transform = *transform_comp;
        // new_transform.translation = rigid_body.translation().to_bevy();
        // new_transform.rotation = rigid_body.rotation().to_bevy();
        // if new_transform != *transform_comp {
        //     *transform_comp = new_transform;
        // }
        let tt = transform_comp.translation.to_rapier();
        if rigid_body.translation() != &tt {
            rigid_body.set_translation(tt, true);
        }
        let rr = transform_comp.rotation.to_rapier();
        if rigid_body.rotation() != &rr {
            rigid_body.set_rotation(rr, true);
        }
    }
}

pub fn physics_step_system(
    time: Res<Time<Fixed>>,
    mut context: ResMut<RapierContext>,
) {
    let params = IntegrationParameters {
        dt: time.delta_seconds_f64(),
        ..default()
    };

    let RapierContext {
        rigid_body_set, collider_set, physics_pipeline, island_manager,
        broad_phase, narrow_phase, impulse_joint_set, multibody_joint_set,
        ccd_solver, query_pipeline, ..
    } = &mut *context;

    physics_pipeline.step(
        &DVec3::new(0., -9.81, 0.).to_rapier(),
        &params,
        island_manager,
        broad_phase,
        narrow_phase,
        rigid_body_set,
        collider_set,
        impulse_joint_set,
        multibody_joint_set,
        ccd_solver,
        Some(query_pipeline),
        &(),
        &(),
    );
}

pub fn physics_rapier2bevy_sync_system(
    context: ResMut<RapierContext>,

    mut rigid_bodies_query: Query<(
        &RigidBodyHandleComp,
        &mut RigidBodySleepingComp,
        &mut VelocityComp,
        &mut AngularVelocityComp,
        &mut Transform64,
    )>,
) {
    for (handle_comp, mut sleeping_comp, mut linvel_comp, mut angvel_comp, mut transform_comp) in rigid_bodies_query.iter_mut() {
        let Some(rigid_body) = context.rigid_body_set.get(handle_comp.handle())
        else { continue; };

        if sleeping_comp.sleeping != rigid_body.is_sleeping() {
            sleeping_comp.sleeping = rigid_body.is_sleeping();
        }

        let mut new_transform = *transform_comp;
        new_transform.rotation = rigid_body.rotation().to_bevy();
        new_transform.translation = rigid_body.translation().to_bevy();
        if new_transform != *transform_comp {
            *transform_comp = new_transform;
        }

        let new_linvel = rigid_body.linvel().to_bevy();
        if new_linvel != linvel_comp.linvel {
            linvel_comp.linvel = new_linvel;
        }
        let new_angvel = rigid_body.angvel().to_bevy();
        if new_angvel != angvel_comp.angvel {
            angvel_comp.angvel = new_angvel;
        }
    }
}
