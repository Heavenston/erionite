use bevy::{math::DVec3, prelude::*};
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

#[derive(Component, Default)]
pub struct Massive {
    pub mass: f64,
}

/// Spatial entities with this component will have it updated with the
/// total gravital force of all Attractors on its position.
///
/// Actual gravity force applied on body should be field_force * body_mass
#[derive(getset::CopyGetters, Component, Default)]
#[getset(get_copy = "pub")]
pub struct GravityFieldSample {
    pub field_force: DVec3,
}

#[derive(Default)]
pub enum GravityFunction {
    Linear,
    #[default]
    Quadratic,
    Cubic,
    Custom {
        /// First argument is the attractor's mass, second one is
        /// the relative position of the sampled point
        /// result value is mutliplied by the gravitational constant to get
        /// the final force's vector norm
        function: Box<dyn Fn(f64, DVec3) -> f64 + Send + Sync>,
    }
}

impl GravityFunction {
    pub fn compute(&self, mass: f64, pos: DVec3) -> f64 {
        match self {
            GravityFunction::Linear => {
                if pos.is_zero_approx() {
                    return 0.;
                }
                mass / pos.length()
            },
            GravityFunction::Quadratic => {
                if pos.is_zero_approx() {
                    return 0.;
                }
                mass / pos.length_squared()
            },
            GravityFunction::Cubic => {
                if pos.is_zero_approx() {
                    return 0.;
                }
                mass / pos.length().powi(3)
            },
            GravityFunction::Custom { function } => {
                function(mass, pos)
            },
        }
    }
}

#[derive(Component, Default)]
pub struct Attractor {
    pub function: GravityFunction,    
}
#[derive(Component, Default)]
pub struct Attracted;

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

pub(crate) fn compute_gravity_field_system(
    cfg: Res<GravityConfig>,

    attractors: Query<(Entity, &GlobalTransform64, &Massive, &Attractor)>,
    mut victims: Query<(Entity, &GlobalTransform64, &mut GravityFieldSample)>,
) {
    for (victim_entity, victim_pos, mut victim_sample) in &mut victims {
        let mut total_force = DVec3::ZERO;

        for (
            attractor_entity, attractor_pos, attractor_mass, attractor
        ) in &attractors {
            if victim_entity == attractor_entity {
                continue;
            }

            let diff = attractor_pos.translation() - victim_pos.translation();
            let force = attractor.function.compute(attractor_mass.mass, diff);

            total_force += diff.normalize() * cfg.gravity_contant * force;
        }

        victim_sample.field_force = total_force;
    }
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
