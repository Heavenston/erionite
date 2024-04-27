use bevy::prelude::*;
use crate::*;

use rapier::dynamics::{RigidBodyHandle, RigidBodyType};

#[derive(getset::CopyGetters, Debug, Component, Clone)]
pub struct RigidBodyHandleComp {
    #[getset(get_copy = "pub")]
    pub(super) handle: RigidBodyHandle,
}

#[derive(Debug, Component, Clone)]
pub struct RigidBodyComp {
    pub kind: RigidBodyType,
    pub enabled: bool,
}

#[derive(Default, Debug, Component, Clone)]
pub struct RigidBodyDampingComp {
    pub angular: Float,
    pub linear: Float,
}

#[derive(getset::CopyGetters, Default, Debug, Component, Clone)]
pub struct RigidBodySleepingComp {
    pub can_sleep: bool,
    #[getset(get_copy = "pub")]
    pub(super) sleeping: bool,
}

#[derive(getset::CopyGetters, Default, Debug, Component, Clone)]
pub struct VelocityComp {
    #[getset(get_copy = "pub")]
    pub(super) linvel: Vector3,
}

#[derive(getset::CopyGetters, Default, Debug, Component, Clone)]
pub struct AngularVelocityComp {
    #[getset(get_copy = "pub")]
    pub(super) angvel: Vector3,
}
