mod aabb;
pub use aabb::*;
use bevy_math::BVec3;

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
