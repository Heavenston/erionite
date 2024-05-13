use bevy::{math::DVec3, prelude::*};

#[derive(Component, Default, Debug, Clone, Copy, PartialEq)]
pub struct Massive {
    pub mass: f64,
}

/// Spatial entities with this component will have it updated with the
/// total gravital force of all Attractors on its position.
///
/// Actual gravity force applied on body should be field_force * body_mass
#[derive(getset::CopyGetters, Component, Debug, Default, PartialEq, Clone, Copy)]
#[getset(get_copy = "pub")]
pub struct GravityFieldSample {
    /// Field force at previous time step
    pub previous_field_force: DVec3,
    /// Field force at current time step
    pub field_force: DVec3,
    pub closest_attractor: Option<AttractorInfo>,
}

#[derive(Component, Debug, Default, Clone)]
pub struct Attractor {
    pub last_svo_position: Option<svo::CellPath>,
}

#[derive(Debug, Clone, Copy,PartialEq)]
pub struct AttractorInfo {
    pub entity: Entity,
    pub force: f64,
    pub squared_distance: f64,
}

#[derive(getset::CopyGetters, Component, Debug, Default, Clone, Copy)]
#[getset(get_copy = "pub")]
pub struct Attracted;
