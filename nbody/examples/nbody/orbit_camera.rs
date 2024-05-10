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

#[derive(Debug, Clone, Copy, derivative::Derivative)]
#[derivative(Default)]
pub struct RotateMode {
    /// Interpolated to target_distance and applied to the transform
    #[derivative(Default(value = "10."))]
    distance: f64,
    #[derivative(Default(value = "10."))]
    target_distance: f64,
    /// Interpolated to target_distance and applied to the transform
    rotation: DQuat,
    target_rotation: DQuat,
}

#[derive(Debug, Clone, Copy, derivative::Derivative)]
#[derivative(Default)]
pub struct MoveMode {
    
}

#[derive(Debug, Clone, Copy, derivative::Derivative)]
#[derivative(Default)]
pub enum CamMode {
    /// Rotate around middle point
    #[derivative(Default)]
    Rotate(RotateMode),
    /// "First cam" move
    Move(MoveMode),
}

impl CamMode {
    pub const ROTATE_BUTTON: MouseButton = MouseButton::Left;
    pub const MOVE_BUTTON: MouseButton = MouseButton::Right;

    pub fn activation_button(&self) -> MouseButton {
        match self {
            CamMode::Rotate(..) => Self::ROTATE_BUTTON,
            CamMode::Move(..) => Self::MOVE_BUTTON,
        }
    }
}

#[derive(Component, derivative::Derivative)]
#[derivative(Default)]
pub struct OrbitCameraComp {
    pub center_translation: DVec3,
    pub is_mouse_active: bool,
    pub mode: CamMode,

    #[derivative(Default(value = "10."))]
    pub movement_speed: f64,
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

    let mouse_sensitivity = 1. / 300.;
    let lerp_proportion = (time.delta_seconds_f64() * 10.).clamp(0., 1.);

    // let movement = {
    //     let forward = camera_transform.forward();
    //     let left = camera_transform.left();
    //     let mut movement = DVec3::ZERO;
    //     if kb_input.pressed(KeyCode::KeyW) || kb_input.pressed(KeyCode::KeyZ) {
    //         movement += forward;
    //     }
    //     if kb_input.pressed(KeyCode::KeyS) {
    //         movement -= forward;
    //     }
    //     if kb_input.pressed(KeyCode::KeyA) || kb_input.pressed(KeyCode::KeyQ) {
    //         movement += left;
    //     }
    //     if kb_input.pressed(KeyCode::KeyD) {
    //         movement -= left;
    //     }
    //     movement
    // };
    let mouse_move = mouse_move_events.read()
        .map(|event| event.delta)
        .sum::<Vec2>();

    let scroll = mouse_wheel_events.read()
        .map(|mwe| mwe.y)
        .sum::<f32>();

    // mode switch
    match &camera_comp.mode {
        CamMode::Rotate(_) => {
            if mouse_input.pressed(CamMode::MOVE_BUTTON) {
                camera_comp.mode = CamMode::Move(default());
            }
        },
        CamMode::Move(_) => {
            if mouse_input.pressed(CamMode::ROTATE_BUTTON) {
                camera_comp.mode = CamMode::Rotate(default());
            }
        },
    }

    camera_comp.is_mouse_active = mouse_input.pressed(
        camera_comp.mode.activation_button()
    );
    if camera_comp.is_mouse_active {
        window.cursor.grab_mode = CursorGrabMode::Confined;
        window.cursor.visible = false;
    }
    else {
        window.cursor.grab_mode = CursorGrabMode::None;
        window.cursor.visible = true;
    }

    let new_mode = match camera_comp.mode {
        CamMode::Rotate(mut mode) => {
            for _ in 0..(scroll.abs().floor() as u32) {
                if scroll < 0. {
                    mode.target_distance *= 1.05;
                }
                else {
                    mode.target_distance *= 0.95;
                }
            }

            if camera_comp.is_mouse_active {
                mode.target_rotation *= DQuat::from_rotation_x(
                    mouse_move.y as f64 * -mouse_sensitivity
                );
                mode.target_rotation *= DQuat::from_rotation_y(
                    mouse_move.x as f64 * -mouse_sensitivity
                );
            }

            mode.distance = mode.distance.lerp(
                mode.target_distance,
                lerp_proportion,
            );
            mode.rotation = mode.rotation.lerp(
                mode.target_rotation,
                lerp_proportion,
            );

            camera_transform.translation = camera_comp.center_translation
                + (mode.rotation * DVec3::NEG_Z) * mode.distance;
            let up = camera_transform.up();
            camera_transform.look_at(
                camera_comp.center_translation, up
            );

            CamMode::Rotate(mode)
        },
        CamMode::Move(mode) => {
            if scroll != 0. {
                if scroll > 0. {
                    camera_comp.movement_speed *= 1.1;
                }
                else {
                    camera_comp.movement_speed *= 0.9;
                }
            }

            CamMode::Move(mode)
        },
    };
    camera_comp.mode = new_mode;
}
