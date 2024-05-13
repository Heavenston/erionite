mod components;
pub use components::*;
mod systems;
pub use systems::*;
mod tree_acceleration;
use tree_acceleration::*;
mod resources;
pub use resources::*;

use bevy::diagnostic::DiagnosticPath;

pub const GRAVITY_COMPUTE_SYSTEM_DURATION: DiagnosticPath =
    DiagnosticPath::const_new("gravity_compute");
pub const GRAVITY_SVO_UPDATE_SYSTEM_DURATION: DiagnosticPath =
    DiagnosticPath::const_new("svo_update_compute");

/// If set to true, when visiting the svo, cells that contains the current particle
/// will always be visited
const FORCE_VISIT_OWN_CELLS: bool = true;

/// If an svo has more than this amount of particles it is splitted if the
/// max depth has not been reached
const SVO_LEAF_MAX_PARTICLE_COUNT: usize = 10;
