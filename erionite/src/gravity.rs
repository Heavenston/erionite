use bevy::{math::DVec3, prelude::*};
use bevy_rapier3d::{dynamics::ExternalForce, geometry::ColliderMassProperties};

const GRAVITY_CONSTANT: f64 = 6.6743;

pub struct GravityPlugin;

impl Plugin for GravityPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Update, (
                sync_attractor_masses_with_colliders_system,
                apply_physics_system,
            ));
    }
}

#[derive(Component)]
pub struct Massive {
    pub mass: f64,
}

#[derive(Component)]
pub struct Attractor;
#[derive(Component)]
pub struct Attracted;

fn sync_attractor_masses_with_colliders_system(
    mut query: Query<(&ColliderMassProperties, &mut Massive)>,
) {
    for (cmp, mut attractor) in &mut query {
        let ColliderMassProperties::MassProperties(props) = cmp
        else { continue; };
        attractor.mass = props.mass as f64;
    }
}

fn apply_physics_system(
    attractors: Query<(Entity, &GlobalTransform, &Massive), With<Attractor>>,
    mut victims: Query<(Entity, &GlobalTransform, &mut ExternalForce, &Massive), With<Attracted>>,
) {
    for (victim, victim_pos, mut victim_force, victim_mass) in &mut victims {
        let mut total_force = DVec3::ZERO;

        for (other, other_pos, other_mass) in &attractors {
            if victim == other {
                continue;
            }

            let diff = (other_pos.translation() - victim_pos.translation()).as_dvec3();
            let distance2 = diff.length_squared() as f64;
            if distance2.abs() < 0.0001 {
                continue;
            }
            let force = (victim_mass.mass * other_mass.mass) / distance2;

            total_force += diff.normalize() * GRAVITY_CONSTANT * force;
        }

        victim_force.force = total_force.as_vec3();
        victim_force.torque = Vec3::ZERO;
    }
}
