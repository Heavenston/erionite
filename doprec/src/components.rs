use std::ops::{Deref, DerefMut, Mul};

use bevy::prelude::*;
use bevy::math::{Affine3, Affine3A, DAffine3, DQuat, DVec3};

#[derive(Component)]
pub struct FloatingOrigin;

// Uses translation, rotation, scale instead of DAffine3 like bevy does 
// gives easy and intuitive access to the three properties
#[derive(Component, PartialEq, Clone, Copy)]
pub struct Transform64 {
    pub translation: DVec3,
    pub rotation: DQuat,
    pub scale: DVec3,
}

impl Transform64 {
    pub const IDENTITY: Transform64 = Transform64 {
        translation: DVec3::ZERO,
        rotation: DQuat::IDENTITY,
        scale: DVec3::ONE,
    };

    pub fn from_translation(translation: DVec3) -> Self {
        Self {
            translation,
            ..Self::IDENTITY
        }
    }

    pub fn from_rotation(rotation: DQuat) -> Self {
        Self {
            rotation,
            ..Self::IDENTITY
        }
    }

    pub fn from_scale(scale: DVec3) -> Self {
        Self {
            scale,
            ..Self::IDENTITY
        }
    }

    /// Does not implement From<Transform> to prevent implicit precision loss
    pub fn from_32(transform: Transform) -> Self {
        Self {
            translation: transform.translation.as_dvec3(),
            rotation: transform.rotation.as_dquat(),
            scale: transform.scale.as_dvec3(),
        }
    }

    /// Does not implement Into<Transform> to prevent implicit precision loss
    pub fn as_32(&self) -> Transform {
        Transform {
            translation: self.translation.as_vec3(),
            rotation: self.rotation.as_quat(),
            scale: self.scale.as_vec3(),
        }
    }
}

impl Mul<DVec3> for Transform64 {
    type Output = DVec3;

    fn mul(self, point: DVec3) -> Self::Output {
        self.rotation * (point * self.scale) + self.translation
    }

}

impl Mul for Transform64 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            translation: self.translation + rhs.translation,
            rotation: self.rotation * rhs.rotation,
            scale: self.scale * rhs.scale,
        }
    }
}

// Uses DAffine3.. because that's why bevy uses, i guess because multiplication is
// faster ?
#[derive(Component, PartialEq, Clone, Copy)]
pub struct GlobalTransform64(DAffine3);

impl GlobalTransform64 {
    pub fn as_32(&self) -> GlobalTransform {
        Affine3A {
            matrix3: self.0.matrix3.as_mat3().into(),
            translation: self.0.translation.as_vec3().into(),
        }.into()
    }

    pub fn set_translation(&mut self, val: DVec3) {
        self.0.translation = val;
    }

    pub fn translation(&self) -> DVec3 {
        self.0.translation
    }

    pub fn from_translation(translation: DVec3) -> Self {
        Self(DAffine3::from_translation(translation))
    }

    pub fn from_rotation(rotation: DQuat) -> Self {
        Self(DAffine3::from_quat(rotation))
    }

    pub fn from_scale(scale: DVec3) -> Self {
        Self(DAffine3::from_scale(scale))
    }
}

impl Mul<DVec3> for GlobalTransform64 {
    type Output = DVec3;

    fn mul(self, point: DVec3) -> Self::Output {
        self.0.transform_point3(point)
    }

}

impl Mul<Transform64> for GlobalTransform64 {
    type Output = GlobalTransform64;

    fn mul(self, rhs: Transform64) -> Self::Output {
        self * Self::from(rhs)
    }
}

impl Mul for GlobalTransform64 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl Into<Transform64> for GlobalTransform64 {
    fn into(self) -> Transform64 {
        let (scale, rotation, translation) = self.0.to_scale_rotation_translation();
        Transform64 { translation, rotation, scale }
    }
}

impl From<DAffine3> for GlobalTransform64 {
    fn from(value: DAffine3) -> Self {
        Self(value)
    }
}

impl From<Transform64> for GlobalTransform64 {
    fn from(value: Transform64) -> Self {
        Self(DAffine3::from_scale_rotation_translation(
            value.scale,
            value.rotation,
            value.translation,
        ))
    }
}
