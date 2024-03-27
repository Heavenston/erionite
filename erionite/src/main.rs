mod generator;

use bevy::{input::mouse::{MouseMotion, MouseWheel}, math::{bounding::Aabb3d, DVec3}, prelude::*};
use generator::Generator;
use svo::CellPath;
use utils::DAabb;
use std::f32::consts::*;

fn main() {
    App::new()
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin)
        .add_plugins(bevy::diagnostic::LogDiagnosticsPlugin::default())
        .add_plugins(DefaultPlugins)

        .add_systems(Startup, setup)
        .add_systems(Update, camera)

        .init_resource::<Cam>()
        
        .run();
}

#[derive(Resource)]
pub struct Cam {
    pub entity: Option<Entity>,
    pub angle: Vec2,
    pub distance: f32,
}

impl Cam {
    fn reset_dist(&mut self) {
        self.distance = 600.;
    }
}

impl FromWorld for Cam {
    fn from_world(_: &mut World) -> Self {
        let mut this = Self {
            entity: None,
            angle: Vec2::ZERO,
            distance: 0.,
        };
        this.reset_dist();
        this
    }
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    // mut materials: ResMut<Assets<StandardMaterial>>,
    mut camera: ResMut<Cam>,
) {
    let et = 5;
    let aabb: DAabb = DAabb::new_center_size(DVec3::ZERO, DVec3::splat(300.));

    println!("Generating...");
    let svo = generator::PlanetGenerator {
        radius: 300.,
        seed: 5,
    }.generate_chunk(aabb, CellPath::new(), et);

    println!("Generating mesh...");
    let mut out = svo::mesh_generation::marching_cubes::Out::default();
    svo::mesh_generation::marching_cubes::run(
        &mut out, CellPath::new(), &svo, aabb.into(), et
    );
    let mesh = meshes.add(out.into_mesh());
    println!("Finished");

    let parent = commands.spawn(TransformBundle::default()).id();
    commands.spawn(PbrBundle {
        mesh,
        ..default()
    }).set_parent(parent);
    
    // camera
    camera.entity = Some(commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    }).id());
}

fn camera(
    mut transforms: Query<&mut Transform>,

    mut camera: ResMut<Cam>,

    mut mouse_move_events: EventReader<MouseMotion>,
    mut mouse_wheel_events: EventReader<MouseWheel>,

    kb_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
) {
    let Some(entity) = camera.entity
    else { return; };

    let mut trans = transforms.get_mut(entity).unwrap();

    for me in mouse_wheel_events.read() {
        if me.y < 0. {
            camera.distance *= 1.25;
        } else if me.y > 0. {
            camera.distance /= 1.25;
        }
    }
    if mouse_input.pressed(MouseButton::Left) {
        for me in mouse_move_events.read() {
            camera.angle.y -= me.delta.y / 120.;
            camera.angle.x -= me.delta.x / 120.;
            camera.angle.y = camera.angle.y.clamp(-PI/2.+0.01, PI/2.-0.01);
        }
    }
    if kb_input.just_pressed(KeyCode::KeyR) {
        camera.reset_dist();
    }

    trans.translation = 
        Quat::from_rotation_y(camera.angle.x) *
        Quat::from_rotation_x(camera.angle.y) *
        (Vec3::new(0., 0., 1.) * camera.distance);
    trans.look_at(Vec3::ZERO, Vec3::Y);
}
