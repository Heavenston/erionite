use bevy::prelude::*;
use rapier::{dynamics::{RigidBodyActivation, RigidBodyBuilder}, geometry::{ColliderBuilder, ColliderMassProps}};

use crate::*;

pub fn rigid_body_init_system(
    mut commands: Commands,
    mut context: ResMut<RapierContext>,

    new_rigid_body_query: Query<(
        Entity,
        &RigidBodyComp,
        &RigidBodyDampingComp,
        &RigidBodySleepingComp,
        &VelocityComp,
        &AngularVelocityComp,

        Option<&ColliderHandleComp>,
    ), (
        Without<RigidBodyHandleComp>,
    )>,
) {
    for (
        entity,
        rigid_body,
        damping, sleeping, velocity, angular_velocity,

        collider,
    ) in &new_rigid_body_query {
        let mut rigid_body = RigidBodyBuilder::new(rigid_body.kind);
        rigid_body.linear_damping = damping.linear;
        rigid_body.angular_damping = damping.angular;
        rigid_body.can_sleep = sleeping.can_sleep;
        rigid_body.linvel = velocity.linvel.to_rapier();
        rigid_body.angvel = angular_velocity.angvel.to_rapier();

        let handle = context.rigid_body_set.insert(rigid_body);

        commands.entity(entity)
            .insert(RigidBodyHandleComp {
                handle,
            });

        if let Some(col_comp) = collider {
            // Partial borrow because we need two mut borrows to context
            let RapierContext { collider_set, rigid_body_set, .. } = &mut *context;
            collider_set.set_parent(col_comp.handle(), Some(handle), rigid_body_set);
        }
    }
}

pub fn rigid_body_remove_system(
    mut commands: Commands,
    mut context: ResMut<RapierContext>,

    invalid_handles: Query<Entity, (With<RigidBodyHandleComp>, Or<(
        Without<RigidBodyComp>,
        Without<RigidBodyDampingComp>,
        Without<RigidBodySleepingComp>,
    )>)>,
        
    mut removed_handles: RemovedComponents<RigidBodyHandleComp>,
) {
    for entity in std::iter::empty()
        .chain(
            removed_handles.read()
        )
        .chain(
            invalid_handles.iter().inspect(|&e| {
                commands.entity(e).remove::<RigidBodyHandleComp>();
            })
        )
    {
        let Some(handle) = context.entities2rigidbodies.remove(&entity)
        else { continue; };

        let RapierContext {
            collider_set, island_manager, rigid_body_set, impulse_joint_set, multibody_joint_set, ..
        } = &mut *context;

        rigid_body_set.remove(
            handle, 
            island_manager, 
            collider_set, 
            impulse_joint_set, 
            multibody_joint_set,
            false
        );
    }
}

pub fn rigid_body_update_system(
    mut context: ResMut<RapierContext>,

    rigid_body_changed_query: Query<(
        &RigidBodyHandleComp, &RigidBodyComp,
    ), (
        Changed<RigidBodyComp>,
    )>,
    damping_changed_query: Query<(
        &RigidBodyHandleComp, &RigidBodyDampingComp,
    ), (
        Changed<RigidBodyDampingComp>,
    )>,
    sleeping_changed_query: Query<(
        &RigidBodyHandleComp, &RigidBodySleepingComp,
    ), (
        Changed<RigidBodySleepingComp>,
    )>,
) {
    for (handle, comp) in &rigid_body_changed_query {
        let Some(rigid_body) = context.rigid_body_set.get_mut(handle.handle)
        else {
            log::warn!("Invlid Rigid Body handle");
            continue;
        };

        rigid_body.set_enabled(comp.enabled);
        rigid_body.set_body_type(comp.kind, false);
    }

    for (handle, comp) in &damping_changed_query {
        let Some(rigid_body) = context.rigid_body_set.get_mut(handle.handle)
        else {
            log::warn!("Invlid Rigid Body handle");
            continue;
        };

        rigid_body.set_linear_damping(comp.linear);
        rigid_body.set_angular_damping(comp.angular);
    }

    for (handle, comp) in &sleeping_changed_query {
        let Some(rigid_body) = context.rigid_body_set.get_mut(handle.handle)
        else {
            log::warn!("Invlid Rigid Body handle");
            continue;
        };

        if comp.can_sleep {
            rigid_body.activation_mut().linear_threshold = RigidBodyActivation::default_linear_threshold();
            rigid_body.activation_mut().angular_threshold = RigidBodyActivation::default_angular_threshold();
        }
        else {
            rigid_body.activation_mut().linear_threshold = -1.;
            rigid_body.activation_mut().angular_threshold = -1.;
        }
    }
}

