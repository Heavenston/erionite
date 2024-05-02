use bevy::{math::DVec3, prelude::*};
use doprec::GlobalTransform64;
use rapier_overlay::*;

pub const GRAVITY_CONSTANT: f64 = 6.6743;

pub struct GravityPlugin;

impl Plugin for GravityPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(PreUpdate, (
                sync_attractor_masses_with_colliders_system,
                compute_field_system,
                apply_gravity_to_attracteds_system,
            ).chain());
    }
}

#[derive(Component, Default)]
pub struct Massive {
    pub mass: f64,
}

/// Spatial entities with this component will have it updated with the
/// total gravital force of all Attractors on its position.
#[derive(Component, Default)]
pub struct GravityFieldSample {
    pub force: DVec3,
}

#[derive(Component, Default)]
pub struct Attractor;
#[derive(Component, Default)]
pub struct Attracted;

fn sync_attractor_masses_with_colliders_system(
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

fn compute_field_system(
    attractors: Query<(Entity, &GlobalTransform64, &Massive), With<Attractor>>,
    mut victims: Query<(Entity, &GlobalTransform64, &mut GravityFieldSample)>,
) {
    for (victim, victim_pos, mut victim_sample) in &mut victims {
        let mut total_force = DVec3::ZERO;

        for (other, other_pos, other_mass) in &attractors {
            if victim == other {
                continue;
            }

            let diff = other_pos.translation() - victim_pos.translation();
            let distance2 = diff.length_squared();
            if distance2.abs() < 0.0001 {
                continue;
            }
            let force = other_mass.mass / distance2;

            total_force += diff.normalize() * GRAVITY_CONSTANT * force;
        }

        victim_sample.force = total_force;
    }
}

fn apply_gravity_to_attracteds_system(
    mut victims: Query<(
        &Massive, &GravityFieldSample,
        &mut ExternalForceComp
    ), With<Attracted>>,
) {
    for (mass, gravity_sample, mut external_forces) in &mut victims {
        external_forces.force = gravity_sample.force * mass.mass;
    }
}
