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

pub trait VecConvert {
    fn to_rapier(&self) -> RapierVector3;
    fn to_bevy(&self) -> Vector3;
}

impl VecConvert for Vector3 {
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

impl VecConvert for RapierVector3 {
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
