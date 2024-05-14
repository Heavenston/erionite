#![feature(duration_millis_float)]

mod orbit_camera;

use std::{ops::Range, time::Instant};

use bevy::{
    core::TaskPoolThreadAssignmentPolicy, diagnostic::{Diagnostic, DiagnosticPath, Diagnostics, DiagnosticsStore, FrameTimeDiagnosticsPlugin, RegisterDiagnostic}, math::{DQuat, DVec3}, prelude::*, render::mesh::{SphereKind, SphereMeshBuilder}, tasks::available_parallelism, utils::HashSet
};
use doprec::{FloatingOrigin, Transform64, Transform64Bundle};
use rand::prelude::*;
use utils::{IsZeroApprox, Vec3Ext};

const COLLISION_DIAG: DiagnosticPath = DiagnosticPath::const_new("collision_compute");
const INTEGRATION_DIAG: DiagnosticPath = DiagnosticPath::const_new("velocity_compute");

fn main() {
    utils::logging::setup_basic_logging().unwrap();

    App::new()
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin)
        .add_plugins(bevy::diagnostic::LogDiagnosticsPlugin::default())

        .add_plugins((
            DefaultPlugins.build()
                .set(TaskPoolPlugin {
                    task_pool_options: TaskPoolOptions {
                        min_total_threads: available_parallelism() + 8,
                        compute: TaskPoolThreadAssignmentPolicy {
                            min_threads: available_parallelism(),
                            max_threads: available_parallelism(),
                            percent: 1.,
                        },
                        ..default()
                    },
                })
                .disable::<bevy::transform::TransformPlugin>()
                .disable::<bevy::log::LogPlugin>(),
            doprec::DoprecPlugin::default(),
            nbody::NBodyPlugin {
                enable_svo: true,
                ..nbody::NBodyPlugin::default()
            },
            orbit_camera::OrbitCameraPlugin::default(),
        ))

        .register_diagnostic(
            Diagnostic::new(COLLISION_DIAG)
                .with_suffix(" ms")
        )

        .add_systems(Startup, setup_system)
        .add_systems(Update, (
            update_particles_colors.run_if(|| false),
            update_debug_text_system,
        ))
        .add_systems(FixedUpdate, (
            particle_merge_system.run_if(|| false),
            position_integration_system,
        ).after(nbody::GravitySystems))

        .insert_resource(Time::<Fixed>::from_hz(60.0))
        
        .run();
}

#[derive(Component, Default, Debug, Clone, Copy, PartialEq)]
struct DebugTextComp;

#[derive(Component, Default, Debug, Clone, Copy, PartialEq)]
pub struct Particle {
    pub radius: f64,
}

#[derive(Component, Default, Debug, Clone, Copy, PartialEq)]
pub struct ParticleVelocity {
    pub velocity: DVec3,
}

#[derive(Resource, Debug, Clone)]
pub struct ParticleConfig {
    pub material: Handle<StandardMaterial>,
    pub mesh: Handle<Mesh>,
    pub density: f64,

    pub sun_mass: f64,

    pub distance_range: Range<f64>,
    pub mass_range: Range<f64>,
}

#[derive(Bundle, Debug)]
pub struct ParticleBundle {
    transform: Transform64Bundle,
    visibility: VisibilityBundle,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    particle: Particle,
    velocity: ParticleVelocity,
    gravity_field_sample: nbody::GravityFieldSample,
    massive: nbody::Massive,
    attracted: nbody::Attracted,
    attractor: nbody::Attractor,
}

impl ParticleBundle {
    pub fn new(
        cfg: &ParticleConfig,
        _meshes: &mut Assets<Mesh>,
        _materials: &mut Assets<StandardMaterial>,
        mass: f64,
        pos: DVec3,
        custom_radius: Option<f64>,
    ) -> Self {
        let radius = custom_radius.unwrap_or_else(||
            (3. * (mass / cfg.density)) / (4. * std::f64::consts::PI)
        );
        // let material = materials.add(materials.get(&cfg.material).unwrap().clone());
        let material = cfg.material.clone();

        Self {
            transform: Transform64Bundle {
                local: Transform64 {
                    translation: pos,
                    rotation: default(),
                    scale: DVec3::splat(radius),
                },
                ..default()
            },
            visibility: default(),
            mesh: cfg.mesh.clone(),
            material,
            particle: Particle { radius },
            velocity: default(),
            gravity_field_sample: default(),
            massive: nbody::Massive { mass },
            attracted: default(),
            attractor: default(),
        }
    }
}

fn spawn_particles(
    cfg: &ParticleConfig,
    gravity_cfg: &nbody::GravityConfig,
    mut commands: Commands,

    materials: &mut Assets<StandardMaterial>,
    meshes: &mut Assets<Mesh>,

    count: usize,
) {
    let mut rng = SmallRng::from_entropy();

    for _ in 0..count {
        let distance = rng.gen_range(cfg.distance_range.clone());
        let angle = rng.gen_range(-std::f64::consts::PI..std::f64::consts::PI);
        let mass = rng.gen_range(cfg.mass_range.clone());

        let pos = DQuat::from_rotation_y(angle) * DVec3::new(0., 0., 1.) * distance;
        let vel_norm = ((gravity_cfg.gravity_constant * cfg.sun_mass) / distance).sqrt();
        let velocity = pos.cross(DVec3::new(0., 1., 0.)).normalize() * vel_norm;

        commands
            .spawn(ParticleBundle {
                velocity: ParticleVelocity { velocity },
                ..ParticleBundle::new(
                    &cfg, meshes, materials,
                    mass, pos,
                    None,
                )
            });
    }
}

fn setup_system(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    gravity_cfg: Res<nbody::GravityConfig>,
) {
    let cfg = ParticleConfig {
        material: materials.add(StandardMaterial {
            base_color: Color::RED,
            unlit: true,
            ..default()
        }),
        mesh: meshes.add(SphereMeshBuilder::new(1., SphereKind::Ico {
            subdivisions: 4,
        }).build()),
        density: 100f64,

        sun_mass: 1_000_000_000.,

        distance_range: 2_000.0..2_500.0,
        mass_range: 100.0..10_000.,
    };
    commands.insert_resource(cfg.clone());
    
    commands
        .spawn(ParticleBundle::new(
            &cfg, &mut *meshes, &mut *materials,
            cfg.sun_mass, DVec3::ZERO,
            Some(1_000f64),
        ));

    spawn_particles(
        &cfg, &*gravity_cfg, commands.reborrow(),
        &mut *materials, &mut *meshes, 1_000,
    );

    // let mass = 1_000f64;
    // commands.spawn(ParticleBundle::new(&cfg, &mut *meshes, &mut *materials, mass, DVec3::new(
    //     -10., 0., 0.,
    // )));
    // commands.spawn(ParticleBundle::new(&cfg, &mut *meshes, &mut *materials, mass, DVec3::new(
    //     10., 0., 0.,
    // )));

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 1_000.,
    });
    
    // camera
    commands.spawn_empty()
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
            orbit_camera::OrbitCameraComp::default(),
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
            "",
            TextStyle {
                font_size: 15.0,
                ..default()
            },
        )).insert(DebugTextComp);
    }).set_parent(root_uinode);
}

fn update_debug_text_system(
    mut commands: Commands,

    diagnostics: Res<DiagnosticsStore>,
    cfg: Res<ParticleConfig>,
    mut gravity_cfg: ResMut<nbody::GravityConfig>,
    gravity_svo_ctx: Res<nbody::GravitySvoContext>,

    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,

    cam_query: Query<(&Transform64, &orbit_camera::OrbitCameraComp)>,
    particles_query: Query<&ParticleVelocity, With<Particle>>,

    kb_input: Res<ButtonInput<KeyCode>>,

    mut debug_text: Query<&mut Text, With<DebugTextComp>>,
) {
    let (cam_transform, orbit_cam) = cam_query.single();

    let fps = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|diag| diag.smoothed())
        .unwrap_or(0.);
    let frame_time = diagnostics.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|diag| diag.smoothed())
        .unwrap_or(0.);

    let grav_compute_duration = diagnostics.get(&nbody::GRAVITY_COMPUTE_SYSTEM_DURATION)
        .and_then(|diag| diag.smoothed())
        .unwrap_or(0.);
    let svo_update_duration = diagnostics.get(&nbody::GRAVITY_SVO_UPDATE_SYSTEM_DURATION)
        .and_then(|diag| diag.smoothed())
        .unwrap_or(0.);
    let collision_compute_duration = diagnostics.get(&COLLISION_DIAG)
        .and_then(|diag| diag.smoothed())
        .unwrap_or(0.);

    let cam_pos = cam_transform.translation;
    let cam_speed = orbit_cam.movement_speed;

    let particle_count = particles_query.iter().count();

    let svo_state = if gravity_cfg.enabled_svo {
        "enabled"
    } else {
        "disabled"
    };

    if kb_input.just_pressed(KeyCode::KeyS) {
        gravity_cfg.enabled_svo = !gravity_cfg.enabled_svo;
    }
    let theta_step = 0.05;
    if kb_input.just_pressed(KeyCode::NumpadAdd) {
        gravity_cfg.svo_skip_threshold += theta_step;
    }
    if kb_input.just_pressed(KeyCode::NumpadSubtract) {
        gravity_cfg.svo_skip_threshold -= theta_step;
        if gravity_cfg.svo_skip_threshold < 0. {
            gravity_cfg.svo_skip_threshold = 0.;
        }
    }
    if kb_input.just_pressed(KeyCode::KeyP) {
        spawn_particles(
            &cfg, &*gravity_cfg, commands.reborrow(),
            &mut *materials, &mut *meshes, 500,
        );
    }

    let svo_depth = gravity_svo_ctx.depth();
    let svo_max_depth = gravity_svo_ctx.max_depth();
    let svo_theta = gravity_cfg.svo_skip_threshold;

    let energy = particles_query.iter().map(|v| v.velocity.length()).sum::<f64>();

    let mut debug_text = debug_text.single_mut();
    debug_text.sections[0].value = format!("\
    {fps:.1} fps - {frame_time:.3} ms/frame\n\
    - Svo update: {svo_update_duration:.3} ms\n\
    - Gravity compute: {grav_compute_duration:.3} ms\n\
    - Collision detection: {collision_compute_duration:.3} ms\n\
    Camera: speed {cam_speed:.3}, position {cam_pos:.3?}\n\
    Particles: count {particle_count} (press 'p' to spawn more),\n   total energy: {energy:.2}\n\
    Svo: {svo_state} (press 's' to toggle), depth: {svo_depth}/{svo_max_depth}, theta: {svo_theta:.2} (+/- {theta_step})\n\
    ");
}

fn update_particles_colors(
    mut materials: ResMut<Assets<StandardMaterial>>,

    mut particle_query: Query<(&nbody::Attractor, &mut Handle<StandardMaterial>), With<Particle>>,
) {
    let min_color = Color::YELLOW.rgba_linear_to_vec4();
    let max_color = Color::RED.rgba_linear_to_vec4();

    let max_depth = particle_query.iter()
        .filter_map(|par| par.0.last_svo_position.as_ref().map(|p| p.depth()))
        .max().unwrap_or_default();

    for (attractor, mut material_handle) in &mut particle_query {
        let depth = attractor.last_svo_position.as_ref()
            .map(|p| p.depth()).unwrap_or(0);
        let prop = depth as f32 / max_depth as f32;

        let color = Color::rgba_linear_from_array(
            min_color * (1. - prop) + max_color * prop
        );

        material_handle.set_changed();
        let material = materials.get_mut(&*material_handle).unwrap();
        material.base_color = color;
    }
}

fn particle_merge_system(
    mut diagnostics: Diagnostics,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time<Fixed>>,

    cfg: Res<ParticleConfig>,

    particle_query: Query<(Entity, &Transform64, &ParticleVelocity, &nbody::GravityFieldSample, &nbody::Massive, &Particle)>,
) {
    let start = Instant::now();
    let mut destroyed = HashSet::<Entity>::new();

    for (
        entity, transform, velocity_comp, sample_comp, massive_comp, particle_comp,
    ) in &particle_query {
        let Some(nbody::AttractorInfo {
            entity: closest_entity, ..
        }) = sample_comp.closest_attractor()
        else { continue; };

        if destroyed.contains(&entity) || destroyed.contains(&closest_entity) {
            continue;
        }

        let Ok((
            _other_entity, other_transform, other_velocity_comp, _other_attracted_comp, other_massive_comp, other_particle_comp,
        )) = particle_query.get(closest_entity)
        else { continue; };

        let p1 = transform.translation;
        let v1 = velocity_comp.velocity;
        let r1 = particle_comp.radius;
        let m1 = massive_comp.mass;

        let p2 = other_transform.translation;
        let v2 = other_velocity_comp.velocity;
        let r2 = other_particle_comp.radius;
        let m2 = other_massive_comp.mass;

        let dp = p1 - p2;
        let dv = v1 - v2;

        if dv.is_zero_approx() {
            continue;
        }

        let t =
            -(dp * dv).array().into_iter().sum::<f64>() /
            dv.array().into_iter().map(|x| x*x).sum::<f64>();

        let t = t.clamp(
            -time.delta_seconds_f64(),
            time.delta_seconds_f64(),
        );

        let closest_distance_squared = (dp + t * dv).length_squared();

        let contact_distance = (r1 + r2) / 2.;

        if contact_distance.powi(2) < closest_distance_squared {
            // no collision
            continue;
        }

        destroyed.insert(entity);
        commands.entity(entity).despawn();
        destroyed.insert(closest_entity);
        commands.entity(closest_entity).despawn();

        let m3 = m1 + m2;
        let v3 = ((m1 * v1) + (m2 * v2)) / m3;
        let p3 = ((p1 + v1 * t) * m1 + (p2 + v2 * t) * m2) / m3;

        commands.spawn(ParticleBundle {
            velocity: ParticleVelocity { velocity: v3 },
            ..ParticleBundle::new(&cfg, &mut *meshes, &mut *materials, m3, p3, None)
        });
    }

    diagnostics.add_measurement(&COLLISION_DIAG, || start.elapsed().as_millis_f64())
}

fn position_integration_system(
    mut diagnostics: Diagnostics,
    time: Res<Time<Fixed>>,

    mut particle_query: Query<(&nbody::GravityFieldSample, &mut ParticleVelocity, &mut Transform64), With<Particle>>,
) {
    let start = Instant::now();
    particle_query.par_iter_mut().for_each(|(
        sample, mut velocity_comp, mut transform,
    )| {
        // leapfrog integration (i hope?)
        let dt = time.delta_seconds_f64();
        let v = velocity_comp.velocity;
        let p = transform.translation;
        let a = sample.previous_field_force;
        let na = sample.field_force;

        let np = p + v * dt + 0.5 * a * dt.powi(2);
        let nv = v + 0.5 * ( a + na ) * dt;

        transform.translation = np;
        velocity_comp.velocity = nv;
    });
    diagnostics.add_measurement(&INTEGRATION_DIAG, || start.elapsed().as_millis_f64())
}