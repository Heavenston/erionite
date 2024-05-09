#![feature(type_changing_struct_update)]
#![feature(option_take_if)]

mod generator;
mod svo_renderer;
use svo_renderer::{ChunkComponent, SvoRendererBundle, SvoRendererComponent, SvoRendererComponentOptions};
mod svo_provider;
use svo_provider::generator_svo_provider;
pub mod task_runner;

use bevy::{core_pipeline::{bloom::{BloomCompositeMode, BloomSettings}, Skybox}, diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin}, ecs::system::EntityCommands, input::mouse::{MouseMotion, MouseWheel}, math::DVec3, pbr::{CascadeShadowConfigBuilder, DirectionalLightShadowMap, NotShadowCaster, NotShadowReceiver}, prelude::*, render::mesh::{SphereKind, SphereMeshBuilder}, window::{CursorGrabMode, PrimaryWindow}};
use utils::DAabb;
use doprec::*;
use rapier_overlay::{rapier::geometry::ColliderBuilder, *};

fn main() {
    utils::logging::setup_basic_logging().unwrap();

    App::new()
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin)
        .add_plugins(bevy::diagnostic::LogDiagnosticsPlugin::default())

        .add_plugins((
            DefaultPlugins.build()
                .disable::<bevy::transform::TransformPlugin>()
                .disable::<bevy::log::LogPlugin>(),
            svo_renderer::SvoRendererPlugin::default(),
            nbody::NBodyPlugin::default(),
            DoprecPlugin::default(),
            RapierPlugin::default(),
        ))

        .add_systems(Startup, setup_system)
        .add_systems(Update, (camera_system, update_debug_text_system))

        .insert_resource(DirectionalLightShadowMap { size: 2048 })
        .insert_resource(RapierConfig {
            gravity: DVec3::ZERO,
        })
        .init_resource::<Cam>()
        
        .run();
}

#[derive(Resource)]
pub struct Cam {
    pub entity: Option<Entity>,
    pub speed: f64,
    pub forced_gravity_toggle: bool,
    /// Changed by the cam controller
    /// Changing it manually have no effect
    pub gravity_redirect_enabled: bool,
}

impl Cam {
}

impl FromWorld for Cam {
    fn from_world(_: &mut World) -> Self {
        Self {
            entity: None,
            speed: 50.,
            forced_gravity_toggle: false,
            gravity_redirect_enabled: false,
        }
    }
}

#[derive(Component)]
struct DebugTextComponent;

fn setup_system(
    gravity_cfg: Res<nbody::GravityConfig>,

    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut camera: ResMut<Cam>,

    assets: Res<AssetServer>,
) {
    let subdivs = 17u32;
    let aabb_size = 2f64.powi((subdivs-2) as i32);
    let radius = aabb_size / 4.;
    let aabb: DAabb = DAabb::new_center_size(DVec3::ZERO, DVec3::splat(aabb_size));
    // let volume = (radius.powi(3) * std::f64::consts::PI * 4.) / 3.;
    // let mass = volume / 1_000_000.;
    let target_surface_gravity = 9.8;
    let mass = (target_surface_gravity / gravity_cfg.gravity_contant) * radius.powi(2);

    log::info!("AABB Size    : {aabb_size}");
    log::info!("Planet radius: {radius}");
    log::info!("Planet's mass: {radius}");

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(SphereMeshBuilder::new(
                10_000.,
                bevy::render::mesh::SphereKind::Ico { subdivisions: 10 },
            ).build()),
            material: materials.add(StandardMaterial {
                base_color: Color::WHITE * 1_000.,
                // emissive: Color::WHITE * 500.,
                unlit: true,
                ..default()
            }),
            ..PbrBundle::default()
        },
        NotShadowCaster,
        NotShadowReceiver,
    )).insert(Transform64Bundle {
        local: Transform64::from_translation(DVec3::new(0., 0., 1_000_000.)),
        ..default()
    });

    commands.spawn(DirectionalLightBundle {
        transform: Transform::default(),
        cascade_shadow_config: CascadeShadowConfigBuilder {
            maximum_distance: 1_000_000.,
            ..default()
        }.build(),
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        ..default()
    }).insert(Transform64Bundle::default());

    let mat = materials.add(StandardMaterial {
        perceptual_roughness: 0.8,
        metallic: 0.,
        ..default()
    });

    commands.spawn(SvoRendererBundle {
        transform: Transform64Bundle::default(),
        svo_render: SvoRendererComponent::new(SvoRendererComponentOptions {
            max_subdivs: subdivs,
            min_subdivs: 4,
            chunk_falloff_multiplier: 30.,
            
            chunk_split_subdivs: 6,
            chunk_merge_subdivs: 5,

            root_aabb: aabb,
            on_new_chunk: Some(Box::new({
                let mat = mat.clone();
                move |mut commands: EntityCommands<'_>| {
                    commands.insert(mat.clone());
                }
            }) as Box<_>),

            ..default()
        }),
        svo_provider: generator_svo_provider::GeneratorSvoProvider::new(
            generator::PlanetGenerator {
                radius,
                seed: 1,
            },
            // generator::SphereGenerator {
            //     radius,
            //     material: svo::TerrainCellKind::Pink,
            // },
            aabb
        ).into(),
    }).insert((
        nbody::Massive {
            mass,
        },
        nbody::Attractor::default(),
    ));

    let cam_pos = DVec3::new(
        0.,
        0.,
        radius+200.
    );
    // let cam_pos = DVec3::new(0., radius * 5., 0.);
    
    // camera
    camera.entity = Some(commands
        .spawn(Camera3dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            ..default()
        })
        .insert(Transform64Bundle {
            local: Transform64::from_translation(cam_pos)
                .looking_at(DVec3::NEG_X + cam_pos, cam_pos.normalize()),
            ..default()
        })
        .insert((
            FloatingOrigin,
            nbody::GravityFieldSample::default(),
            BloomSettings {
                intensity: 0.02,
                composite_mode: BloomCompositeMode::EnergyConserving,

                ..default()
            },
            Skybox {
                image: assets.load("images/skybox/skybox.ktx2"),
                brightness: 1000.0,
            },
        ))
        .id()
    );

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
            "Chunks: ",
            TextStyle {
                font_size: 15.0,
                ..default()
            },
        )).insert(DebugTextComponent);
    }).set_parent(root_uinode);
}

fn update_debug_text_system(
    time: Res<Time>,
    diagnostics: Res<DiagnosticsStore>,

    cam_query: Query<(&Transform64, &nbody::GravityFieldSample)>,
    camera: Res<Cam>,

    mut debug_text: Query<&mut Text, With<DebugTextComponent>>,
    chunks: Query<&ChunkComponent>,
) {
    let Some(cam_entity) = camera.entity
    else { return; };
    let (cam_transform, cam_gravity) = cam_query.get(cam_entity).unwrap();

    let mut fps = 0.0;
    if let Some(fps_diagnostic) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(fps_smoothed) = fps_diagnostic.smoothed() {
            fps = fps_smoothed;
        }
    }

    let mut frame_time = time.delta_seconds_f64();
    if let Some(frame_time_diagnostic) =
        diagnostics.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
    {
        if let Some(frame_time_smoothed) = frame_time_diagnostic.smoothed() {
            frame_time = frame_time_smoothed;
        }
    }

    let mut chunk_count = 0;
    let mut chunk_gen_count = 0;
    let mut chunk_mesh_gen_count = 0;
    let mut chunk_col_gen_count = 0;
    for chunk in &chunks {
        chunk_count += 1;
        if chunk.is_generating() {
            chunk_gen_count += 1;
        }
        if chunk.is_generating_mesh() {
            chunk_mesh_gen_count += 1;
        }
        if chunk.is_generating_collider() {
            chunk_col_gen_count += 1;
        }
    }

    let cam_pos = cam_transform.translation;
    let cam_speed = camera.speed;

    let mut grav_info = String::new();
    grav_info += &format!("G: {:.2}", cam_gravity.field_force.length());
    grav_info += ", grav_redirect: ";
    grav_info += if camera.gravity_redirect_enabled { "enabled" } else { "disabled" };
    grav_info += "\n";

    let mut debug_text = debug_text.single_mut();
    debug_text.sections[0].value = format!("\
{fps:.1} fps - {frame_time:.3} ms/frame \n\
Chunks: {chunk_count}, gen {chunk_gen_count}, mesh {chunk_mesh_gen_count}, col {chunk_col_gen_count} \n\
Camera: speed {cam_speed:.3}, position {cam_pos:.3?} \n\
{grav_info}
    ");
}

fn camera_system(
    mut commands: Commands,

    mut camera_query: Query<(&mut Transform64, &nbody::GravityFieldSample)>,
    mut renderers: Query<&mut SvoRendererComponent>,

    mut camera: ResMut<Cam>,

    mut mouse_move_events: EventReader<MouseMotion>,
    mut mouse_wheel_events: EventReader<MouseWheel>,

    kb_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,

    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,

    mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let mut window = q_windows.single_mut();
    
    let Some(entity) = camera.entity
    else { return; };

    let (
        mut camera_trans,
        camera_gravity,
    ) = camera_query.get_mut(entity).unwrap();

    if mouse_input.just_pressed(MouseButton::Left) {
        window.cursor.grab_mode = CursorGrabMode::Confined;
        window.cursor.visible = false;
    }
    if mouse_input.just_released(MouseButton::Left) {
        window.cursor.grab_mode = CursorGrabMode::None;
        window.cursor.visible = true;
    }

    if kb_input.just_pressed(KeyCode::KeyR) {
        for mut r in &mut renderers {
            r.options.enable_subdivs_update = !r.options.enable_subdivs_update;
        }
    }

    if kb_input.just_pressed(KeyCode::KeyG) {
        camera.forced_gravity_toggle = !camera.forced_gravity_toggle;
    }

    if kb_input.just_pressed(KeyCode::KeyB) {
        log::info!("Spawning ball !");
        let radius = 0.25;
        let mass = 5.;
        commands.spawn((
            Transform64Bundle {
                local: Transform64::from_translation(camera_trans.translation),
                ..default()
            },
            VisibilityBundle::default(),
            meshes.add(SphereMeshBuilder::new(radius, SphereKind::Ico {
                subdivisions: 5,
            }).build()),
            materials.add(StandardMaterial {
                perceptual_roughness: 0.2,
                metallic: 0.8,
                base_color: Color::GOLD,
                ..default()
            }),
            ColliderBundle {
                mass: ColliderMassComp { mass },
                ..ColliderBundle::from(ColliderBuilder::ball(radius as f64))
            },
            RigidBodyBundle::dynamic(),
            nbody::GravityFieldSample::default(),
            nbody::Massive::default(),
            nbody::Attracted,
        ));
    }

    for mwe in mouse_wheel_events.read() {
        if mwe.y < 0. {
            camera.speed *= 0.9;
        }
        else if mwe.y > 0. {
            camera.speed *= 1.1;
        }
    }

    if mouse_input.pressed(MouseButton::Left) {
        for me in mouse_move_events.read() {
            let mov = me.delta.as_dvec2() / -300.;

            camera_trans.rotate_local_y(mov.x);
            camera_trans.rotate_local_x(mov.y);
        }
    }

    let forward = camera_trans.forward();
    let left = camera_trans.left();

    camera.gravity_redirect_enabled =
        !camera.forced_gravity_toggle &&
        camera_gravity.field_force.length() > 9.;
    if camera.gravity_redirect_enabled {
        let target_down = camera_gravity.field_force.normalize();
        let target_down_local = camera_trans.rotation.inverse() * target_down;
        let angle = DVec3::new(
            target_down_local.x,
            target_down_local.y,
            0.,
        ).angle_between(DVec3::new(0., -1., 0.));
        let dir = target_down_local.x.signum();
        let prop = angle / std::f64::consts::PI;

        let rot_speed = prop.sqrt() * 5.;
        let rot_speed = if prop < 0.001 { angle } else { rot_speed * time.delta_seconds_f64() };
        camera_trans.rotate_local_z((rot_speed * dir).clamp(-angle.abs(), angle.abs()));
    }

    let mut movement = DVec3::ZERO;
    if kb_input.pressed(KeyCode::KeyW) {
        movement += forward;
    }
    if kb_input.pressed(KeyCode::KeyS) {
        movement -= forward;
    }
    if kb_input.pressed(KeyCode::KeyA) {
        movement += left;
    }
    if kb_input.pressed(KeyCode::KeyD) {
        movement -= left;
    }
    camera_trans.translation += movement.normalize_or_zero() * camera.speed * time.delta_seconds_f64();
}
