use bevy::{math::DVec3, prelude::*};
use utils::SmallVec;

#[derive(Component, Default, Debug, Clone, Copy, PartialEq)]
pub struct Massive {
    pub mass: f64,
}

/// Spatial entities with this component will have it updated with the
/// total gravital force of all Attractors on its position.
///
/// Actual gravity force applied on body should be field_force * body_mass
#[derive(getset::Getters, Component, Debug, Default, PartialEq, Clone)]
#[getset(get = "pub")]
pub struct GravityFieldSample {
    /// List of samples of the field force were the last one is the latest one
    /// the second-to-last one is the previous one etc... up to the limit set
    /// in the [GravityConfig]
    field_forces: SmallVec<[DVec3; 1]>,
    pub(crate) closest_attractor: Option<AttractorInfo>,
    /// Any attractor closer than this distance do not count for the field_force
    /// (still for closest_attractor)
    #[getset(skip)]
    pub min_affect_distance: f64,
}

impl GravityFieldSample {
    /// Sets [Self::min_affect_distance]
    pub fn with_min_affect_distance(self, value: f64) -> Self {
        Self {
            min_affect_distance: value,
            ..self
        }
    }

    pub(crate) fn new_field_force(&mut self, force: DVec3, count_limit: usize) {
        // Most of the time only one force will be removed so no perf problem
        // can arise from this not 'bulk' removing them
        while self.field_forces.len()+1 > count_limit {
            self.field_forces.remove(0);
        }
        self.field_forces.push(force);
    }

    /// Returns the nth latest computed force
    /// So 0 is the latest and 1 the previous one
    pub fn field_force(&self, go_back: usize) -> Option<DVec3> {
        if go_back >= self.field_forces.len() {
            None
        }
        else {
            Some(self.field_forces[self.field_forces.len() - go_back - 1])
        }
    }
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

/// Optional component that if added will make the current entity skip timesteps
#[derive(getset::CopyGetters, Component, Debug, Clone, Copy, derivative::Derivative)]
#[derivative(Default)]
#[getset(get_copy = "pub")]
pub struct TimeStep {
    /// 1 -> normal time steps
    /// 2 -> timesteps are twice as long so half of timesteps are skipped
    #[getset(skip)]
    #[derivative(Default(value = "1"))]
    pub multiplier: u32,
    pub(crate) offset: u32,
    /// Wether or not the last update didn't skip this entity
    pub(crate) last_updated: bool,
}
