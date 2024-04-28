pub mod systems;
use bevy::math::DQuat;
use rapier::na::{Quaternion, UnitQuaternion};
pub(crate) use systems::*;

mod plugin;
pub use plugin::*;

mod resources;
pub use resources::*;

mod collider;
pub use collider::*;

mod rigid_body;
pub use rigid_body::*;

pub use rapier3d_f64 as rapier;

pub type Float = rapier::math::Real;
pub type Vector3 = bevy::math::DVec3;
pub type RapierVector3 = rapier::math::Vector<Float>;

pub trait LibConvert {
    type RapierType: LibConvert<RapierType = Self::RapierType, BevyType = Self::BevyType>;
    type BevyType: LibConvert<RapierType = Self::RapierType, BevyType = Self::BevyType>;

    fn to_rapier(&self) -> Self::RapierType;
    fn to_bevy(&self) -> Self::BevyType;
}

impl LibConvert for Vector3 {
    type RapierType = RapierVector3;
    type BevyType = Vector3;

    fn to_rapier(&self) -> RapierVector3 {
        RapierVector3::new(
            self.x,
            self.y,
            self.z,
        )
    }

    fn to_bevy(&self) -> Vector3 {
        *self
    }
}

impl LibConvert for RapierVector3 {
    type RapierType = RapierVector3;
    type BevyType = Vector3;

    fn to_rapier(&self) -> RapierVector3 {
        *self
    }

    fn to_bevy(&self) -> Vector3 {
        Vector3::new(
            self.x,
            self.y,
            self.z,
        )
    }
}

impl LibConvert for DQuat {
    type RapierType = UnitQuaternion<Float>;
    type BevyType = DQuat;

    fn to_rapier(&self) -> Self::RapierType {
        UnitQuaternion::<Float>::new_normalize(Quaternion::from_vector(
            rapier::na::Vector4::new(
                self.x,
                self.y,
                self.z,
                self.w,
            )
        ))
    }

    fn to_bevy(&self) -> Self::BevyType {
        *self
    }
}

impl LibConvert for UnitQuaternion<Float> {
    type RapierType = UnitQuaternion<Float>;
    type BevyType = DQuat;

    fn to_rapier(&self) -> Self::RapierType {
        *self
    }

    fn to_bevy(&self) -> Self::BevyType {
        DQuat::from_xyzw(
            self.coords.x,
            self.coords.y,
            self.coords.z,
            self.coords.w,
        )
    }
}
