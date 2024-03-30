use crate::{AabbExt, GlamFloat, Vec3Ext};

pub struct EveryCubes<T: GlamFloat> {
    aabb: T::Aabb3d,
    cube_size: T::Vec3,
    current: T::Vec3,
}

impl<T: GlamFloat> Iterator for EveryCubes<T> {
    type Item = T::Vec3;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.current.z() > self.aabb.max().z() {
            return None;
        }

        let p = self.current;

        *self.current.x_mut() += self.cube_size.x();
        if self.current.x() > self.aabb.max().x() {
            *self.current.x_mut() = self.aabb.min().x();
            *self.current.y_mut() += self.cube_size.y();
        }
        if self.current.y() > self.aabb.max().y() {
            *self.current.y_mut() = self.aabb.min().y();
            *self.current.z_mut() += self.cube_size.z();
        }

        Some(p)
    }
}

#[inline]
pub fn every_cubes<T: GlamFloat>(aabb: T::Aabb3d, cube_size: T::Vec3) -> EveryCubes<T> {
    EveryCubes {
        aabb, cube_size,
        current: aabb.min()
    }
}

