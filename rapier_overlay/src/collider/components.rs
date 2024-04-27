use bevy::prelude::*;

use crate::{rapier, Float};

use rapier::geometry::{ColliderHandle, SharedShape};

#[derive(Debug, Bundle, Clone)]
pub struct ColliderBundle {
    pub shape: ColliderShapeComp,
    pub friction: ColliderFrictionComp,
    pub mass: ColliderMassComp,
}

#[derive(getset::CopyGetters, Debug, Component, Clone)]
pub struct ColliderHandleComp {
    #[getset(get_copy = "pub")]
    pub(super) handle: ColliderHandle,
}

#[derive(Debug, Component, Clone)]
pub struct ColliderShapeComp {
    pub shape: SharedShape,
}

#[derive(Debug, Component, Clone)]
pub struct ColliderFrictionComp {
    pub friction: Float,
}

#[derive(Debug, Component, Clone)]
pub struct ColliderMassComp {
    pub mass: Float,
}

