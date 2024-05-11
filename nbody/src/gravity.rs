use std::time::Instant;

use bevy::{diagnostic::{DiagnosticPath, Diagnostics}, math::DVec3, prelude::*};
use doprec::GlobalTransform64;
#[cfg(feature = "rapier")]
use rapier_overlay::*;
use svo::AggregateData;
use utils::{AsVecExt, DAabb, IsZeroApprox, Vec3Ext as _};
use either::Either;
use arbitrary_int::*;

pub const GRAVITY_COMPUTE_SYSTEM_DURATION: DiagnosticPath =
    DiagnosticPath::const_new("gravity_compute");
pub const GRAVITY_SVO_UPDATE_SYSTEM_DURATION: DiagnosticPath =
    DiagnosticPath::const_new("svo_update_compute");

#[derive(SystemSet, Debug, PartialEq, Eq, Default, Hash, Clone, Copy)]
pub struct GravitySystems;

#[derive(Resource)]
pub struct GravityConfig {
    pub gravity_contant: f64,
    pub enabled_svo: bool,
}

impl Default for GravityConfig {
    fn default() -> Self {
        Self {
            gravity_contant: 6.6743f64,
            enabled_svo: true,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SvoEntityRepr {
    pub entity: Entity,
    /// Pos is relative to the cell it is in
    /// between (0., 0., 0.) and (1., 1., 1.)
    pub pos: DVec3,
    pub mass: f64,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct SvoData {
    pub entities: Vec<SvoEntityRepr>,
    pub remaining_allowed_depth: u8,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct SvoInternalData {
    pub count: u32,
    pub total_mass: f64,
    /// Relative to the AABB -> 0,0 for the min corner and 1,1 for the max corner
    pub center_of_mass: DVec3,
}

impl svo::Data for SvoData {
    type Internal = SvoInternalData;
}

impl svo::AggregateData for SvoData {
    fn aggregate<'a>(
        children: [svo::EitherDataRef<Self>; 8]
    ) -> Self::Internal {
        let mut count = 0;
        let mut total_mass = 0f64;
        let mut weighed_pos_sum = DVec3::ZERO;

        for (comp, cell) in svo::CellPath::components().iter().zip(children.into_iter()) {
            let sub_cell_min = comp.as_uvec().as_dvec3() / 2.;
            match cell {
                Either::Left(internal) => {
                    count += internal.count;
                    total_mass += internal.total_mass;
                    weighed_pos_sum += internal.total_mass * (
                        internal.center_of_mass / 2. + sub_cell_min
                    );
                },
                Either::Right(leaf) => {
                    count += u32::try_from(leaf.entities.len()).expect("too much entities!!");
                    total_mass += leaf.entities.iter().map(|e| e.mass).sum::<f64>();
                    weighed_pos_sum += leaf.entities.iter()
                        .map(|e| e.pos / 2. + sub_cell_min)
                        .sum::<DVec3>();
                },
            }
        }

        SvoInternalData {
            total_mass,
            count,
            center_of_mass: weighed_pos_sum / total_mass,
        }
    }
}

impl svo::SplittableData for SvoData {
    fn should_auto_split(&self) -> bool {
        self.remaining_allowed_depth > 0 &&
        self.entities.len() > 10
    }

    fn split(self) -> (Self::Internal, [Self; 8]) {
        let mut children = svo::CellPath::components().map(|_| SvoData {
            remaining_allowed_depth: self.remaining_allowed_depth.saturating_sub(1),
            ..default()
        });

        for entity in self.entities {
            let mut target = 0b000usize;
            if entity.pos.x > 0.5 {
                target |= 0b001;
            }
            if entity.pos.y > 0.5 {
                target |= 0b010;
            }
            if entity.pos.z > 0.5 {
                target |= 0b100;
            }
            children[target].entities.push(entity);
        }

        let internal = SvoData::aggregate(
            children.each_ref().map(|leaf| Either::Right(leaf))
        );

        (internal, children)
    }
}

impl svo::MergeableData for SvoData {
    fn should_auto_merge(
        this: &Self::Internal,
        _children: [&Self; 8]
    ) -> bool {
        this.count < 10
    }

    fn merge(
        _this: Self::Internal,
        children: [Self; 8]
    ) -> Self {
        Self {
            remaining_allowed_depth: children.iter()
                .map(|c| c.remaining_allowed_depth).max().unwrap_or_default() + 1,
            entities: children.into_iter().flat_map(|x| x.entities).collect(),
        }
    }
}

impl svo::InternalData for SvoInternalData {
    
}

#[derive(Resource)]
pub struct GravitySvoContext {
    root_cell: Option<svo::BoxCell<SvoData>>,
    root_aabb: DAabb,
    max_depth: u32,
}

impl Default for GravitySvoContext {
    fn default() -> Self {
        Self {
            root_cell: default(),
            root_aabb: DAabb::new_center_size(DVec3::zero(), DVec3::splat(100_000f64)),
            max_depth: 20,
        }
    }
}

impl GravitySvoContext {
    pub fn depth(&self) -> u32 {
        self.root_cell.as_ref().map(|svo| svo.depth()).unwrap_or(0)
    }

    pub fn max_depth(&self) -> u32 {
        self.max_depth
    }

    pub fn root_aabb(&self) -> DAabb {
        self.root_aabb
    }
}

#[derive(Component, Default, Debug, Clone, Copy, PartialEq)]
pub struct Massive {
    pub mass: f64,
}

/// Spatial entities with this component will have it updated with the
/// total gravital force of all Attractors on its position.
///
/// Actual gravity force applied on body should be field_force * body_mass
#[derive(getset::CopyGetters, Component, Debug, Default, PartialEq, Clone, Copy)]
#[getset(get_copy = "pub")]
pub struct GravityFieldSample {
    /// Field force at previous time step
    pub previous_field_force: DVec3,
    /// Field force at current time step
    pub field_force: DVec3,
}

#[derive(Component, Debug, Default, Clone)]
pub struct Attractor {
    pub last_svo_position: Option<svo::CellPath>,
}

#[derive(Debug, Clone, Copy)]
pub struct AttractorInfo {
    pub entity: Entity,
    pub force: f64,
    pub squared_distance: f64,
}

#[derive(getset::CopyGetters, Component, Debug, Default, Clone, Copy)]
#[getset(get_copy = "pub")]
pub struct Attracted {
    closest_attractor: Option<AttractorInfo>,
}

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

    mut attractors: Query<(Entity, Ref<GlobalTransform64>, Ref<Massive>, &mut Attractor)>,
) {
    let start = Instant::now();

    let mut was_reset = false;
    if !cfg.enabled_svo {
        svo_ctx.root_cell = None;
        return;
    }
    if true {
        was_reset = true;
        svo_ctx.root_cell = Some(svo::LeafCell {
            data: SvoData {
                entities: default(),
                remaining_allowed_depth: u8::try_from(svo_ctx.max_depth).expect("too deep"),
            },
        }.into());
    }

    for (
        attractor_entity,
        attractor_transform,
        attractor_massive,
        mut attractor,
    ) in &mut attractors {
        if !was_reset &&
            attractor.last_svo_position.is_some() &&
            !attractor_transform.is_changed() &&
            !attractor_massive.is_changed()
        {
            continue;
        }

        let mut relative_pos = (attractor_transform.translation() - svo_ctx.root_aabb.min()) / svo_ctx.root_aabb.size;
        let mut target_cell = svo_ctx.root_cell.as_mut().expect("set at the start");
        let mut path = svo::CellPath::new();

        while let svo::Cell::Internal(internal) = target_cell {
            let mut new_comp = 0b000u8;
            if relative_pos.x > 0.5 {
                relative_pos.x -= 0.5;
                new_comp |= 0b001;
            }
            if relative_pos.y > 0.5 {
                relative_pos.y -= 0.5;
                new_comp |= 0b010;
            }
            if relative_pos.z > 0.5 {
                relative_pos.z -= 0.5;
                new_comp |= 0b100;
            }
            relative_pos *= 2.;

            path.push(u3::new(new_comp));

            target_cell = internal.get_child_mut(u3::new(new_comp));
        }

        let svo::Cell::Leaf(leaf_target_cell) = target_cell
        else { unreachable!("not internal so must be leaf") };

        leaf_target_cell.data.entities.push(SvoEntityRepr {
            entity: attractor_entity,
            pos: relative_pos,
            mass: attractor_massive.mass,
        });

        let root_cell = svo_ctx.root_cell.as_mut().expect("set at the start");
        root_cell.auto_split_on_path(path.clone());
        root_cell.auto_merge_on_path(path.clone());

        attractor.last_svo_position = Some(path);
    }

    diagnostics.add_measurement(
        &GRAVITY_SVO_UPDATE_SYSTEM_DURATION,
        || start.elapsed().as_millis_f64(),
    );
}

pub(crate) fn compute_gravity_field_system_no_svo(
    mut diagnostics: Diagnostics,
    cfg: Res<GravityConfig>,

    attractors: Query<(Entity, &GlobalTransform64, &Massive, &Attractor)>,
    mut victims: Query<(Entity, &GlobalTransform64, &mut GravityFieldSample, Option<&mut Attracted>)>,
) {
    if cfg.enabled_svo {
        return;
    }
    let start = Instant::now();

    victims.par_iter_mut().for_each(|(
        victim_entity, victim_pos, mut victim_sample, victim_attracted,
    )| {
        let mut total_force = DVec3::ZERO;

        let mut closest_attractor = None::<AttractorInfo>;

        for (
            attractor_entity, attractor_pos, attractor_mass, _attractor
        ) in &attractors {
            if victim_entity == attractor_entity {
                continue;
            }

            let diff = attractor_pos.translation() - victim_pos.translation();
            if diff.is_zero_approx() {
                continue;
            }
            let squared_distance = diff.length_squared();
            let force = attractor_mass.mass / squared_distance;

            let info = AttractorInfo {
                entity: attractor_entity,
                force,
                squared_distance,
            };

            if closest_attractor
                .map(|oi| oi.squared_distance > info.squared_distance)
                .unwrap_or(true)
            {
                closest_attractor = Some(info);
            }

            total_force += diff.normalize() * cfg.gravity_contant * force;
        }

        if let Some(mut victim_attracted) = victim_attracted {
            victim_attracted.closest_attractor = closest_attractor;
        }

        victim_sample.previous_field_force = victim_sample.field_force;
        victim_sample.field_force = total_force;
    });

    diagnostics.add_measurement(
        &GRAVITY_COMPUTE_SYSTEM_DURATION,
        || start.elapsed().as_millis_f64(),
    );
}

pub(crate) fn compute_gravity_field_system_yes_svo(
    mut diagnostics: Diagnostics,
    cfg: Res<GravityConfig>,
    svo_ctx: Res<GravitySvoContext>,

    mut victims: Query<(Entity, &GlobalTransform64, &mut GravityFieldSample)>,
) {
    if !cfg.enabled_svo {
        return;
    }
    let start = Instant::now();

    let Some(root_cell) = &svo_ctx.root_cell
    else { return };

    victims.par_iter_mut().for_each(|(victim_entity, victim_pos, mut victim_sample)| {
        let mut total_force = DVec3::ZERO;

        let mut cell_stack = vec![(
            root_cell,
            svo::CellPath::new(),
            svo_ctx.root_aabb,
        )];
        while let Some((cell, path, aabb)) = cell_stack.pop() {
            match cell {
                svo::Cell::Internal(internal) => {
                    for comp in svo::CellPath::components() {
                        cell_stack.push((
                            internal.get_child(comp),
                            path.clone().with_push(comp),
                            svo::CellPath::new().with_push(comp).get_aabb(aabb),
                        ));
                    }
                },
                svo::Cell::Leaf(l) => {
                    for entity_repr in &l.data.entities {
                        if entity_repr.entity == victim_entity {
                            continue;
                        }
                        let pos = aabb.position + aabb.size * entity_repr.pos;

                        let diff = pos - victim_pos.translation();
                        if diff.is_zero_approx() {
                            continue;
                        }
                        let squared_distance = diff.length_squared();
                        let force = entity_repr.mass / squared_distance;

                        total_force += diff.normalize() * cfg.gravity_contant * force;
                    }
                },
                svo::Cell::Packed(_) => unreachable!("No packed cell"),
            }
        }

        victim_sample.field_force = total_force;
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
        &mut RigidBodyExternalForceComp
    ), With<Attracted>>,
) {
    for (mass, gravity_sample, mut external_forces) in &mut victims {
        external_forces.force = gravity_sample.field_force * mass.mass;
    }
}
