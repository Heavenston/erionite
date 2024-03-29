mod aabb;
use std::{ops::{Add, Range, Sub}, process::Output};

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

pub trait RangeExt<T> {
    type This<U>: RangeExt<U>;

    fn extent(&self) -> <T as Sub<T>>::Output
        where T: Sub<T> + Copy;
    
    fn clamped(&self, value: T) -> T
        where T: PartialOrd + Copy;

    fn range_map<O>(&self, f: impl Fn(&T) -> O) -> Self::This<O>;
    fn range_map_with<T2, O>(
        &self, other: &Self::This<T2>,
        f: impl Fn(&T, &T2) -> O
    ) -> Self::This<O>;

    fn range_sum(&self) -> T
        where T: Add<T, Output = T> + Copy;
}

impl<T> RangeExt<T> for Range<T> {
    type This<U> = Range<U>;

    fn extent(&self) -> <T as Sub<T>>::Output
        where T: Sub<T> + Copy
    {
        self.end - self.start
    }

    fn clamped(&self, value: T) -> T
        where T: PartialOrd + Copy
    {
        if value < self.start {
            return self.start;
        }
        if value > self.end {
            return self.end;
        }
        return value;
    }

    fn range_map<O>(&self, f: impl Fn(&T) -> O) -> Range<O> {
        Range {
            start: f(&self.start),
            end: f(&self.end),
        }
    }

    fn range_map_with<T2, O>(
        &self, other: &Range<T2>,
        f: impl Fn(&T, &T2) -> O
    ) -> Range<O> {
        Range {
            start: f(&self.start, &other.start),
            end: f(&self.end, &other.end),
        }
    }

    fn range_sum(&self) -> T
        where T: Add<T, Output = T> + Copy {
        self.start + self.end
    }
}
