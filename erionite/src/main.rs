#![feature(type_changing_struct_update)]
#![feature(option_take_if)]

mod generator;
mod svo_renderer;
mod svo_provider;

use bevy::{ecs::system::EntityCommands, input::mouse::{MouseMotion, MouseWheel}, math::DVec3, prelude::*};
use svo_provider::generator_svo_provider;
use svo_renderer::{SvoRendererBundle, SvoRendererComponent, SvoRendererComponentOptions};
use utils::DAabb;
use std::f32::consts::*;

fn main() {
    App::new()
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin)
        .add_plugins(bevy::diagnostic::LogDiagnosticsPlugin::default())
        .add_plugins((
            DefaultPlugins.set(bevy::log::LogPlugin {
                level: bevy::log::Level::INFO,
                filter: "wgpu=error,erionite=trace".to_string(),
                ..default()
            }),
            svo_renderer::SvoRendererPlugin::default()
        ))

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
        self.distance = 3000.;
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
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut camera: ResMut<Cam>,
) {
    let radius = 2000.;
    let aabb: DAabb = DAabb::new_center_size(DVec3::ZERO, DVec3::splat(radius*2.));

    let mat = materials.add(StandardMaterial {
        ..default()
    });

    commands.spawn(SvoRendererBundle {
        transform: TransformBundle::default(),
        svo_render: SvoRendererComponent::new(SvoRendererComponentOptions {
            total_subdivs: 4..8,
            chunk_split_subdivs: 4,
            chunk_merge_subdivs: 1,

            chunk_subdiv_distances: 0.0..20.0,
            root_aabb: aabb,
            on_new_chunk: Some(Box::new(move |mut commands: EntityCommands<'_>| {
                commands.insert(mat.clone());
            }) as Box<_>),
        }),
        svo_provider: generator_svo_provider::GeneratorSvoProvider::new(
            generator::PlanetGenerator {
                radius,
                seed: 5,
            },
            aabb
        ).into(),
    });

    commands.spawn(DirectionalLightBundle {
        transform: Transform {
            rotation: Quat::from_rotation_x(std::f32::consts::PI / 4.),
            ..default()
        },
        ..default()
    });
    
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
