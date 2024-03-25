use bevy::{input::mouse::{MouseMotion, MouseWheel}, prelude::*, window::{CursorGrabMode, PrimaryWindow}};
use std::f32::consts::*;

fn main() {
    App::new()
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin)
        .add_plugins(bevy::diagnostic::LogDiagnosticsPlugin::default())
        .add_plugins(DefaultPlugins)

        .add_systems(Startup, setup)
        .add_systems(Update, (camera, camera_capture))

        .init_resource::<Cam>()
        
        .run();
}

#[derive(Resource)]
pub struct Cam {
    pub entity: Option<Entity>,
    pub angle: Vec2,
    pub distance: f32,
}

impl FromWorld for Cam {
    fn from_world(_: &mut World) -> Self {
        Self {
            entity: None,
            angle: Vec2::ZERO,
            distance: 5.,
        }
    }
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut camera: ResMut<Cam>,
) {
    // circular base
    commands.spawn(PbrBundle {
        mesh: meshes.add(Circle::new(4.0).mesh().resolution(60).build()),
        material: materials.add(Color::WHITE),
        transform: Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        ..default()
    });
    // cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
        material: materials.add(Color::rgb_u8(124, 144, 255)),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });
    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
    // camera
    camera.entity = Some(commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    }).id());
}

fn camera_capture(
    kb_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let mut w = q_windows.single_mut();

    if mouse_input.just_pressed(MouseButton::Left) {
        w.cursor.grab_mode = CursorGrabMode::Locked;
        w.cursor.visible = false;
    }

    if kb_input.just_pressed(KeyCode::Escape) {
        w.cursor.grab_mode = CursorGrabMode::None;
        w.cursor.visible = true;
    }
}

fn camera(
    mut q_windows: Query<&mut Window, With<PrimaryWindow>>,

    mut transforms: Query<&mut Transform>,

    mut camera: ResMut<Cam>,
    time: Res<Time>,

    kb_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mouse_wheel_input: Res<ButtonInput<MouseButton>>,

    mut mouse_move_events: EventReader<MouseMotion>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
) {
    let w = q_windows.single_mut();

    let Some(entity) = camera.entity
    else { return; };

    let mut trans = transforms.get_mut(entity).unwrap();

    if !w.cursor.visible {
        for me in mouse_wheel_events.read() {
            if me.y < 0. {
                camera.distance *= 1.25;
            } else if me.y > 0. {
                camera.distance /= 1.25;
            }
        }
        camera.distance = camera.distance.clamp(2., 15.);

        for me in mouse_move_events.read() {
            camera.angle.y -= me.delta.y / 120.;
            camera.angle.x -= me.delta.x / 120.;
            camera.angle.y = camera.angle.y.clamp(-PI/2.+0.01, PI/2.-0.01);
        }
    }

    trans.translation = 
        Quat::from_rotation_y(camera.angle.x) *
        Quat::from_rotation_x(camera.angle.y) *
        (Vec3::new(0., 0., 1.) * camera.distance);
    trans.look_at(Vec3::ZERO, Vec3::Y);
}
