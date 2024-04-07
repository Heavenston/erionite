#![feature(type_changing_struct_update)]
#![feature(option_take_if)]

mod generator;
mod svo_renderer;
use svo_renderer::{ChunkComponent, SvoRendererBundle, SvoRendererComponent, SvoRendererComponentOptions};
mod svo_provider;
use svo_provider::generator_svo_provider;

use bevy::{diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin}, ecs::system::EntityCommands, input::mouse::{MouseMotion, MouseWheel}, math::DVec3, prelude::*, window::{CursorGrabMode, PrimaryWindow}};
use utils::DAabb;
use bevy_rapier3d::prelude::*;

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

        .add_plugins(RapierDebugRenderPlugin::default())

        .add_plugins((
            DefaultPlugins.build().disable::<bevy::log::LogPlugin>(),
            RapierPhysicsPlugin::<NoUserData>::default(),
            svo_renderer::SvoRendererPlugin::default()
        ))

        .add_systems(Startup, setup)
        .add_systems(Update, (camera, update_debug_text))

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
        let mut this = Self {
            entity: None,
            speed: 20.,
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
    let radius = 2000.;
    let aabb: DAabb = DAabb::new_center_size(DVec3::ZERO, DVec3::splat(radius*2.));

    let mat = materials.add(StandardMaterial {
        ..default()
    });

    commands.spawn(SvoRendererBundle {
        transform: TransformBundle::default(),
        svo_render: SvoRendererComponent::new(SvoRendererComponentOptions {
            max_subdivs: 12,
            min_subdivs: 4,
            chunk_falloff_multiplier: 20.,
            
            chunk_split_subdivs: 6,
            chunk_merge_subdivs: 5,

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
        transform: Transform::from_xyz(0., (radius*4.) as f32, 0.)
            .looking_at(Vec3::ZERO, Vec3::Y),
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
    mut transforms: Query<&mut Transform>,

    mut camera: ResMut<Cam>,

    mut mouse_move_events: EventReader<MouseMotion>,
    mut mouse_wheel_events: EventReader<MouseWheel>,

    kb_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,

    mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let mut window = q_windows.single_mut();
    
    let Some(entity) = camera.entity
    else { return; };

    let mut trans = transforms.get_mut(entity).unwrap();

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
            let mov = me.delta / -300.;

            trans.rotate_local_y(mov.x);
            trans.rotate_local_x(mov.y);
        }

        let f = trans.forward();
        let l = trans.left();
        if kb_input.pressed(KeyCode::KeyW) {
            trans.translation += f * camera.speed;
        }
        if kb_input.pressed(KeyCode::KeyS) {
            trans.translation -= f * camera.speed;
        }
        if kb_input.pressed(KeyCode::KeyA) {
            trans.translation += l * camera.speed;
        }
        if kb_input.pressed(KeyCode::KeyD) {
            trans.translation -= l * camera.speed;
        }
    }
}
