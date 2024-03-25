use bevy_math::{bounding::Aabb3d, DVec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DAabb {
    /// Also min position, included
    pub position: DVec3,
    /// Position+size is included
    pub size: DVec3,
}

impl DAabb {
    pub fn from_minmax(min: DVec3, max: DVec3) -> Self {
        Self {
            position: min,
            size: max - min,
        }
    }

    pub fn min(&self) -> DVec3 {
        return self.position;
    }

    pub fn max(&self) -> DVec3 {
        return self.position + self.size;
    }

    pub fn set_min(&mut self, val: impl Into<DVec3>) {
        self.position = val.into();
    }

    pub fn set_max(&mut self, val: impl Into<DVec3>) {
        self.size = val.into() - self.position;
    }
}

impl Into<Aabb3d> for DAabb {
    fn into(self) -> Aabb3d {
        Aabb3d {
            min: self.min().as_vec3(),
            max: self.max().as_vec3(),
        }
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
