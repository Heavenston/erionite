#![feature(duration_millis_float)]

mod orbit_camera;

use std::time::{Duration, Instant};

use bevy::{diagnostic::{Diagnostic, DiagnosticPath, Diagnostics, DiagnosticsStore, FrameTimeDiagnosticsPlugin, RegisterDiagnostic}, math::DVec3, prelude::*, render::mesh::{SphereKind, SphereMeshBuilder}, time::common_conditions::on_timer, utils::HashSet};
use doprec::{FloatingOrigin, Transform64, Transform64Bundle};
use nbody::GravityConfig;
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
            update_particles_colors.run_if(on_timer(Duration::from_millis(100))),
            update_debug_text_system,
        ))
        .add_systems(FixedUpdate, (
            particle_merge_system,
            position_integration_system,
        ).after(nbody::GravitySystems))
        
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
    pub density: f64,
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
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<StandardMaterial>,
        mass: f64, pos: DVec3
    ) -> Self {
        let radius = (3. * (mass / cfg.density)) / (4. * std::f64::consts::PI);
        let material = materials.add(materials.get(&cfg.material).unwrap().clone());

        Self {
            transform: Transform64Bundle {
                local: Transform64::from_translation(pos),
                ..default()
            },
            visibility: default(),
            mesh: meshes.add(SphereMeshBuilder::new(radius as f32, SphereKind::Ico {
                subdivisions: 5,
            }).build()),
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

fn setup_system(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let mut rng = SmallRng::from_entropy();

    let cfg = ParticleConfig {
        material: materials.add(StandardMaterial {
            base_color: Color::RED,
            unlit: true,
            ..default()
        }),
        density: 1_000f64,
    };
    commands.insert_resource(cfg.clone());
    
    for _ in 0..1_500 {
        let mass = rng.gen_range(1_000f64..100_000.);

        let pos = DVec3::new(
            rng.gen_range(-10_000f64..10_000.),
            rng.gen_range(-10_000f64..10_000.),
            rng.gen_range(-10_000f64..10_000.),
        );

        commands.spawn(ParticleBundle::new(
            &cfg, &mut *meshes, &mut *materials, mass, pos
        ));
    }
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
    diagnostics: Res<DiagnosticsStore>,
    mut gravity_cfg: ResMut<GravityConfig>,

    cam_query: Query<(&Transform64, &orbit_camera::OrbitCameraComp)>,
    particles_query: Query<(), With<Particle>>,

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

    let mut debug_text = debug_text.single_mut();
    debug_text.sections[0].value = format!("\
    {fps:.1} fps - {frame_time:.3} ms/frame\n\
    - Svo update: {svo_update_duration:.3} ms\n\
    - Gravity compute: {grav_compute_duration:.3} ms\n\
    - Collision detection: {collision_compute_duration:.3} ms\n\
    Camera: speed {cam_speed:.3}, position {cam_pos:.3?}\n\
    Particles: cout {particle_count}\n\
    Svo: {svo_state} (press 's' to toggle)\n\
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

    particle_query: Query<(Entity, &Transform64, &ParticleVelocity, &nbody::Attracted, &nbody::Massive, &Particle)>,
) {
    let start = Instant::now();
    let mut destroyed = HashSet::<Entity>::new();

    for (
        entity, transform, velocity_comp, attracted_comp, massive_comp, particle_comp,
    ) in &particle_query {
        let Some(nbody::AttractorInfo {
            entity: closest_entity, ..
        }) = attracted_comp.closest_attractor()
        else { continue; };

        if destroyed.contains(&entity) || destroyed.contains(&closest_entity) {
            continue;
        }

        let Ok((
            _other_entity, other_transform, other_velocity_comp, _other_attracted_comp, other_massive_comp, other_particle_comp,
        )) = particle_query.get(closest_entity)
        else { continue; };

        let dp = transform.translation - other_transform.translation;
        let dv = velocity_comp.velocity - other_velocity_comp.velocity;

        if dv.is_zero_approx() {
            continue;
        }

        let closest_approach_time =
            -(dp * dv).array().into_iter().sum::<f64>() /
            dv.array().into_iter().map(|x| x*x).sum::<f64>();

        let closest_approach_time = closest_approach_time.clamp(
            -time.delta_seconds_f64(),
            time.delta_seconds_f64(),
        );

        let closest_distance_squared = (dp + closest_approach_time * dv).length_squared();

        let sumed_radius = (particle_comp.radius + other_particle_comp.radius) / 2.;

        if sumed_radius.powi(2) < closest_distance_squared {
            // no collision
            continue;
        }

        destroyed.insert(entity);
        commands.entity(entity).despawn();
        destroyed.insert(closest_entity);
        commands.entity(closest_entity).despawn();

        let pos = (transform.translation + other_transform.translation) / 2.;
        let mass = massive_comp.mass + other_massive_comp.mass;
        let velocity = velocity_comp.velocity + other_velocity_comp.velocity;

        commands.spawn(ParticleBundle {
            velocity: ParticleVelocity { velocity },
            ..ParticleBundle::new(&cfg, &mut *meshes, &mut *materials, mass, pos)
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
