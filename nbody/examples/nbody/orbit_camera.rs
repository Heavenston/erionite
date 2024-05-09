use bevy::{input::mouse::{MouseMotion, MouseWheel}, math::{DQuat, DVec3}, prelude::*, window::{CursorGrabMode, PrimaryWindow}};
use doprec::Transform64;

#[derive(Default)]
pub struct OrbitCameraPlugin;

impl Plugin for OrbitCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            camera_system,
        ));
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CamMouseMode {
    #[default]
    Idle,
    /// Rotate around middle point
    Rotate,
    /// "First cam" move
    Move,
}

impl CamMouseMode {
    pub fn button(&self) -> MouseButton {
        match self {
            CamMouseMode::Idle => unreachable!(),
            CamMouseMode::Rotate => MouseButton::Left,
            CamMouseMode::Move => MouseButton::Right,
        }
    }
}

#[derive(Component)]
pub struct OrbitCameraComp {
    pub target: DVec3,
    pub mouse_mode: CamMouseMode,

    pub movement_speed: f64,

    /// Actual pos may be interpolated
    pub target_translation: DVec3,
    /// Actual rot may be interpolated
    pub target_rotation: DQuat,
}

impl Default for OrbitCameraComp {
    fn default() -> Self {
        Self {
            target: default(),
            mouse_mode: default(),

            movement_speed: 10.,

            target_translation: default(),
            target_rotation: default(),
        }
    }
}

fn camera_system(
    mut cam_query: Query<(
        &mut OrbitCameraComp, &mut Transform64
    )>,

    mut mouse_move_events: EventReader<MouseMotion>,
    mut mouse_wheel_events: EventReader<MouseWheel>,

    kb_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,

    time: Res<Time>,

    mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let Ok(mut window) = q_windows.get_single_mut()
    else { return };
    let Ok((mut camera_comp, mut camera_transform)) = cam_query.get_single_mut()
    else { return };

    camera_transform.translation = camera_transform.translation.lerp(
        camera_comp.target_translation, (time.delta_seconds_f64() * 10.).clamp(0., 1.)
    );
    camera_transform.rotation = camera_transform.rotation.lerp(
        camera_comp.target_rotation, (time.delta_seconds_f64() * 10.).clamp(0., 1.)
    );

    let cam_sensitivity = 1. / 300.;

    {
        let forward = camera_transform.forward();
        let left = camera_transform.left();
        let mut movement = DVec3::ZERO;
        if kb_input.pressed(KeyCode::KeyW) || kb_input.pressed(KeyCode::KeyZ) {
            movement += forward;
        }
        if kb_input.pressed(KeyCode::KeyS) {
            movement -= forward;
        }
        if kb_input.pressed(KeyCode::KeyA) || kb_input.pressed(KeyCode::KeyQ) {
            movement += left;
        }
        if kb_input.pressed(KeyCode::KeyD) {
            movement -= left;
        }
        let movement_speed = camera_comp.movement_speed;
        camera_comp.target_translation += movement.normalize_or_zero()
            * movement_speed * time.delta_seconds_f64();
    }

    for mwe in mouse_wheel_events.read() {
        let mut position_relative_to_center = camera_comp.target_translation - camera_comp.target;
        if mwe.y < 0. {
            position_relative_to_center *= 0.95;
        }
        else if mwe.y > 0. {
            position_relative_to_center *= 1.05;
        }
        camera_comp.target_translation = position_relative_to_center + camera_comp.target;
    }

    match camera_comp.mouse_mode {
        CamMouseMode::Idle => {
            if mouse_input.pressed(CamMouseMode::Rotate.button()) {
                camera_comp.mouse_mode = CamMouseMode::Rotate;

                window.cursor.grab_mode = CursorGrabMode::Confined;
                window.cursor.visible = false;

                return;
            }
            if mouse_input.pressed(CamMouseMode::Move.button()) {
                camera_comp.mouse_mode = CamMouseMode::Move;

                window.cursor.grab_mode = CursorGrabMode::Confined;
                window.cursor.visible = false;

                return;
            }
        },
        CamMouseMode::Rotate => {
            if !mouse_input.pressed(CamMouseMode::Rotate.button()) {
                camera_comp.mouse_mode = CamMouseMode::Idle;

                window.cursor.grab_mode = CursorGrabMode::None;
                window.cursor.visible = true;

                return;
            }

            let mut position_relative_to_center = camera_comp.target_translation - camera_comp.target;

            for me in mouse_move_events.read() {
                let mov = me.delta.as_dvec2() * -cam_sensitivity;

                position_relative_to_center =
                    DQuat::from_euler(EulerRot::YXZ, mov.x, mov.y, 0.)
                    * position_relative_to_center;
            }

            camera_comp.target_translation = position_relative_to_center + camera_comp.target;
        },
        CamMouseMode::Move => {
            if !mouse_input.pressed(CamMouseMode::Move.button()) {
                camera_comp.mouse_mode = CamMouseMode::Idle;

                window.cursor.grab_mode = CursorGrabMode::None;
                window.cursor.visible = true;

                return;
            }

            for me in mouse_move_events.read() {
                let mov = me.delta.as_dvec2() * -cam_sensitivity;

                camera_transform.rotate_local_y(mov.x);
                camera_transform.rotate_local_x(mov.y);
            }
        },
    }
}
