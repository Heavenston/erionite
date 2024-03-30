#![feature(type_changing_struct_update)]
#![feature(option_take_if)]

mod generator;
mod svo_renderer;
mod svo_provider;

use bevy::{diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin}, ecs::system::EntityCommands, input::mouse::{MouseMotion, MouseWheel}, math::DVec3, prelude::*};
use svo_provider::generator_svo_provider;
use svo_renderer::{ChunkComponent, SvoRendererBundle, SvoRendererComponent, SvoRendererComponentOptions};
use utils::DAabb;
use std::f32::consts::*;

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
        .add_plugins((
            DefaultPlugins.build().disable::<bevy::log::LogPlugin>(),
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
    pub angle: Vec2,
    pub local_angle: Quat,
    pub distance: f32,
}

impl Cam {
    fn reset_dist(&mut self) {
        self.distance = 20_000.;
    }
}

impl FromWorld for Cam {
    fn from_world(_: &mut World) -> Self {
        let mut this = Self {
            entity: None,
            angle: Vec2::ZERO,
            local_angle: default(),
            distance: 0.,
        };
        this.reset_dist();
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
            chunk_subdiv_half_life: 100.,

            chunk_split_subdivs: 7,
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
        transform: Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
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

    mut debug_text: Query<&mut Text, With<DebugTextComponent>>,
    chunks: Query<(&ChunkComponent, &ViewVisibility)>,
) {
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
    let mut chunks_list = String::new();
    for (chunk, visible) in &chunks {
        chunk_count += 1;
        chunks_list += &format!("[{chunk_count:02}] {:?} @ {}", chunk.path, chunk.target_subdivs);
        if visible.get() {
            chunks_list += " [visible]"
        }
        if chunk.is_generating() {
            chunks_list += " [generating]"
        }
        if chunk.is_generating_mesh() {
            chunks_list += " [mesh generating]"
        }
        chunks_list.push('\n');
    }

    let mut debug_text = debug_text.single_mut();
    debug_text.sections[0].value = format!(
        "{fps:.1} fps - {frame_time:.3} ms/frame\nChunks: {chunk_count}\n{chunks_list}"
    );
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
        camera.distance -= 2000.;
        if me.y < 0. {
            camera.distance *= 1.1;
        } else if me.y > 0. {
            camera.distance /= 1.1;
        }
        camera.distance += 2000.;
    }
    if mouse_input.pressed(MouseButton::Left) {
        camera.local_angle = default();
        for me in mouse_move_events.read() {
            camera.angle.y -= me.delta.y / 120.;
            camera.angle.x -= me.delta.x / 120.;
            camera.angle.y = camera.angle.y.clamp(-PI/2.+0.01, PI/2.-0.01);
        }
    }
    if mouse_input.pressed(MouseButton::Right) {
        for me in mouse_move_events.read() {
            camera.local_angle *= Quat::from_rotation_y(me.delta.x / 240.);
            camera.local_angle *= Quat::from_rotation_x(me.delta.y / 240.);
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
    trans.rotate_local(camera.local_angle);
}
