#![allow(incomplete_features)]
#![feature(generic_const_exprs)]
#![feature(maybe_uninit_uninit_array)]
#![feature(maybe_uninit_array_assume_init)]
#![feature(maybe_uninit_write_slice)]

mod aabb;
pub use aabb::*;
mod every_cubes;
pub use every_cubes::*;
mod generic_glam;
pub use generic_glam::*;
#[cfg(feature = "logging")]
pub mod logging;
mod is_zero_approx;
pub use is_zero_approx::*;

pub use replace_with::replace_with_or_abort as replace_with;
pub use bimap::BiHashMap;

use bevy_math::{BVec3, DMat3, DQuat, DVec3, UVec3};
use std::{mem::{ManuallyDrop, MaybeUninit}, ops::{Add, Range, Sub}};

/// Copies the content of given arrays into a new bigger array.
///
/// # Example
/// ```
/// assert_eq!(
///    utils::join_arrays([0; 3], [1; 3]).map(|x| x*2).as_slice(),
///    [0,0,0,2,2,2].as_slice(),
/// );
/// assert_eq!(
///    utils::join_arrays([0; 3], [1; 3]).as_slice(),
///    [0,0,0,1,1,1].as_slice(),
/// );
/// assert_eq!(
///    utils::join_arrays([1,2], [-1; 4]).as_slice(),
///    [1,2,-1,-1,-1,-1].as_slice(),
/// );
/// assert_eq!(
///    utils::join_arrays([0; 0], [-1; 4]).as_slice(),
///    [-1,-1,-1,-1].as_slice(),
/// );
/// assert_eq!(
///    utils::join_arrays([1,2,3], [-1; 0]).as_slice(),
///    [1,2,3].as_slice(),
/// );
/// ```
pub fn join_arrays<T, const AS: usize, const BS: usize>(
    a: [T; AS],
    b: [T; BS],
) -> [T; AS + BS]
    where T: Copy
{
    let mut out = MaybeUninit::uninit_array();

    MaybeUninit::copy_from_slice(&mut out[..AS], &a);
    MaybeUninit::copy_from_slice(&mut out[AS..], &b);

    unsafe { MaybeUninit::array_assume_init(out) }
}

pub fn box_to_array<T, const SIZE: usize>(
    slice: Box<[T]>,
) -> Result<[T; SIZE], Box<[T]>> {
    if slice.len() != SIZE {
        return Err(slice);
    }

    unsafe {
        let slice = ManuallyDrop::new(slice);
        let mut out = MaybeUninit::<T>::uninit_array::<SIZE>();
        for i in 0..SIZE {
            out[i].write(std::ptr::read(&slice[i]));
        }
        Ok(MaybeUninit::array_assume_init(out))
    }
}

pub trait AsVecExt {
    fn as_bvec(&self) -> BVec3;
    fn as_uvec(&self) -> UVec3;
}

impl AsVecExt for arbitrary_int::u3 {
    fn as_bvec(&self) -> BVec3 {
        BVec3::new(
            self.value() & 0b001 != 0,
            self.value() & 0b010 != 0,
            self.value() & 0b100 != 0,
        )
    }
    fn as_uvec(&self) -> UVec3 {
        UVec3::new(
            if self.value() & 0b001 != 0 { 1 } else { 0 },
            if self.value() & 0b010 != 0 { 1 } else { 0 },
            if self.value() & 0b100 != 0 { 1 } else { 0 },
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
        value
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

pub trait DQuatExt {
    fn looking_at(direction: DVec3, up: DVec3) -> DQuat;
}

impl DQuatExt for DQuat {
    fn looking_at(direction: DVec3, up: DVec3) -> DQuat {
        let back = -direction.try_normalize().unwrap_or(DVec3::NEG_Z);
        let up = up.try_normalize().unwrap_or(DVec3::Y);
        let right = up
            .cross(back)
            .try_normalize()
            .unwrap_or_else(|| up.any_orthonormal_vector());
        let up = back.cross(right);
        DQuat::from_mat3(&DMat3::from_cols(right, up, back))
    }
}
