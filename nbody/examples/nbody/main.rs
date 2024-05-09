mod orbit_camera;

use bevy::{diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin}, math::DVec3, pbr::{CascadeShadowConfigBuilder, DirectionalLightShadowMap}, prelude::*, render::mesh::PlaneMeshBuilder};
use doprec::{FloatingOrigin, Transform64, Transform64Bundle};
use rapier_overlay::{rapier::geometry::{Capsule, ColliderBuilder, SharedShape}, ColliderBundle, ColliderMassComp, RapierConfig, RigidBodyBundle};

fn main() {
    utils::logging::setup_basic_logging().unwrap();

    App::new()
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin)
        .add_plugins(bevy::diagnostic::LogDiagnosticsPlugin::default())

        .add_plugins((
            DefaultPlugins.build()
                .disable::<bevy::transform::TransformPlugin>()
                .disable::<bevy::log::LogPlugin>(),
            doprec::DoprecPlugin::default(),
            nbody::NBodyPlugin::default(),
            orbit_camera::OrbitCameraPlugin::default(),
        ))

        .add_systems(Startup, setup_system)
        .add_systems(Update, update_debug_text_system)

        .insert_resource(DirectionalLightShadowMap { size: 2048 })
        .insert_resource(RapierConfig {
            gravity: DVec3::ZERO,
        })
        
        .run();
}

#[derive(Component, Default)]
struct DebugTextComp;

fn setup_system(
    gravity_cfg: Res<nbody::GravityConfig>,

    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,

    assets: Res<AssetServer>,
) {
    let cam_pos = DVec3::new(0., 3., 0.);
    
    // camera
    commands.spawn_empty()
        .insert(ColliderBundle::from(ColliderBuilder::new(SharedShape::new(
            Capsule::new_y(1., 0.5)
        )).mass(100.)))
        .insert(Camera3dBundle {
            projection: Projection::Perspective(PerspectiveProjection {
                fov: 100f32.to_radians(),
                ..default()
            }),
            ..default()
        })
        .insert(Transform64Bundle::default())
        .insert((
            FloatingOrigin,
            orbit_camera::OrbitCameraComp {
                target_translation: cam_pos,
                ..default()
            },
        ))
    ;

    let root_uinode = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::SpaceBetween,

                ..default()
            },
            ..default()
        })
        .id();

    commands.spawn(NodeBundle {
        style: Style {
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Start,
            flex_grow: 1.,
            margin: UiRect::axes(Val::Px(5.), Val::Px(5.)),
            ..default()
        },
        ..default()
    }).with_children(|builder| {
        builder.spawn(TextBundle::from_section(
            "Bonjur",
            TextStyle {
                font_size: 15.0,
                ..default()
            },
        )).insert(DebugTextComp);
    }).set_parent(root_uinode);
}

fn update_debug_text_system(
    diagnostics: Res<DiagnosticsStore>,

    cam_query: Query<(&Transform64, &orbit_camera::OrbitCameraComp)>,

    mut debug_text: Query<&mut Text, With<DebugTextComp>>,
) {
    let (cam_transform, orbit_cam) = cam_query.single();

    let fps = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|diag| diag.smoothed())
        .unwrap_or(0.);
    let frame_time = diagnostics.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|diag| diag.smoothed())
        .unwrap_or(0.);

    let cam_pos = cam_transform.translation;
    let cam_speed = orbit_cam.movement_speed;

    let mut debug_text = debug_text.single_mut();
    debug_text.sections[0].value = format!("\
    {fps:.1} fps - {frame_time:.3} ms/frame\n\
    Camera: speed {cam_speed:.3}, position {cam_pos:.3?}\n\
    ");
}
