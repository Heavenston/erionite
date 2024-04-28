use bevy::prelude::*;
use crate::*;

use rapier::dynamics::{RigidBodyHandle, RigidBodyType};

#[derive(Debug, Bundle, Clone)]
pub struct RigidBodyBundle {
    pub rigid_body: RigidBodyComp,
    pub damping: RigidBodyDampingComp,
    pub sleeping: RigidBodySleepingComp,
    pub linvel: VelocityComp,
    pub angvel: AngularVelocityComp,
    pub external_force: ExternalForceComp,
}

impl RigidBodyBundle {
    pub fn new(kind: RigidBodyType) -> Self {
        Self {
            rigid_body: RigidBodyComp {
                kind,
                enabled: true,
            },
            damping: default(),
            sleeping: default(),
            linvel: default(),
            angvel: default(),
            external_force: default(),
        }
    }

    pub fn dynamic() -> Self {
        Self::new(RigidBodyType::Dynamic)
    }

    pub fn fixed() -> Self {
        Self::new(RigidBodyType::Fixed)
    }
}

#[derive(getset::CopyGetters, Default, Debug, Component, Clone)]
pub struct RigidBodyHandleComp {
    #[getset(get_copy = "pub")]
    pub(crate) handle: RigidBodyHandle,
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

#[derive(getset::CopyGetters, Debug, Component, Clone)]
pub struct RigidBodySleepingComp {
    pub can_sleep: bool,
    #[getset(get_copy = "pub")]
    pub(crate) sleeping: bool,
}

impl Default for RigidBodySleepingComp {
    fn default() -> Self {
        Self {
            can_sleep: true,
            sleeping: false,
        }
    }
}

impl RigidBodySleepingComp {
    pub fn new(can_sleep: bool) -> Self {
        Self {
            can_sleep,
            ..default()
        }
    }
}

#[derive(getset::CopyGetters, Default, Debug, Component, Clone)]
pub struct VelocityComp {
    #[getset(get_copy = "pub")]
    pub(crate) linvel: Vector3,
}

impl VelocityComp {
    pub fn new(linvel: Vector3) -> Self {
        Self {
            linvel,
        }
    }
}

#[derive(getset::CopyGetters, Default, Debug, Component, Clone)]
pub struct AngularVelocityComp {
    #[getset(get_copy = "pub")]
    pub(crate) angvel: Vector3,
}

#[derive(Default, Debug, Component, Clone)]
pub struct ExternalForceComp {
    pub force: Vector3,
    pub torque: Vector3,
}
