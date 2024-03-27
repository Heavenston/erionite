mod aabb;
pub use aabb::*;
mod every_cubes;
pub use every_cubes::*;
mod generic_glam;
pub use generic_glam::*;

use bevy_math::{bounding::Aabb3d, BVec3, Vec3};
use num_traits::Num;

pub trait AsBVecExt {
    type BVec;
    fn as_bvec(&self) -> Self::BVec;
}

impl AsBVecExt for arbitrary_int::u3 {
    type BVec = BVec3;
    
    fn as_bvec(&self) -> Self::BVec {
        BVec3::new(
            self.value() & 0b001 != 0,
            self.value() & 0b010 != 0,
            self.value() & 0b100 != 0,
        )
    }
}
