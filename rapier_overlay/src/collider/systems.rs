use bevy::prelude::*;
use doprec::{GlobalTransform64, Transform64};
use rapier::geometry::{ColliderBuilder, ColliderMassProps};

use crate::*;

pub fn collider_init_system(
    mut commands: Commands,
    mut context: ResMut<RapierContext>,

    new_colliders_query: Query<(
        Entity,
        &GlobalTransform64,
        &ColliderShapeComp,
        &ColliderFrictionComp,
        &ColliderMassComp,

        Option<&RigidBodyHandleComp>,
    ), (
        Without<ColliderHandleComp>,
    )>,
) {
    for (
        entity, global_transform,
        shape, friction_comp, mass_comp,

        rigid_body,
    ) in &new_colliders_query {
        let mut collider = ColliderBuilder {
            mass_properties: ColliderMassProps::Mass(mass_comp.mass),
            friction: friction_comp.friction,
            ..ColliderBuilder::new(shape.shape.clone())
        };

        if rigid_body.is_none() {
            let t = Transform64::from(*global_transform);
            collider.position.translation = t.translation.to_rapier().into();
            collider.position.rotation = t.rotation.to_rapier();
        }

        let handle = context.collider_set.insert(collider);
        context.entities2colliders.insert(entity, handle);
        
        commands.entity(entity).insert(ColliderHandleComp {
            handle,
        });
        
        if let Some(rigid_body) = rigid_body {
            // Partial borrow because we need two mut borrows to context
            let RapierContext { collider_set, rigid_body_set, .. } = &mut *context;
            collider_set.set_parent(handle, Some(rigid_body.handle()), rigid_body_set);
        }
    }
}

pub fn collider_remove_system(
    mut commands: Commands,
    mut context: ResMut<RapierContext>,

    invalid_handles: Query<Entity, (With<ColliderHandleComp>, Or<(
        Without<ColliderShapeComp>,
        Without<ColliderFrictionComp>,
        Without<ColliderMassComp>,
    )>)>,
        
    mut removed_handles: RemovedComponents<ColliderHandleComp>,
) {
    for entity in std::iter::empty()
        .chain(
            removed_handles.read()
        )
        .chain(
            invalid_handles.iter().inspect(|&e| {
                commands.entity(e).remove::<ColliderHandleComp>();
            })
        )
    {
        let Some(handle) = context.entities2colliders.remove(&entity)
        else { continue; };

        let RapierContext {
            collider_set, island_manager, rigid_body_set, ..
        } = &mut *context;

        collider_set.remove(handle, island_manager, rigid_body_set, false);
    }
}

pub fn collider_update_system(
    mut context: ResMut<RapierContext>,

    shape_changed_query: Query<(
        &ColliderHandleComp, &ColliderShapeComp,
    ), (
        Changed<ColliderShapeComp>,
    )>,
    friction_changed_query: Query<(
        &ColliderHandleComp, &ColliderFrictionComp,
    ), (
        Changed<ColliderFrictionComp>,
    )>,
    mass_changed_query: Query<(
        &ColliderHandleComp, &ColliderMassComp,
    ), (
        Changed<ColliderMassComp>,
    )>,
) {
    for (handle, shape) in &shape_changed_query {
        let Some(collider) = context.collider_set.get_mut(handle.handle)
        else {
            log::warn!("Invalid collider handle");
            continue;
        };

        collider.set_shape(shape.shape.clone());
    }
    for (handle, friction) in &friction_changed_query {
        let Some(collider) = context.collider_set.get_mut(handle.handle)
        else {
            log::warn!("Invalid collider handle");
            continue;
        };

        collider.set_friction(friction.friction);
    }
    for (handle, mass) in &mass_changed_query {
        let Some(collider) = context.collider_set.get_mut(handle.handle)
        else {
            log::warn!("Invalid collider handle");
            continue;
        };

        collider.set_mass(mass.mass);
    }
}

