use bevy::{math::DVec3, prelude::*};
use doprec::{GlobalTransform64, Transform64};
use rapier::dynamics::IntegrationParameters;

use crate::*;

pub fn physics_step_system(
    time: Res<Time<Fixed>>,
    mut context: ResMut<RapierContext>,
    cfg: Option<Res<RapierConfig>>,
) {
    let default_cfg = RapierConfig::default();
    let cfg = cfg.as_ref().map(|res| &**res).unwrap_or(&default_cfg);

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
        &cfg.gravity.to_rapier(),
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
    mut context: ResMut<RapierContext>,

    mut globals_transes_query: Query<&mut GlobalTransform64>,

    mut rigid_bodies_query: Query<(
        Entity,
        &RigidBodyHandleComp,
        &mut RigidBodySleepingComp,
        &mut VelocityComp,
        &mut AngularVelocityComp,
        &mut Transform64,

        Option<&Parent>,
    )>,
) {
    let RapierContext { rigid_body_set, entities_last_set_transform, .. }
        = &mut *context;

    for (
        entity, handle_comp, mut sleeping_comp, mut linvel_comp, mut angvel_comp,
        mut transform_comp, parent_comp,
    ) in rigid_bodies_query.iter_mut() {
        let Some(rigid_body) = rigid_body_set.get(handle_comp.handle())
        else { continue; };

        let parent_trans = parent_comp
            .and_then(|parent| globals_transes_query.get(parent.get()).ok())
            .map(|&trans| trans)
            .unwrap_or_default();

        let Ok(mut global_trans_comp) = globals_transes_query.get_mut(entity)
        else { continue; };

        if sleeping_comp.sleeping != rigid_body.is_sleeping() {
            sleeping_comp.sleeping = rigid_body.is_sleeping();
        }

        if rigid_body.is_moving() {
            let new_transform = Transform64::from(parent_trans.inverse()) *
                Transform64 {
                    translation: rigid_body.translation().to_bevy(),
                    rotation: rigid_body.rotation().to_bevy(),
                    scale: DVec3::ONE,
                };
            let new_global_transform = parent_trans * new_transform;

            if new_transform != *transform_comp {
                entities_last_set_transform.insert(entity, new_global_transform);
                // Sets global to avoid thinking it changed when it just wans't synced
                // yet
                *global_trans_comp = new_global_transform;
                *transform_comp = new_transform;
            }
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
