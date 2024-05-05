use bevy::{math::DVec3, prelude::*};

use crate::*;

#[derive(Resource)]
pub struct RapierConfig {
    pub gravity: Vector3,
}

impl Default for RapierConfig {
    fn default() -> Self {
        Self {
            gravity: DVec3::new(0., -9.8, 0.),
        }
    }
}
