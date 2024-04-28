use bevy::prelude::*;

use crate::{rapier, Float};

use rapier::geometry::{Collider, ColliderBuilder, ColliderHandle, SharedShape};

#[derive(Debug, Bundle, Clone)]
pub struct ColliderBundle {
    pub shape: ColliderShapeComp,
    pub friction: ColliderFrictionComp,
    pub mass: ColliderMassComp,
}

impl From<ColliderBuilder> for ColliderBundle {
    fn from(value: ColliderBuilder) -> Self {
        Self::from(value.build())
    }
}

impl From<Collider> for ColliderBundle {
    fn from(value: Collider) -> Self {
        Self {
            shape: ColliderShapeComp { shape: value.shared_shape().clone() },
            friction: ColliderFrictionComp { friction: value.friction() },
            mass: ColliderMassComp { mass: value.mass(), },
        }
    }
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

impl Default for ColliderFrictionComp {
    fn default() -> Self {
        Self {
            friction: ColliderBuilder::default_friction(),
        }
    }
}

#[derive(Debug, Component, Clone)]
pub struct ColliderMassComp {
    pub mass: Float,
}

impl Default for ColliderMassComp {
    fn default() -> Self {
        Self {
            mass: 1.,
        }
    }
}
