#![feature(duration_millis_float)]

pub(crate) mod plugin;
use bevy::diagnostic::DiagnosticPath;
pub use plugin::*;
pub(crate) mod systems;
pub(crate) mod components;
pub use components::*;

pub use systems::TransformSystems;

pub const TRANSFORM_SYSTEMS_DURATION_DIAG: DiagnosticPath = DiagnosticPath::const_new("transform64_systems");
