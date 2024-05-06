use bevy::prelude::*;
use crate::*;

use rapier::{
    control::{CharacterAutostep, CharacterLength, KinematicCharacterController},
    na::Unit,
};

#[derive(Debug, Clone, Default, Bundle)]
pub struct CharacterControllerBundle {
    pub comp: CharacterControllerComp,
    pub next_translation: CharacterNextTranslationComp,
    pub results: CharacterResultsComp,
}

/// See [rapier::control::KinematicCharacterController]
#[derive(Debug, Clone, Component)]
pub struct CharacterControllerComp {
    pub up: Vector3,
    pub offset: CharacterLength,
    pub slide: bool,
    pub autostep: Option<CharacterAutostep>,
    pub max_slope_climb_angle: Float,
    pub min_slope_slide_angle: Float,
    pub snap_to_ground: Option<CharacterLength>,
    pub normal_nudge_factor: Float,
}

impl Default for CharacterControllerComp {
    fn default() -> Self {
        Self::from(KinematicCharacterController::default())
    }
}

impl From<KinematicCharacterController> for CharacterControllerComp {
    fn from(value: KinematicCharacterController) -> Self {
        Self {
            up: value.up.to_bevy(),
            offset: value.offset,
            slide: value.slide,
            autostep: value.autostep,
            max_slope_climb_angle: value.max_slope_climb_angle,
            min_slope_slide_angle: value.min_slope_slide_angle,
            snap_to_ground: value.snap_to_ground,
            normal_nudge_factor: value.normal_nudge_factor,
        }
    }
}

impl Into<KinematicCharacterController> for &CharacterControllerComp {
    fn into(self) -> KinematicCharacterController {
        KinematicCharacterController {
            up: Unit::new_normalize(self.up.to_rapier()),
            offset: self.offset,
            slide: self.slide,
            autostep: self.autostep,
            max_slope_climb_angle: self.max_slope_climb_angle,
            min_slope_slide_angle: self.min_slope_slide_angle,
            snap_to_ground: self.snap_to_ground,
            normal_nudge_factor: self.normal_nudge_factor,
        }
    }
}

impl Into<KinematicCharacterController> for CharacterControllerComp {
    fn into(self) -> KinematicCharacterController {
        (&self).into()
    }
}

#[derive(Debug, Clone, Component, Default)]
pub struct CharacterNextTranslationComp {
    pub enabled: bool,
    pub next_translation: Vector3,
}

/// Set after applying the target translation
#[derive(getset::CopyGetters, Debug, Clone, Component, Default)]
pub struct CharacterResultsComp {
    pub(crate) on_ground: bool,
    pub(crate) is_sliding: bool,
}
