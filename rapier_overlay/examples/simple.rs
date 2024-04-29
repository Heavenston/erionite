#![feature(type_changing_struct_update)]
#![feature(option_take_if)]

use bevy::{diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin}, input::mouse::{MouseMotion, MouseWheel}, math::DVec3, pbr::{CascadeShadowConfigBuilder, DirectionalLightShadowMap}, prelude::*, render::mesh::{PlaneMeshBuilder, SphereKind, SphereMeshBuilder}, window::{CursorGrabMode, PrimaryWindow}};
use doprec::{ DoprecPlugin, FloatingOrigin, Transform64, Transform64Bundle };
use rapier_overlay::*;
use rapier::{geometry::ColliderBuilder};

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
            DefaultPlugins.build()
                .disable::<bevy::transform::TransformPlugin>()
                .disable::<bevy::log::LogPlugin>(),
            DoprecPlugin::default(),
            rapier_overlay::RapierPlugin::default(),
        ))

        .add_systems(Startup, setup_system)
        .add_systems(Update, (camera_system, update_debug_text_system))

        .insert_resource(DirectionalLightShadowMap { size: 2048 })
        .init_resource::<Cam>()
        
        .run();
}

#[derive(Resource)]
pub struct Cam {
    pub entity: Option<Entity>,
    pub speed: f64,
}

impl Cam {
}

impl FromWorld for Cam {
    fn from_world(_: &mut World) -> Self {
        Self {
            entity: None,
            speed: 10.,
        }
    }
}

#[derive(Component)]
struct DebugTextComponent;

fn setup_system(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut camera: ResMut<Cam>,
) {
    // Sun's light
    commands.spawn(DirectionalLightBundle {
        cascade_shadow_config: CascadeShadowConfigBuilder {
            maximum_distance: 1_000.,
            ..default()
        }.build(),
        directional_light: DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        ..default()
    }).insert(Transform64Bundle {
        local: Transform64::from_translation(DVec3::new(10., 10., 0.))
            .looking_at(DVec3::ZERO, DVec3::X),
        ..default()
    });

    // cube stack
    {
        let material = materials.add(StandardMaterial {
            base_color: Color::GOLD,
            perceptual_roughness: 0.1,
            metallic: 0.8,
            ..default()
        });

        let cube_size = DVec3::new(2., 1., 1.);
        let mesh = meshes.add(Cuboid::from_size(cube_size.as_vec3()));

        let origin = DVec3::new(-20., 0.1, 0.);

        let level_max = 50;
        for level in 0..level_max {
            let cube_count = level_max - level;
            let level_start = origin +
                (cube_size * DVec3::new(0., 0., -0.75)) * cube_count as f64 +
                (cube_size * DVec3::new(0., 0.99, 0.)) * level as f64;
            for x in 0..cube_count {
                commands.spawn((
                    PbrBundle {
                        material: material.clone(),
                        mesh: mesh.clone(),
                        ..PbrBundle::default()
                    },
                    ColliderBundle::from(ColliderBuilder::cuboid(
                        cube_size.x / 2.,
                        cube_size.y / 2.,
                        cube_size.z / 2.,
                    )),
                    RigidBodyBundle {
                        sleeping: RigidBodySleepingComp::new_sleeping(),
                        ..RigidBodyBundle::dynamic()
                    }
                )).insert(Transform64Bundle {
                    local: Transform64::from_translation(
                        level_start +
                        cube_size * 0.5 +
                        cube_size * DVec3::new(0., 0., 1.5) * x as f64
                    ),
                    ..default()
                });
            }
        }
    }

    // Floor
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(
                PlaneMeshBuilder::new(
                    Direction3d::Y, Vec2::new(200., 200.)
                ).build()
            ),
            material: materials.add(
                StandardMaterial {
                    base_color: Color::WHITE,
                    ..default()
                }
            ),
            ..PbrBundle::default()
        },
        ColliderBundle {
            mass: ColliderMassComp { mass: 0. },
            ..ColliderBundle::from(ColliderBuilder::cuboid(
                100., 0.1, 100.
            ))
        }
    )).insert(Transform64Bundle::default());

    let cam_pos = DVec3::new(0., 3., 0.);
    
    // camera
    camera.entity = Some(commands
        .spawn(Camera3dBundle {
            projection: Projection::Perspective(PerspectiveProjection {
                fov: 100f32.to_radians(),
                ..default()
            }),
            ..default()
        })
        .insert(Transform64Bundle {
            local: Transform64::from_translation(cam_pos)
                .looking_at(DVec3::NEG_X + cam_pos, cam_pos.normalize()),
            ..default()
        })
        .insert((
            FloatingOrigin,
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

    cam_query: Query<(&Transform64,)>,
    camera: Res<Cam>,

    mut debug_text: Query<&mut Text, With<DebugTextComponent>>,

    rigid_bodies: Query<(&RigidBodySleepingComp,)>,
) {
    let Some(cam_entity) = camera.entity
    else { return; };
    let (cam_transform,) = cam_query.get(cam_entity).unwrap();

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

    let mut rigid_body_count = 0;
    let mut slepping_body_count = 0;
    for (sleeping_comp,) in &rigid_bodies {
        rigid_body_count += 1;
        if sleeping_comp.sleeping() {
            slepping_body_count += 1;
        }
    }

    let cam_pos = cam_transform.translation;
    let cam_speed = camera.speed;

    let mut debug_text = debug_text.single_mut();
    debug_text.sections[0].value = format!("\
{fps:.1} fps - {frame_time:.3} ms/frame\n\
Camera: speed {cam_speed:.3}, position {cam_pos:.3?}\n\
Rigid Bodies: {rigid_body_count}, sleeping: {slepping_body_count}\n\
    ");
}

fn camera_system(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,

    mut camera_query: Query<(&mut Transform64,)>,

    mut camera: ResMut<Cam>,

    mut mouse_move_events: EventReader<MouseMotion>,
    mut mouse_wheel_events: EventReader<MouseWheel>,

    kb_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,

    time: Res<Time>,

    mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let mut window = q_windows.single_mut();
    
    let Some(entity) = camera.entity
    else { return; };

    let (
        mut camera_trans,
    ) = camera_query.get_mut(entity).unwrap();

    if mouse_input.just_pressed(MouseButton::Left) {
        window.cursor.grab_mode = CursorGrabMode::Confined;
        window.cursor.visible = false;
    }
    if mouse_input.just_released(MouseButton::Left) {
        window.cursor.grab_mode = CursorGrabMode::None;
        window.cursor.visible = true;
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

    if mouse_input.just_pressed(MouseButton::Right) {
        let mat = materials.add(StandardMaterial {
            base_color: Color::GRAY,
            perceptual_roughness: 0.8,
            ..default()
        });
        commands.spawn((
            PbrBundle {
                material: mat,
                mesh: meshes.add(SphereMeshBuilder::new(
                    1., SphereKind::Ico { subdivisions: 10 }
                ).build()),
                ..PbrBundle::default()
            },
            ColliderBundle {
                mass: ColliderMassComp { mass: 50. },
                ..ColliderBundle::from(ColliderBuilder::ball(1.))
            },
            RigidBodyBundle {
                linvel: VelocityComp::new(camera_trans.forward() * 20.),
                ..RigidBodyBundle::dynamic()
            },
        )).insert(Transform64Bundle {
            local: Transform64::from_translation(camera_trans.translation),
            ..default()
        });
    }

    let forward = camera_trans.forward();
    let left = camera_trans.left();

    {
        let target_down = DVec3::new(0., -1., 0.);
        let target_down_local = camera_trans.rotation.inverse() * target_down;
        let angle = DVec3::new(
            target_down_local.x,
            target_down_local.y,
            0.,
        ).angle_between(DVec3::new(0., -1., 0.));
        let dir = target_down_local.x.signum();

        camera_trans.rotate_local_z(angle.abs() * dir);
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
    camera_trans.translation += movement.normalize_or_zero() * camera.speed * (time.delta_seconds() as f64);
}
