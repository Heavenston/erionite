#![feature(type_changing_struct_update)]
#![feature(option_take_if)]

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    input::mouse::{MouseMotion, MouseWheel},
    math::DVec3,
    pbr::{CascadeShadowConfigBuilder, DirectionalLightShadowMap},
    prelude::*,
    render::mesh::{PlaneMeshBuilder, SphereKind, SphereMeshBuilder},
    window::{CursorGrabMode, PrimaryWindow},
};
use doprec::{ DoprecPlugin, FloatingOrigin, GlobalTransform64, Transform64, Transform64Bundle };
use rapier::{dynamics::RigidBodyType, geometry::{Capsule, ColliderBuilder, SharedShape}, pipeline::QueryFilterFlags};
use rapier_overlay::*;

fn main() {
    utils::logging::setup_basic_logging().unwrap();

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
        .add_systems(Update, (
            update_debug_text_system,
            player_input_system,
        ))
        .add_systems(FixedUpdate, (
            player_physics_system.before(PhysicsStepSystems),
            player_after_physics_system.after(PhysicsStepSystems),
        ))

        .insert_resource(DirectionalLightShadowMap { size: 2048 })
        .init_resource::<Player>()
        
        .run();
}

#[derive(Resource)]
pub struct Player {
    pub entity: Entity,
    pub camera_entity: Entity,
    pub speed: f64,

    pub input_velocity: DVec3,
    pub velocity: DVec3,

    pub collide_with_rigid_bodies: bool,
}

impl FromWorld for Player {
    fn from_world(_: &mut World) -> Self {
        Self {
            entity: Entity::PLACEHOLDER,
            camera_entity: Entity::PLACEHOLDER,
            speed: 10.,

            input_velocity: default(),
            velocity: default(),

            collide_with_rigid_bodies: true,
        }
    }
}

#[derive(Component)]
struct DebugTextComponent;

fn setup_system(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut player: ResMut<Player>,
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

        let origin = DVec3::new(-20., 1.1, 0.);

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
                        // sleeping: RigidBodySleepingComp::new_sleeping(),
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
    )).insert(Transform64Bundle {
        local: Transform64 {
            translation: DVec3::new(0., 1., 0.),
            ..default()
        },
        ..default()
    });

    let cam_pos = DVec3::new(0., 3., 0.);
    
    // player
    player.entity = commands.spawn_empty()
        .insert(ColliderBundle::from(ColliderBuilder::new(SharedShape::new(
            Capsule::new_y(1., 0.5)
        )).mass(100.)))
        .insert(RigidBodyBundle {
            ..RigidBodyBundle::new(RigidBodyType::KinematicPositionBased)
        })
        .insert(Transform64Bundle {
            local: Transform64::from_translation(cam_pos)
                .looking_at(DVec3::NEG_X + cam_pos, cam_pos.normalize()),
            ..default()
        })
        .insert(CharacterControllerBundle {
            comp: CharacterControllerComp {
                ..default()
            },
            ..default()
        })
        .with_children(|c| {
            player.camera_entity = c.spawn_empty()
                .insert(FloatingOrigin)
                .insert(Camera3dBundle {
                    projection: Projection::Perspective(PerspectiveProjection {
                        fov: 100f32.to_radians(),
                        ..default()
                    }),
                    ..default()
                })
                .insert(Transform64Bundle {
                    local: Transform64::from_translation(DVec3::new(
                        0.,
                        1.5,
                        0.,
                    )),
                    ..default()
                })
                .id()
            ;
        })
        .id()
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

    player_query: Query<(&Transform64,)>,
    player: Res<Player>,

    mut debug_text: Query<&mut Text, With<DebugTextComponent>>,

    rigid_bodies: Query<(&RigidBodySleepingComp,)>,
) {
    let (player_transform,) = player_query.get(player.entity).unwrap();

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

    let player_pos = player_transform.translation;
    let player_speed = player.speed;

    let inf_inert = !player.collide_with_rigid_bodies;

    let mut debug_text = debug_text.single_mut();
    debug_text.sections[0].value = format!("\
    {fps:.1} fps - {frame_time:.3} ms/frame\n\
    Player:\n - speed {player_speed:.3}\n - position {player_pos:.3?}\n - infinite inertia: {inf_inert} (toggle with c)\n\
    Rigid Bodies: {rigid_body_count}, sleeping: {slepping_body_count}\n\
    ");
}

fn player_input_system(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,

    mut player_query: Query<(
        &mut Transform64,
        &mut CharacterControllerComp,
        &CharacterResultsComp,
    ), With<CharacterControllerComp>>,
    mut camera_query: Query<(
        &mut Transform64,
        &GlobalTransform64,
    ), Without<CharacterControllerComp>>,

    mut player: ResMut<Player>,

    mut mouse_move_events: EventReader<MouseMotion>,
    mut mouse_wheel_events: EventReader<MouseWheel>,

    kb_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,

    mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let mut window = q_windows.single_mut();

    let (
        mut player_transform,
        mut player_char_comp,
        _player_character_results,
    ) = player_query.get_mut(player.entity).unwrap();
    let (
        mut camera_transform,
        camera_global_transform,
    ) = camera_query.get_mut(player.camera_entity).unwrap();

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
            player.speed *= 0.9;
        }
        else if mwe.y > 0. {
            player.speed *= 1.1;
        }
    }

    if kb_input.just_pressed(KeyCode::Space) {
        player.velocity.y += 10.;
    }

    if kb_input.just_pressed(KeyCode::KeyC) {
        player.collide_with_rigid_bodies = !player.collide_with_rigid_bodies;
        if player.collide_with_rigid_bodies {
            player_char_comp.filter_flags = QueryFilterFlags::empty();
        }
        else {
            player_char_comp.filter_flags = QueryFilterFlags::EXCLUDE_DYNAMIC;
        }
    }

    if mouse_input.pressed(MouseButton::Left) {
        for me in mouse_move_events.read() {
            let mov = me.delta.as_dvec2() / -300.;

            player_transform.rotate_local_y(mov.x);
            camera_transform.rotate_local_x(mov.y);
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
                ..ColliderBundle::from(ColliderBuilder::ball(1.)
                    .mass(50.))
            },
            RigidBodyBundle {
                linvel: VelocityComp::new(camera_global_transform.forward() * 20.),
                ..RigidBodyBundle::dynamic()
            },
        )).insert(Transform64Bundle {
            local: Transform64::from_translation(player_transform.translation),
            ..default()
        });
    }

    let forward = player_transform.forward();
    let left = player_transform.left();

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
    let speed = player.speed;
    player.input_velocity = movement.normalize_or_zero() * speed;
}

fn player_physics_system(
    mut player_query: Query<&mut CharacterNextTranslationComp>,

    mut player: ResMut<Player>,

    time: Res<Time<Fixed>>,
) {
    let mut player_next_translation =
        player_query.get_mut(player.entity).unwrap();

    let sideway_vel = player.velocity * DVec3::new(1., 0., 1.);
    let vert_vel = player.velocity * DVec3::new(0., 1., 0.);

    player.velocity = vert_vel + sideway_vel.lerp(player.input_velocity, 0.2);
    player.velocity += DVec3::new(0., -9.8, 0.) * time.delta_seconds_f64();

    player_next_translation.next_translation =
         player.velocity * time.delta_seconds_f64();
}

fn player_after_physics_system(
    player_query: Query<&CharacterResultsComp>,

    mut player: ResMut<Player>,

    time: Res<Time<Fixed>>,
) {
    let results =
        player_query.get(player.entity).unwrap();

    player.velocity = results.translation() / time.delta_seconds_f64();
}
