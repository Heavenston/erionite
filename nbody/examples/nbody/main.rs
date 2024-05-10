mod orbit_camera;

use bevy::{diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin}, math::DVec3, prelude::*, render::mesh::{SphereKind, SphereMeshBuilder}, utils::HashSet};
use doprec::{FloatingOrigin, Transform64, Transform64Bundle};
use rand::prelude::*;

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
        .add_systems(FixedUpdate, (
            particle_merge_system,
            gravity_to_velocities_system,
            apply_velocities_system,
        ).chain())
        
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
        mass: f64, pos: DVec3
    ) -> Self {
        let radius = (3. * (mass / cfg.density)) / (4. * std::f64::consts::PI);

        Self {
            transform: Transform64Bundle {
                local: Transform64::from_translation(pos),
                ..default()
            },
            visibility: default(),
            mesh: meshes.add(SphereMeshBuilder::new(radius as f32, SphereKind::Ico {
                subdivisions: 5,
            }).build()),
            material: cfg.material.clone(),
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
            base_color: Color::WHITE,
            emissive: Color::WHITE,
            ..default()
        }),
        density: 10f64,
    };
    commands.insert_resource(cfg.clone());
    
    // for _ in 0..2_000 {
    //     let mass = rng.gen_range(1_000f64..100_000.);

    //     let pos = DVec3::new(
    //         rng.gen_range(-10_000f64..10_000.),
    //         rng.gen_range(-10_000f64..10_000.),
    //         rng.gen_range(-10_000f64..10_000.),
    //     );

    //     commands.spawn(ParticleBundle::new(&cfg, &mut *meshes, mass, pos));
    // }
    let mass = 10.;
    commands.spawn(ParticleBundle::new(&cfg, &mut *meshes, mass, DVec3::new(
        -10., 0., 0.
    )));
    commands.spawn(ParticleBundle::new(&cfg, &mut *meshes, mass, DVec3::new(
        10., 0., 0.
    )));

    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 100_000.,
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

    let grav_compute_duration = diagnostics.get(&nbody::GRAVITY_COMPUTE_SYSTEM_DURATION)
        .and_then(|diag| diag.smoothed())
        .unwrap_or(0.);

    let cam_pos = cam_transform.translation;
    let cam_speed = orbit_cam.movement_speed;

    let mut debug_text = debug_text.single_mut();
    debug_text.sections[0].value = format!("\
    {fps:.1} fps - {frame_time:.3} ms/frame - {grav_compute_duration:.3} ms for gravity compute\n\
    Camera: speed {cam_speed:.3}, position {cam_pos:.3?}\n\
    ");
}

fn particle_merge_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,

    cfg: Res<ParticleConfig>,

    particle_query: Query<(Entity, &Transform64, &ParticleVelocity, &nbody::Attracted, &nbody::Massive, &Particle)>,
) {
    let mut destroyed = HashSet::<Entity>::new();
    for (
        entity, transform, velocity_comp, attracted_comp, massive_comp, particle_comp,
    ) in &particle_query {
        let Some(closest) = attracted_comp.closest_attractor()
        else { continue; };

        if destroyed.contains(&entity) || destroyed.contains(&closest.entity) {
            continue;
        }

        let Ok((
            _other_entity, other_transform, other_velocity_comp, _other_attracted_comp, other_massive_comp, other_particle_comp,
        )) = particle_query.get(closest.entity)
        else { continue; };

        let sumed_radius = particle_comp.radius + other_particle_comp.radius;

        if sumed_radius.powi(2) < closest.squared_distance {
            // no collision
            continue;
        }

        destroyed.insert(entity);
        commands.entity(entity).despawn();
        destroyed.insert(closest.entity);
        commands.entity(closest.entity).despawn();

        let pos = (transform.translation + other_transform.translation) / 2.;
        let mass = massive_comp.mass + other_massive_comp.mass;
        let velocity = velocity_comp.velocity + other_velocity_comp.velocity;

        commands.spawn(ParticleBundle {
            velocity: ParticleVelocity { velocity },
            ..ParticleBundle::new(&cfg, &mut *meshes, mass, pos)
        });
    }
}

fn gravity_to_velocities_system(
    time: Res<Time<Fixed>>,

    mut particle_query: Query<(&nbody::GravityFieldSample, &mut ParticleVelocity), With<Particle>>,
) {
    for (
        sample, mut velocity_comp
    ) in &mut particle_query {
        velocity_comp.velocity += sample.field_force * time.delta_seconds_f64();
    }
}

fn apply_velocities_system(
    time: Res<Time<Fixed>>,

    mut particle_query: Query<(&mut Transform64, &ParticleVelocity), With<Particle>>,
) {
    for (
        mut transform, velocity_comp
    ) in &mut particle_query {
        transform.translation += velocity_comp.velocity * time.delta_seconds_f64();
    }
}
