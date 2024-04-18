#![feature(type_changing_struct_update)]
#![feature(option_take_if)]

mod generator;
mod svo_renderer;
use svo_renderer::{ChunkComponent, SvoRendererBundle, SvoRendererComponent, SvoRendererComponentOptions};
mod svo_provider;
use svo_provider::generator_svo_provider;
pub mod task_runner;
mod gravity;

use bevy::{diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin}, ecs::system::EntityCommands, input::mouse::{MouseMotion, MouseWheel}, math::DVec3, pbr::{CascadeShadowConfigBuilder, DirectionalLightShadowMap}, prelude::*, render::mesh::SphereMeshBuilder, window::{CursorGrabMode, PrimaryWindow}};
use utils::DAabb;
use bevy_rapier3d::{prelude::*, rapier::geometry::ColliderBuilder};

fn setup_logger() -> Result<(), Box<dyn std::error::Error>> {
    use fern::colors::{ ColoredLevelConfig, Color };
    use log::LevelFilter as LF;

    let colors = ColoredLevelConfig::new()
        .error(Color::Red)
        .warn(Color::Yellow)
        .info(Color::Green)
        .debug(Color::White)
        .trace(Color::BrightBlack);

    fern::Dispatch::new()
        .filter(|m|
            !m.target().starts_with("mio")
        )
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{color_line}[{}][{}] {}\x1B[39m",
                record.target(),
                record.level(),
                message,

                color_line = format_args!(
                    "\x1B[{}m",
                    colors.get_color(&record.level()).to_fg_str()
                ),
            ))
        })
        .level(LF::Info)
        .level_for("wgpu", LF::Error)
        .level_for("wgpu_hal", LF::Error)
        .level_for("erionite", LF::Trace)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}

fn main() {
    setup_logger().unwrap();

    App::new()
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin)
        .add_plugins(bevy::diagnostic::LogDiagnosticsPlugin::default())

        // .add_plugins(RapierDebugRenderPlugin::default())

        .add_plugins((
            DefaultPlugins.build().disable::<bevy::log::LogPlugin>(),
            RapierPhysicsPlugin::<NoUserData>::default(),
            svo_renderer::SvoRendererPlugin::default(),
            gravity::GravityPlugin,
        ))

        .add_systems(Startup, setup)
        .add_systems(Update, (camera, update_debug_text))

        .insert_resource(RapierConfiguration {
            gravity: Vec3::ZERO,
            ..default()
        })
        .insert_resource(DirectionalLightShadowMap { size: 2048 })
        .init_resource::<Cam>()
        
        .run();
}

#[derive(Resource)]
pub struct Cam {
    pub entity: Option<Entity>,
    pub speed: f32,
}

impl Cam {
}

impl FromWorld for Cam {
    fn from_world(_: &mut World) -> Self {
        let this = Self {
            entity: None,
            speed: 2.,
        };
        this
    }
}

#[derive(Component)]
struct DebugTextComponent;

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut camera: ResMut<Cam>,
) {
    let subdivs = 18u32;
    let aabb_size = 2f64.powi(subdivs as i32);
    let radius = aabb_size / 4.;
    let aabb: DAabb = DAabb::new_center_size(DVec3::ZERO, DVec3::splat(aabb_size));

    log::info!("AABB Size: {aabb_size}");
    log::info!("Planet radius: {radius}");

    let mat = materials.add(StandardMaterial {
        perceptual_roughness: 0.8,
        metallic: 0.,
        ..default()
    });

    commands.spawn(SvoRendererBundle {
        transform: TransformBundle::default(),
        svo_render: SvoRendererComponent::new(SvoRendererComponentOptions {
            max_subdivs: subdivs,
            min_subdivs: 5,
            chunk_falloff_multiplier: 30.,
            
            chunk_split_subdivs: 5,
            chunk_merge_subdivs: 6,

            root_aabb: aabb,
            on_new_chunk: Some(Box::new(move |mut commands: EntityCommands<'_>| {
                commands.insert(mat.clone());
            }) as Box<_>),

            ..default()
        }),
        svo_provider: generator_svo_provider::GeneratorSvoProvider::new(
            generator::PlanetGenerator {
                radius,
                seed: 5,
            },
            aabb
        ).into(),
    }).insert((
        gravity::Massive {
            mass: 1_000_000_000.,
        },
        gravity::Attractor,
    ));

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
    });

    let cam_pos = DVec3::new(0., 0., radius+200.);
    // let cam_pos = DVec3::new(0., radius * 5., 0.);
    
    // camera
    camera.entity = Some(commands.spawn(Camera3dBundle {
        transform: Transform::from_translation(cam_pos.as_vec3())
            .looking_at(Vec3::NEG_X + cam_pos.as_vec3(), cam_pos.normalize().as_vec3()),
            // .looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    }).id());
    // // ui camera
    // commands.spawn(Camera2dBundle::default());

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

fn update_debug_text(
    time: Res<Time>,
    diagnostics: Res<DiagnosticsStore>,

    transforms: Query<&Transform>,
    camera: Res<Cam>,

    mut debug_text: Query<&mut Text, With<DebugTextComponent>>,
    chunks: Query<&ChunkComponent>,
) {
    let Some(cam_entity) = camera.entity
    else { return; };
    let cam_transform = transforms.get(cam_entity).unwrap();

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

    let mut debug_text = debug_text.single_mut();
    debug_text.sections[0].value = format!("\
{fps:.1} fps - {frame_time:.3} ms/frame \n\
Chunks: {chunk_count}, gen {chunk_gen_count}, mesh {chunk_mesh_gen_count}, col {chunk_col_gen_count} \n\
Camera: speed {cam_speed}, position {cam_pos} \n\
    ");
}

fn camera(
    mut commands: Commands,

    mut camera_query: Query<&mut Transform>,
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

    let mut trans = camera_query.get_mut(entity).unwrap();

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

    if kb_input.just_pressed(KeyCode::KeyB) {
        log::info!("Spawning ball !");
        commands.spawn((
            TransformBundle {
                local: Transform::from_translation(trans.translation),
                ..default()
            },
            VisibilityBundle::default(),
            Collider::ball(1.),
            meshes.add(SphereMeshBuilder::new(1., bevy::render::mesh::SphereKind::Ico {
                subdivisions: 5,
            }).build()),
            materials.add(StandardMaterial {
                perceptual_roughness: 0.8,
                metallic: 0.,
                base_color: Color::rgb(1., 0.5, 0.),
                ..default()
            }),
            ColliderMassProperties::Mass(10.),
            ExternalForce::default(),
            RigidBody::Dynamic,
            gravity::Massive { mass: 50. },
            gravity::Attracted,
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
            let mov = me.delta / -300.;

            trans.rotate_local_y(mov.x);
            trans.rotate_local_x(mov.y);
        }
    }

    let f = *trans.forward();
    let l = *trans.left();
    let mut movement = Vec3::ZERO;
    if kb_input.pressed(KeyCode::KeyW) {
        movement += f;
    }
    if kb_input.pressed(KeyCode::KeyS) {
        movement -= f;
    }
    if kb_input.pressed(KeyCode::KeyA) {
        movement += l;
    }
    if kb_input.pressed(KeyCode::KeyD) {
        movement -= l;
    }
    trans.translation += movement.normalize_or_zero() * camera.speed * time.delta_seconds();
}
