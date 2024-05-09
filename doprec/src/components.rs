use std::ops::Mul;

use bevy::prelude::*;
use bevy::math::{Affine3A, DAffine3, DQuat, DVec3};
use utils::DQuatExt;

// TODO: Gather info about why trans / rot / scale is separated for Transform and not
// for GlobalTransform
// (there are bevy issues (and rfcs?) about it)

#[derive(Bundle, Default)]
pub struct Transform64Bundle {
    pub local: Transform64,
    pub global: GlobalTransform64,
    pub bevy_local: Transform,
    pub bevy_global: GlobalTransform,
}

#[derive(Component)]
pub struct FloatingOrigin;

// Uses translation, rotation, scale instead of DAffine3 like bevy does 
// gives easy and intuitive access to the three properties
#[derive(Component, Debug, PartialEq, Clone, Copy)]
pub struct Transform64 {
    pub translation: DVec3,
    pub rotation: DQuat,
    pub scale: DVec3,
}

impl Default for Transform64 {
    fn default() -> Self {
        Self::IDENTITY
    }
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

    /// See bevy's Transform::looking_at
    #[inline]
    #[must_use]
    pub fn looking_at(mut self, target: DVec3, up: DVec3) -> Self {
        self.look_at(target, up);
        self
    }

    /// See bevy's Transform::look_at
    pub fn look_at(&mut self, target: DVec3, up: DVec3) {
        self.look_to(target - self.translation, up);
    }

    /// See bevy's Transform::look_to
    pub fn look_to(&mut self, direction: DVec3, up: DVec3) {
        self.rotation = DQuat::looking_at(direction, up);
    }

    pub fn rotate_local(&mut self, rotation: DQuat) {
        self.rotation *= rotation;
    }

    pub fn rotate_local_y(&mut self, angle: f64) {
        self.rotate_local(DQuat::from_rotation_y(angle));
    }

    pub fn rotate_local_x(&mut self, angle: f64) {
        self.rotate_local(DQuat::from_rotation_x(angle));
    }

    pub fn rotate_local_z(&mut self, angle: f64) {
        self.rotate_local(DQuat::from_rotation_z(angle));
    }

    pub fn local_x(&self) -> DVec3 {
        self.rotation * DVec3::X
    }

    pub fn local_y(&self) -> DVec3 {
        self.rotation * DVec3::Y
    }

    pub fn local_z(&self) -> DVec3 {
        self.rotation * DVec3::Z
    }

    pub fn forward(&self) -> DVec3 {
        -self.local_z()
    }

    pub fn back(&self) -> DVec3 {
        self.local_z()
    }

    pub fn left(&self) -> DVec3 {
        -self.local_x()
    }

    pub fn right(&self) -> DVec3 {
        self.local_x()
    }

    pub fn up(&self) -> DVec3 {
        self.local_y()
    }

    pub fn down(&self) -> DVec3 {
        -self.local_y()
    }

    pub fn inverse(&self) -> Self {
        Self {
            translation: -self.translation,
            rotation: self.rotation.inverse(),
            scale: 1. / self.scale,
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

impl From<GlobalTransform64> for Transform64 {
    fn from(value: GlobalTransform64) -> Self {
        let (scale, rotation, translation) = value.0.to_scale_rotation_translation();
        Transform64 { translation, rotation, scale }
    }
}

#[derive(Component, Debug, PartialEq, Clone, Copy)]
pub struct GlobalTransform64(DAffine3);

impl Default for GlobalTransform64 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl GlobalTransform64 {
    pub const IDENTITY: Self = Self(DAffine3::IDENTITY);

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

    pub fn rotation(&self) -> DQuat {
        self.0.to_scale_rotation_translation().1
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

    pub fn inverse(&self) -> Self {
        Self(self.0.inverse())
    }

    pub fn local_x(&self) -> DVec3 {
        self.rotation() * DVec3::X
    }

    pub fn local_y(&self) -> DVec3 {
        self.rotation() * DVec3::Y
    }

    pub fn local_z(&self) -> DVec3 {
        self.rotation() * DVec3::Z
    }

    pub fn forward(&self) -> DVec3 {
        -self.local_z()
    }

    pub fn back(&self) -> DVec3 {
        self.local_z()
    }

    pub fn left(&self) -> DVec3 {
        -self.local_x()
    }

    pub fn right(&self) -> DVec3 {
        self.local_x()
    }

    pub fn up(&self) -> DVec3 {
        self.local_y()
    }

    pub fn down(&self) -> DVec3 {
        -self.local_y()
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
