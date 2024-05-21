use super::*;

use std::time::Instant;

use arbitrary_int::u3;
use bevy::{diagnostic::Diagnostics, math::DVec3, prelude::*};
use doprec::GlobalTransform64;
#[cfg(feature = "rapier")]
use rapier_overlay::*;
use svo::SplittableData as _;
use utils::{DAabb, IsZeroApprox};
use bumpalo::boxed::Box as BumpBox;

#[derive(SystemSet, Debug, PartialEq, Eq, Default, Hash, Clone, Copy)]
pub struct GravitySystems;

#[cfg(feature = "rapier")]
pub(crate) fn sync_attractor_masses_with_colliders_system(
    mut query: Query<(
        &ColliderMassComp, &mut Massive
    ), (
        Changed<ColliderMassComp>,
    )>,
) {
    for (cmp, mut attractor) in &mut query {
        let &ColliderMassComp { mass } = cmp;
        attractor.mass = mass;
    }
}

pub(crate) fn update_svo_system(
    mut diagnostics: Diagnostics,
    cfg: Res<GravityConfig>,
    mut svo_ctx: ResMut<GravitySvoContext>,

    transforms: Query<&GlobalTransform64, With<Attractor>>,
    entity_transform_mass: Query<(Entity, &GlobalTransform64, &Massive), With<Attractor>>,
    mut attractors: Query<&mut Attractor>,
) {
    let start = Instant::now();

    if !cfg.enabled_svo {
        svo_ctx.alloc = default();
        return;
    }

    let root_aabb = transforms.iter()
        .fold(DAabb::new_center_size(DVec3::ZERO, DVec3::ONE), |mut aabb, transform| {
           aabb.expand_to_contain_point(transform.translation());
           aabb
        });
    svo_ctx.root_aabb = root_aabb;

    let max_depth = svo_ctx.max_depth;
    svo_ctx.alloc.build_svo(|herd| {
        let mut root_cell: svo::BumpCell<SvoData> = svo::LeafCell {
            data: SvoData {
                aabb: root_aabb,
                entities: entity_transform_mass.iter()
                    .map(|(entity, transform, massive)| SvoEntityRepr {
                        entity,
                        global_pos: transform.translation(),
                        mass: massive.mass,
                    })
                    .collect(),
                remaining_allowed_depth:
                    u8::try_from(max_depth).expect("too deep"),
            },
        }.into();

        let herd_local = thread_local::ThreadLocal::new();

        root_cell.par_auto_replace_with(
            default(), &|_, c| {
                let member = herd_local.get_or(|| herd.get());

                match c {
                    svo::Cell::Leaf(l) => {
                        if l.data.should_auto_split() {
                            let (data, splitted) = l.data.split();
                            svo::InternalCell {
                                children: splitted.map(|child_data| { svo::BumpBoxPtr(
                                    unsafe { BumpBox::from_raw(
                                        member.alloc(svo::LeafCell::new(child_data).into())
                                    ) }
                                ) }),
                                data,
                            }.into()
                        }
                        else {
                            l.into()
                        }
                    },
                    other => other,
                }
            }, &|_, c| c,
        );
        root_cell.auto_merge_borrow();

        for item in root_cell.iter() {
            let mut iter = attractors.iter_many_mut(
                item.data.entities.iter().map(|repr| repr.entity)
            );
            while let Some(mut attractor) = iter.fetch_next() {
                attractor.last_svo_position = Some(item.path.clone());
            }
        }

        root_cell
    });

    diagnostics.add_measurement(
        &GRAVITY_SVO_UPDATE_SYSTEM_DURATION,
        || start.elapsed().as_millis_f64(),
    );
}

pub(crate) fn compute_gravity_field_system_no_svo(
    mut diagnostics: Diagnostics,
    cfg: Res<GravityConfig>,

    attractors: Query<(Entity, &GlobalTransform64, &Massive, &Attractor)>,
    mut victims: Query<(
        Entity, &GlobalTransform64, &mut GravityFieldSample,
        Option<&mut TimeStep>,
    )>,

    mut update_counter: Local<u32>,
) {
    if cfg.enabled_svo {
        return;
    }
    let start = Instant::now();

    *update_counter = update_counter.wrapping_add(1);

    victims.par_iter_mut().for_each(|(
        victim_entity, victim_translation, mut victim_sample, victim_timestep
    )| {
        if let Some(mut victim_timestep) = victim_timestep {
            victim_timestep.offset = victim_entity.index();
            if (*update_counter + victim_timestep.offset) % victim_timestep.multiplier != 0 {
                victim_timestep.last_updated = false;
                // skip timestep for this entity
                return;
            }
            victim_timestep.last_updated = true;
        }
        let victim_pos = victim_translation.translation();

        let mut total_force = DVec3::ZERO;

        let mut closest_attractor = None::<AttractorInfo>;

        for (
            attractor_entity, attractor_pos, attractor_mass, _attractor
        ) in &attractors {
            if victim_entity == attractor_entity {
                continue;
            }

            let attractor_pos = attractor_pos.translation();
            let diff = attractor_pos - victim_pos;
            if diff.is_zero_approx() {
                continue;
            }
            let distance_squared = diff.length_squared();
            let distance = distance_squared.sqrt();
            let force = attractor_mass.mass / distance_squared;

            let info = AttractorInfo {
                entity: attractor_entity,
                force,
                squared_distance: distance_squared,
            };

            if closest_attractor
                .map(|oi| oi.squared_distance > info.squared_distance)
                .unwrap_or(true)
            {
                closest_attractor = Some(info);
            }

            if distance > victim_sample.min_affect_distance {
                total_force += (diff / distance) * cfg.gravity_constant * force;
            }
        }

        victim_sample.closest_attractor = closest_attractor;
        victim_sample.new_field_force(
            total_force, cfg.gravity_field_sample_backlog_count
        );
    });

    diagnostics.add_measurement(
        &GRAVITY_COMPUTE_SYSTEM_DURATION,
        || start.elapsed().as_millis_f64(),
    );
}

/// Does the actual svo traversal for a given victim
fn compute_svo_gravity_field_util(
    cfg: &GravityConfig,
    root_cell: &svo::BumpCell<'_, SvoData>,
    max_depth: u32,

    victim_entity: Entity,
    victim_pos: &GlobalTransform64,
    mut victim_sample: Mut<GravityFieldSample>,
    victim_attractor_bundle: Option<(&Massive, &Attractor)>,
) {
    let victim_pos = victim_pos.translation();

    let mut total_force = DVec3::ZERO;

    #[derive(Debug, Clone)]
    struct CellStep<'a, 'b> {
        cell: &'a svo::BumpCell<'b, SvoData>,
        path: svo::CellPath,
        current_child: Option<u3>,
    }

    let mut cell_stack = Vec::with_capacity(max_depth as usize);
    cell_stack.push(CellStep {
        cell: root_cell,
        path: svo::CellPath::default(),
        current_child: None,
    });
    'svo_loop: while let Some(step) = cell_stack.pop() {
        match step.cell {
            svo::Cell::Internal(internal) => {
                if let Some(current_child) = step.current_child {
                    // Re-push current cell for next child if any
                    if current_child != u3::new(0b111) {
                        cell_stack.push(CellStep {
                            current_child: Some(u3::new(current_child.value() + 1)),
                            ..step.clone()
                        });
                    }
                    // Push child
                    cell_stack.push(CellStep {
                        cell: internal.get_child(current_child),
                        path: step.path.clone().with_push(current_child),
                        current_child: None,
                    });
                    continue 'svo_loop;
                }

                'simplified: {
                    let mut stats = internal.data;

                    if let Some((victim_mass, victim_attractor)) = victim_attractor_bundle {
                        let contains_victim = victim_attractor.last_svo_position.as_ref()
                            .is_some_and(|pos| step.path.is_prefix_of(pos));
                        if contains_victim && FORCE_VISIT_OWN_CELLS {
                            break 'simplified;
                        }
                        if contains_victim && SHOULD_CORRECT_STATS_ON_OWN_CELL {
                            stats.center_of_mass -=
                                (victim_pos * victim_mass.mass) / stats.total_mass;
                            stats.total_mass -= victim_mass.mass;
                            stats.count -= 1;
                        }
                    }

                    let region_width = stats.aabb.size.x;
                    let diff_to_com = stats.center_of_mass - victim_pos;
                    let distance_to_com_squared = diff_to_com.length_squared();
                    let distance_to_com = distance_to_com_squared.sqrt();
                    let ratio = region_width / distance_to_com;

                    if stats.count != 1 && ratio > cfg.svo_skip_threshold {
                        break 'simplified;
                    }
                    
                    if distance_to_com > victim_sample.min_affect_distance {
                        let force = stats.total_mass / distance_to_com_squared;
                        total_force += (diff_to_com / distance_to_com) * cfg.gravity_constant * force;
                    }

                    continue 'svo_loop;
                }

                // With Some(0) each child will be seen
                cell_stack.push(CellStep {
                    current_child: Some(u3::new(0)),
                    ..step
                });
            },
            svo::Cell::Leaf(l) => {
                'entity_loop: for entity_repr in &l.data.entities {
                    if entity_repr.entity == victim_entity {
                        continue 'entity_loop;
                    }
                    let attractor_pos = entity_repr.global_pos;

                    let diff = attractor_pos - victim_pos;
                    if diff.is_zero_approx() {
                        continue 'entity_loop;
                    }
                    let squared_distance = diff.length_squared();
                    let distance = squared_distance.sqrt();
                    let force = entity_repr.mass / squared_distance;

                    let info = AttractorInfo {
                        entity: entity_repr.entity,
                        force,
                        squared_distance,
                    };
                    if victim_sample.closest_attractor
                        .map(|oi| oi.squared_distance > info.squared_distance)
                        .unwrap_or(true)
                    {
                        victim_sample.closest_attractor = Some(info);
                    }

                    if distance > victim_sample.min_affect_distance {
                        total_force += (diff / distance) * cfg.gravity_constant * force;
                    }
                }
            },
            svo::Cell::Packed(_) => unreachable!("No packed cell"),
        }
    }

    victim_sample.new_field_force(
        total_force, 
        cfg.gravity_field_sample_backlog_count,
    );
}

#[allow(clippy::type_complexity)]
pub(crate) fn compute_gravity_field_system_yes_svo(
    mut diagnostics: Diagnostics,
    cfg: Res<GravityConfig>,
    svo_ctx: Res<GravitySvoContext>,

    mut victims: Query<(
        Entity, &GlobalTransform64, &mut GravityFieldSample, Option<&mut TimeStep>,
        Option<(&Massive, &Attractor)>
    )>,

    mut update_counter: Local<u32>,
) {
    if !cfg.enabled_svo {
        return;
    }
    let start = Instant::now();

    *update_counter = update_counter.wrapping_add(1);

    let max_depth = svo_ctx.max_depth;
    svo_ctx.alloc.with_root_cell(|root_cell| {
        let Some(root_cell) = root_cell
        else { return; };
        victims.par_iter_mut().for_each(|(
            victim_entity, victim_pos, victim_sample,
            victim_timestep,
            victim_attractor_bundle
        )| {
            if let Some(mut victim_timestep) = victim_timestep {
                victim_timestep.offset = victim_entity.index();
                if (*update_counter + victim_timestep.offset) % victim_timestep.multiplier != 0 {
                    victim_timestep.last_updated = false;
                    // skip timestep for this entity
                    return;
                }
                victim_timestep.last_updated = true;
            }
            compute_svo_gravity_field_util(
                &cfg, root_cell,
                max_depth,
                victim_entity,
                victim_pos,
                victim_sample,
                victim_attractor_bundle,
            );
        });
    });

    diagnostics.add_measurement(
        &GRAVITY_COMPUTE_SYSTEM_DURATION,
        || start.elapsed().as_millis_f64(),
    );
}

#[cfg(feature = "rapier")]
pub(crate) fn apply_gravity_to_attracted_rigid_bodies_system(
    mut victims: Query<(
        &Massive, &GravityFieldSample,
        &mut RigidBodyExternalForceComp,
        Option<&TimeStep>,
    ), With<Attracted>>,
) {
    for (mass, gravity_sample, mut external_forces, timestep) in &mut victims {
        if let Some(timestep) = timestep {
            if !timestep.last_updated {
                continue;
            }
        }
        external_forces.force = gravity_sample.field_force(0).unwrap_or_default() * mass.mass;
    }
}
