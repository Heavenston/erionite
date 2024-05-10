use std::time::Instant;

use bevy::{diagnostic::{DiagnosticPath, Diagnostics}, math::DVec3, prelude::*};
use doprec::GlobalTransform64;
#[cfg(feature = "rapier")]
use rapier_overlay::*;
use utils::Vec3Ext as _;

#[derive(Resource)]
pub struct GravityConfig {
    pub gravity_contant: f64,
}

impl Default for GravityConfig {
    fn default() -> Self {
        Self {
            gravity_contant: 6.6743f64,
        }
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
    pub field_force: DVec3,
}

#[derive(Component, Default, Debug, Clone, Copy, PartialEq)]
pub struct GravityFieldComputeInfo {
    pub attractors_mass: f64,
    /// Attractor's position relative to the sample point
    pub relative_position: DVec3,
    /// Pre computed squared norm of [Self::relative_position]
    pub squared_distance: f64,
}

#[derive(Default, derivative::Derivative)]
#[derivative(Debug)]
pub enum GravityFunction {
    Linear,
    #[default]
    Quadratic,
    Cubic,
    Custom {
        /// result value is mutliplied by the gravitational constant to get
        /// the final force's vector norm
        #[derivative(Debug = "ignore")]
        function: Box<dyn Fn(&GravityFieldComputeInfo) -> f64 + Send + Sync>,
    }
}

impl GravityFunction {
    pub fn compute(&self, info: &GravityFieldComputeInfo) -> f64 {
        match self {
            GravityFunction::Linear => {
                if info.relative_position.is_zero_approx() {
                    return 0.;
                }
                info.attractors_mass / info.squared_distance.sqrt()
            },
            GravityFunction::Quadratic => {
                if info.relative_position.is_zero_approx() {
                    return 0.;
                }
                info.attractors_mass / info.squared_distance
            },
            GravityFunction::Cubic => {
                if info.relative_position.is_zero_approx() {
                    return 0.;
                }
                info.attractors_mass / info.squared_distance.sqrt().powi(3)
            },
            GravityFunction::Custom { function } => {
                function(info)
            },
        }
    }
}

#[derive(Component, Debug, Default)]
pub struct Attractor {
    pub function: GravityFunction,
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
    // TODO: Consider adding a generic or dyn abstraction to add any other
    //       stats
    strongest_attractor: Option<AttractorInfo>,
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

pub const GRAVITY_COMPUTE_SYSTEM_DURATION: DiagnosticPath =
    DiagnosticPath::const_new("gravity_compute");

pub(crate) fn compute_gravity_field_system(
    mut diagnostics: Diagnostics,
    cfg: Res<GravityConfig>,

    attractors: Query<(Entity, &GlobalTransform64, &Massive, &Attractor)>,
    mut victims: Query<(Entity, &GlobalTransform64, &mut GravityFieldSample, Option<&mut Attracted>)>,
) {
    let start = Instant::now();

    victims.par_iter_mut()
        .for_each(|(victim_entity, victim_pos, mut victim_sample, victim_attracted)| {
            let mut total_force = DVec3::ZERO;

            let mut strongest = None::<AttractorInfo>;
            let mut closest = None::<AttractorInfo>;

            for (
                attractor_entity, attractor_pos, attractor_mass, attractor
            ) in &attractors {
                if victim_entity == attractor_entity {
                    continue;
                }

                let diff = attractor_pos.translation() - victim_pos.translation();
                let squared_distance = diff.length_squared();
                let force = attractor.function.compute(&GravityFieldComputeInfo {
                    attractors_mass: attractor_mass.mass,
                    relative_position: diff,
                    squared_distance,
                });

                let attractor_info = AttractorInfo {
                    entity: attractor_entity,
                    force,
                    squared_distance,
                };
                if strongest
                    .map(|s| s.force < force)
                    .unwrap_or(true)
                {
                    strongest = Some(attractor_info);
                }
                if closest
                    .map(|s| s.squared_distance < attractor_info.squared_distance)
                    .unwrap_or(true)
                {
                    closest = Some(attractor_info);
                }

                total_force += diff.normalize() * cfg.gravity_contant * force;
            }

            if let Some(mut attracted) = victim_attracted {
                attracted.strongest_attractor = strongest;
                attracted.closest_attractor = closest;
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
