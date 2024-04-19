use bevy::{log, math::DVec3, prelude::*, utils::HashSet};
use crate::components::{ GlobalTransform64, Transform64, FloatingOrigin };

pub fn propagate_transforms_system(
    mut root_query: Query<(
        &Children, &Transform64, &mut GlobalTransform64
    ), Without<Parent>>,
    mut transform_query: Query<(
        &Transform64, &mut GlobalTransform64, Option<&Children>
    ), With<Parent>>,
) {
    let mut done = HashSet::<Entity>::new();
    let mut to_do = Vec::<(GlobalTransform64, Entity)>::new();
    for (root_children, &root_trans, mut root_global_trans) in &mut root_query {
        let new_global = GlobalTransform64::from(root_trans);
        if new_global != *root_global_trans {
            *root_global_trans = new_global;
        }

        to_do.extend(root_children.iter().map(|x| (*root_global_trans, *x)));
        done.extend(root_children.iter());
    }

    while let Some((parent_transform, entity)) = to_do.pop() {
        let Ok((&transform, mut global_trans, children)) = transform_query.get_mut(entity)
        else { continue; };

        let new_global = parent_transform * transform;
        if new_global != *global_trans {
            *global_trans = new_global;
        }

        if let Some(children) = children {
            to_do.extend(children.iter().copied()
                .filter(|x| !done.contains(x))
                .map(|x| (new_global, x)));
            done.extend(children.iter());
        }
    }
}

pub fn update_on_floating_origin_system(
    mut floating_origin: Query<(&GlobalTransform64, &mut GlobalTransform), With<FloatingOrigin>>,
    mut all_transforms: Query<(&GlobalTransform64, &mut GlobalTransform), (Without<FloatingOrigin>, Changed<GlobalTransform64>)>,
) {
    let Ok((&floating_origin, mut floating_origin_bevy_trans)) = floating_origin.get_single_mut()
    else {
        log::warn!("No floating origin found");
        return;
    };

    {
        let mut new = floating_origin;
        new.set_translation(DVec3::ZERO);
        let new = new.as_32();
        if new != *floating_origin_bevy_trans {
            *floating_origin_bevy_trans = new.into();
        }
    }

    let floating_trans = GlobalTransform64::from_translation(-floating_origin.translation());
    
    for (&global_trans, mut bevy_global_trans) in &mut all_transforms {
        *bevy_global_trans = (floating_trans * global_trans).as_32();
    }
}
