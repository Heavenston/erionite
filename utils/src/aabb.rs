use arbitrary_int::u3;
use bevy_math::{bounding::Aabb3d, DVec3};
use bevy_render::primitives::Aabb;

use crate::{AabbExt, AsVecExt};

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub struct DAabb {
    /// Also min position, included
    pub position: DVec3,
    /// Position+size is included
    pub size: DVec3,
}

impl DAabb {
    pub fn new_center_size(center: DVec3, size: DVec3) -> Self {
        Self {
            position: center - size / 2.,
            size,
        }
    }

    pub fn from_minmax(min: DVec3, max: DVec3) -> Self {
        Self {
            position: min,
            size: max - min,
        }
    }

    pub fn min(&self) -> DVec3 {
        self.position
    }

    pub fn max(&self) -> DVec3 {
        self.position + self.size
    }

    pub fn set_min(&mut self, val: impl Into<DVec3>) {
        self.position = val.into();
    }

    pub fn set_max(&mut self, val: impl Into<DVec3>) {
        self.size = val.into() - self.position;
    }

    pub fn corners(&self) -> [DVec3; 8] {
        let mut out = [DVec3::ZERO; 8];

        for (i, comp) in (0..0b111u8).enumerate() {
            let dx = if comp & 0b001 == 0 { 0. } else { 1. };
            let dy = if comp & 0b010 == 0 { 0. } else { 1. };
            let dz = if comp & 0b100 == 0 { 0. } else { 1. };
            out[i] = DVec3::new(
                self.position.x + self.size.x * dx,
                self.position.y + self.size.y * dy,
                self.position.z + self.size.z * dz,
            );
        }

        out
    }

    pub fn translated(self, diff: DVec3) -> Self {
        Self {
            position: self.position + diff,
            ..self
        }
    }

    pub fn fully_contained_in_sphere(self, sphere_origin: DVec3, sphere_radius: f64) -> bool {
        let r2 = sphere_radius.powi(2);
        self.translated(-sphere_origin).corners()
            .into_iter().all(|c| c.length_squared() <= r2)
    }

    /// Returns true if the aabb is touching, or is inside the sphere
    pub fn touching_sphere(self, sphere_origin: DVec3, sphere_radius: f64) -> bool {
        let point = self.closest_point(sphere_origin);

        point.length_squared() <= sphere_radius.powi(2)
    }

    /// Returns true if the aabb is touching, but not fully inside the sphere
    pub fn is_touching_sphere_surface(self, sphere_origin: DVec3, sphere_radius: f64) -> bool {
        self.touching_sphere(sphere_origin, sphere_radius)
            && (!self.fully_contained_in_sphere(sphere_origin, sphere_radius))
    }

    pub fn expand_to_contain_aabb(&mut self, aabb: DAabb) {
        self.set_min(DVec3::min(self.min(), aabb.min()));
        self.set_max(DVec3::max(self.max(), aabb.max()));
    }

    pub fn expand_to_contain_point(&mut self, point: DVec3) {
        self.set_min(DVec3::min(self.min(), point));
        self.set_max(DVec3::max(self.max(), point));
    }

    pub fn octdivide(&mut self, comp: u3) {
        self.size /= 2.;
        self.position += comp.as_uvec().as_dvec3() * self.size;
    }

    pub fn octdivided(mut self, comp: u3) -> Self {
        self.size /= 2.;
        self.position += comp.as_uvec().as_dvec3() * self.size;
        self
    }
}

impl From<DAabb> for Aabb3d {
    fn from(val: DAabb) -> Self {
        Aabb3d {
            min: val.min().as_vec3(),
            max: val.max().as_vec3(),
        }
    }
}

impl From<DAabb> for Aabb {
    fn from(val: DAabb) -> Self {
        Aabb::from_min_max(val.min().as_vec3(), val.max().as_vec3())
    }
}

impl From<Aabb3d> for DAabb {
    fn from(value: Aabb3d) -> Self {
        Self {
            position: value.min.as_dvec3(),
            size: value.max.as_dvec3() - value.min.as_dvec3(),
        }
    }
}
