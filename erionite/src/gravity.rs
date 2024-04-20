use bevy::{math::DVec3, prelude::*};
use doprec::GlobalTransform64;

const GRAVITY_CONSTANT: f64 = 6.6743;

pub struct GravityPlugin;

impl Plugin for GravityPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, (
                // sync_attractor_masses_with_colliders_system,
                compute_field_system,
                apply_gravity_to_attracteds_system,
            ).chain());
    }
}

#[derive(Component)]
pub struct Massive {
    pub mass: f64,
}

/// Spatial entities with this component will have it updated with the
/// total gravital force of all Attractors on its position.
#[derive(Component)]
pub struct GravityFieldSample {
    pub force: DVec3,
}

#[derive(Component)]
pub struct Attractor;
#[derive(Component)]
pub struct Attracted;

// fn sync_attractor_masses_with_colliders_system(
//     mut query: Query<(&ColliderMassProperties, &mut Massive)>,
// ) {
//     for (cmp, mut attractor) in &mut query {
//         let ColliderMassProperties::MassProperties(props) = cmp
//         else { continue; };
//         attractor.mass = props.mass as f64;
//     }
// }

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
    _victims: Query<(Entity, &GlobalTransform64, &Massive, &GravityFieldSample), With<Attracted>>,
) { }
