use std::iter::FusedIterator;

use arbitrary_int::*;
use bevy_math::{DVec3, UVec3};
use utils::{ AsVecExt, DAabb, GlamFloat, Vec3Ext };

type CellPathInner = u64;

/// Represent a path on the stack by packing a u3 array into a number with
/// a leading 1 bit as terminator
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct CellPath(CellPathInner);
impl CellPath {
    pub const MAX_CAPACITY: u32 = CellPathInner::BITS.div_floor(3);

    pub fn new() -> Self {
        Self(0x1)
    }

    pub const fn capacity(&self) -> u32 {
        Self::MAX_CAPACITY
    }

    const fn mark_bit_position(&self) -> u32 {
        debug_assert!(
            self.0.leading_zeros() < CellPathInner::BITS,
            "invalid inner value"
        );
        let sb = CellPathInner::BITS - self.0.leading_zeros() - 1;
        debug_assert!(sb % 3 == 0, "invalid inner value");
        sb
    }

    #[doc(alias = "depth")]
    pub const fn len(&self) -> u32 {
        return self.mark_bit_position() / 3;
    }

    #[doc(alias = "len")]
    pub const fn depth(&self) -> u32 {
        self.len()
    }

    pub fn push(&mut self, v: u3) {
        assert!(self.capacity() > self.len());

        self.0 <<= 3;
        self.0 |= CellPathInner::from(v.value());
    }

    pub fn with_push(mut self, v: u3) -> Self {
        self.push(v);
        self
    }

    pub fn push_back(&mut self, v: u3) {
        assert!(self.capacity() > self.len());

        let marker_bit = self.mark_bit_position();
        // let x = marker_bit - 3;

        // remove marker bit
        self.0 &= !(CellPathInner::MAX << marker_bit);
        // add new val
        self.0 |= CellPathInner::from(v.value()) << marker_bit;
        // add new marker bit
        self.0 |= 1 << marker_bit + 3;
    }

    pub fn with_push_back(mut self, v: u3) -> Self {
        self.push_back(v);
        self
    }

    pub fn peek(&self) -> Option<u3> {
        if self.len() == 0 {
            return None;
        }

        let marker_bit = self.mark_bit_position();
        Some(u3::new((self.0 >> ((marker_bit - 3) as usize)) as u8))
    }

    pub fn pop(&mut self) -> Option<u3> {
        let marker_bit = self.mark_bit_position();

        if marker_bit == 0 {
            return None;
        }

        let x = marker_bit - 3;
        let val = unsafe { u3::new_unchecked(((self.0 >> x) & 0b111) as u8) };

        // remove last bits
        self.0 &= !(CellPathInner::MAX << x);
        // add marker bit
        self.0 |= 1 << x;

        Some(val)
    }

    pub fn pop_back(&self) -> Option<u3> {
        let len = self.len() as usize;
        if len == 0
        { return None; }

        Some(u3::new((self.0 >> ((len-1) * 3) & 0b111) as u8))
    }
    
    pub fn parent(&self) -> Option<Self> {
        if self.len() == 0
        { return None; }

        Some(Self(self.0 >> 3))
    }

    /// Returns an iterator over all parents, from the deepest to the root
    /// excluding self
    pub fn parents(&self) -> impl Iterator<Item = Self> {
        let mut current = self.clone();
        std::iter::from_fn(move || { current = current.parent()?; Some(current.clone()) })
    }

    pub fn get_aabb(&self, mut root: DAabb) -> DAabb {
        for x in self {
            root.size /= 2.;
            root.position = DVec3::select(
                x.as_bvec(),
                root.position + root.size, root.position
            );
        }
        root
    }

    /// Get the position of the cell considering one unit per cell of the current
    /// depth
    /// # Example:
    /// ```
    /// use arbitrary_int::u3;
    /// use bevy_math::UVec3;
    /// use svo::CellPath;
    ///
    /// assert_eq!(CellPath::new().get_pos(), UVec3::new(0,0,0));
    /// assert_eq!(
    ///     CellPath::new()
    ///         .with_push(u3::new(0b000))
    ///         .get_pos(),
    ///     UVec3::new(0,0,0),
    /// );
    /// assert_eq!(
    ///     CellPath::new()
    ///         .with_push(u3::new(0b011))
    ///         .get_pos(),
    ///     UVec3::new(1,1,0),
    /// );
    /// assert_eq!(
    ///     CellPath::new()
    ///         .with_push(u3::new(0b010))
    ///         .with_push(u3::new(0b100))
    ///         .get_pos(),
    ///     UVec3::new(0,2,1),
    /// );
    /// ```
    pub fn get_pos(&self) -> UVec3 {
        let mut size = UVec3::splat(2u32.pow(self.depth()));
        let mut result = UVec3::ZERO;
        for x in self {
            size /= 2;
            result += size * x.as_uvec();
        }
        result
    }

    pub fn neighbor(&self, dx: i8, dy: i8, dz: i8) -> Option<Self> {
        assert!(
            dx >= -1 && dx <= 1 &&
            dy >= -1 && dy <= 1 &&
            dz >= -1 && dz <= 1
        );

        let mut new = self.clone();

        for (d, i) in [(dx, 0), (dy, 1), (dz, 2)].into_iter() {
            if d == 0
            { continue; }
            let mut diff: CellPathInner = 0;
            let bit: CellPathInner = 1 << i;
            loop {
                if (diff / 3) >= self.len().into() {
                    return None;
                }
                // HAHAHAHAHA
                if (d == 1 && (new.0 >> diff) & bit == 0) ||
                    (d == -1 && (new.0 >> diff) & bit != 0) {
                    if d == 1 {
                        new.0 |= bit << diff;
                    }
                    else {
                        new.0 &= !(bit << diff);
                    }
                    break;
                }
                else {
                    if d == 1 {
                        new.0 &= !(bit << diff);
                    }
                    else {
                        new.0 |= bit << diff;
                    }
                    diff += 3;
                }
            }
        }

        Some(new)
    }

    /// Iterator over all neighbors of this path, excluding itself
    pub fn neighbors(self) -> impl Iterator<Item = ((i8, i8, i8), Self)> + DoubleEndedIterator {
        (-1..=1).flat_map(move |x| (-1..=1)
            .flat_map(move |y| (-1..=1)
            .map(move |z| (x, y, z))))
            // excluding itself
            .filter(|&(x, y, z)| x != 0 || y != 0 || z != 0)
            .filter_map(move |(x, y, z)| self.neighbor(x, y, z)
                .map(|xx| ((x, y, z), xx)))
    }

    pub const fn components() -> [u3; 8] {
        [
            u3::new(0b000), u3::new(0b001), u3::new(0b010), u3::new(0b011),
            u3::new(0b100), u3::new(0b101), u3::new(0b110), u3::new(0b111),
        ]
    }

    pub fn children(&self) -> [Self; 8] {
        Self::components().map(|p| self.clone().with_push(p))
    }

    /// Returns an iterator over all paths possible with the given depth
    pub fn all_iter(depth: u32) -> impl Iterator<Item = Self> + DoubleEndedIterator {
        let sections = depth * 3;
        (0..(1 << sections)).map(move |i| Self(i | (1 << sections)))
    }

    /// Returns a number representation of the current path, unique
    /// for all path of the same depth and occuppy the whole range from
    /// 0 to 2^(3 * depth)
    /// Can be use to *index* (wink) into an array
    /// Note that it can only work for paths of the same depth, collisions can
    /// occure between paths of different depths
    pub fn index(&self) -> CellPathInner {
        let marker_bit = self.mark_bit_position();

        self.0 & !(CellPathInner::MAX << marker_bit)
    }

    pub fn from_index(index: CellPathInner, depth: u32) -> Self {
        if depth*3 > Self::MAX_CAPACITY {
            panic!("Depth higher than capacity");
        }

        Self(index | (1 << depth as CellPathInner * 3))
    }

    /// Return a new CellPath with only the first [depth] elements of self
    /// Panics if [depth] is higher than [len](Self::len)
    /// the exact inverse of [reparent]
    pub fn take(&self, depth: u32) -> Self {
        assert!(depth <= self.len());
        let to_remove = self.len() - depth;
        Self(self.0 >> (3 * to_remove))
    }

    /// Return a new CellPath with the first [depth_to_remove] elements of self removed
    /// so with only the last (len - depth_to_remove) elements remaining
    /// Panics if [depth_to_remove] is higher than [len](Self::len)
    /// the inverse operation of [take_depth]
    pub fn reparent(self, depth_to_remove: u32) -> Self {
        assert!(depth_to_remove <= self.len());
        // mask of used-bits for the original path
        let full_mask = 1 << (self.len() * 3) - 1;
        let new_mask = full_mask >> depth_to_remove;

        let new_depth = self.len() * depth_to_remove;
        let new_end_bit = 1 << new_depth;
        Self((self.0 & new_mask) | new_end_bit)
    }

    pub fn extend(&mut self, other: &Self) {
        assert!(Self::MAX_CAPACITY > self.len() + other.len());
        self.0 = (self.0 << (other.len() * 3)) | other.index();
    }

    pub fn extended(mut self, other: &Self) -> Self {
        self.extend(other);
        self
    }

    pub fn in_unit_cube<T>(depth: u32, mut coords: T::Vec3) -> Option<Self>
        where T: GlamFloat
    {
        if coords.x() < T::new(0.) || coords.x() > T::new(1.)
        || coords.y() < T::new(0.) || coords.y() > T::new(1.)
        || coords.z() < T::new(0.) || coords.z() > T::new(1.) {
            return None;
        }

        let mut result = CellPath::new();
        for _ in 0..depth {
            let dd = coords.array_mut().map(|x| {
                if *x <= T::new(0.5) {
                    *x *= T::new(2.);
                    0
                } else {
                    *x -= T::new(0.5);
                    *x *= T::new(2.);
                    1
                }
            });
            result.push(u3::new(dd[0] | dd[1] << 1 | dd[2] << 2));
        }

        Some(result)
    }
}

impl Default for CellPath {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for CellPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CellPath(")?;
        f.write_str(&format!("{:b}", self.0))?;
        f.write_str(")")?;

        Ok(())
        // f.debug_tuple("CellPath")
        //     .field(&format!("{:b}", self.0))
        //     .finish()
    }
}

impl IntoIterator for &CellPath {
    type Item = u3;
    type IntoIter = CellPathIterator;

    fn into_iter(self) -> Self::IntoIter {
        CellPathIterator { path: self.clone() }
    }
}

pub struct CellPathIterator {
    path: CellPath,
}

impl CellPathIterator {
    pub fn new(path: CellPath) -> Self {
        Self { path }
    }
}

impl From<CellPath> for CellPathIterator {
    fn from(value: CellPath) -> Self {
        Self::new(value)
    }
}

impl Iterator for CellPathIterator {
    type Item = u3;

    fn next(&mut self) -> Option<Self::Item> {
        self.path.pop()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.path.len() as usize;
        (len, Some(len))
    }
}
impl DoubleEndedIterator for CellPathIterator {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.path.pop_back()
    }
}
impl ExactSizeIterator for CellPathIterator {  }
impl FusedIterator for CellPathIterator {  }

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_math::dvec3;

    #[test]
    fn test_neighbor() {
        let path = CellPath(0b1_000);
        assert_eq!(path.neighbor(0, 0, 0).as_ref(), Some(&path));
        assert_eq!(path.neighbor(0, 1, 0), Some(CellPath(0b1_010)));
        assert_eq!(path.neighbor(1, 1, 0), Some(CellPath(0b1_011)));
        assert_eq!(path.neighbor(1, 1, 1), Some(CellPath(0b1_111)));
        assert_eq!(path.neighbor(-1, 1, 1), None);

        let path = CellPath(0b1_010);
        assert_eq!(path.neighbor(0, 0, 0).as_ref(), Some(&path));
        assert_eq!(path.neighbor(0, 1, 0), None);
        assert_eq!(path.neighbor(0, -1, 0), Some(CellPath(0b1_000)));
        assert_eq!(path.neighbor(1, 0, 0), Some(CellPath(0b1_011)));
        assert_eq!(path.neighbor(1, 0, 1), Some(CellPath(0b1_111)));
        assert_eq!(path.neighbor(-1, 1, 1), None);

        let path = CellPath(0b1_100_010);
        assert_eq!(path.neighbor(0, 0, 0), Some(CellPath(0b1_100_010)));
        assert_eq!(path.neighbor(1, 0, 0), Some(CellPath(0b1_100_011)));
        assert_eq!(path.neighbor(1, 0, 1), Some(CellPath(0b1_100_111)));
        assert_eq!(path.neighbor(1, -1, 1), Some(CellPath(0b1_100_101)));
        assert_eq!(path.neighbor(1, 1, 1), Some(CellPath(0b1_110_101)));

        let path = CellPath(0b1_000_111);
        assert_eq!(path.neighbor(0, 0, 0), Some(CellPath(0b1_000_111)));
        assert_eq!(path.neighbor(1, 0, 0), Some(CellPath(0b1_001_110)));
        assert_eq!(path.neighbor(0, 1, 0), Some(CellPath(0b1_010_101)));
        assert_eq!(path.neighbor(1, 1, 0), Some(CellPath(0b1_011_100)));
        assert_eq!(path.neighbor(0, 0, 1), Some(CellPath(0b1_100_011)));
        assert_eq!(path.neighbor(1, 0, 1), Some(CellPath(0b1_101_010)));
        assert_eq!(path.neighbor(0, 1, 1), Some(CellPath(0b1_110_001)));
        assert_eq!(path.neighbor(1, 1, 1), Some(CellPath(0b1_111_000)));
        assert_eq!(path.neighbor(-1, 0, 0),   Some(CellPath(0b1_000_110)));
        assert_eq!(path.neighbor(0, -1, 0),   Some(CellPath(0b1_000_101)));
        assert_eq!(path.neighbor(-1, -1, 0),  Some(CellPath(0b1_000_100)));
        assert_eq!(path.neighbor(0, 0, -1),   Some(CellPath(0b1_000_011)));
        assert_eq!(path.neighbor(-1, 0, -1),  Some(CellPath(0b1_000_010)));
        assert_eq!(path.neighbor(0, -1, -1),  Some(CellPath(0b1_000_001)));
        assert_eq!(path.neighbor(-1, -1, -1), Some(CellPath(0b1_000_000)));
    }

    #[test]
    fn test_extended() {
        let path_a = CellPath(0b1_000_000);
        let path_b = CellPath(0b1_000_000);
        assert_eq!(path_a.extended(&path_b), CellPath(0b1_000_000_000_000));
        let path_a = CellPath(0b1_110_011);
        let path_b = CellPath(0b1_100_101);
        assert_eq!(path_a.clone().extended(&path_b), CellPath(0b1_110_011_100_101));
        assert_eq!(path_b.clone().extended(&path_a), CellPath(0b1_100_101_110_011));
    }

    #[test]
    fn test_take_depth() {
        assert_eq!(
            CellPath(0b1_000_010_010_111).take(1),
            CellPath(0b1_000)
        );
        assert_eq!(
            CellPath(0b1_001_010_101_010).take(1),
            CellPath(0b1_001)
        );
        assert_eq!(
            CellPath(0b1_010_110_101_010).take(2),
            CellPath(0b1_010_110)
        );
        assert_eq!(
            CellPath(0b1_010_110_101_010).take(3),
            CellPath(0b1_010_110_101)
        );
    }

    #[test]
    fn test_push() {
        assert_eq!(
            CellPath(0b1).with_push(u3::new(0b000)),
            CellPath(0b1_000)
        );
        assert_eq!(
            CellPath(0b1_000).with_push(u3::new(0b010)),
            CellPath(0b1_000_010)
        );
        assert_eq!(
            CellPath(0b1_010).with_push(u3::new(0b010)),
            CellPath(0b1_010_010)
        );
        assert_eq!(
            CellPath(0b1_111).with_push(u3::new(0b010)),
            CellPath(0b1_111_010)
        );
        assert_eq!(
            CellPath(0b1_111).with_push(u3::new(0b010)),
            CellPath(0b1_111_010)
        );
    }

    #[test]
    fn test_push_back() {
        assert_eq!(
            CellPath(0b1).with_push_back(u3::new(0b000)),
            CellPath(0b1_000)
        );
        assert_eq!(
            CellPath(0b1_000).with_push_back(u3::new(0b010)),
            CellPath(0b1_010_000)
        );
        assert_eq!(
            CellPath(0b1_010).with_push_back(u3::new(0b010)),
            CellPath(0b1_010_010)
        );
        assert_eq!(
            CellPath(0b1_111).with_push_back(u3::new(0b010)),
            CellPath(0b1_010_111)
        );
        assert_eq!(
            CellPath(0b1_111).with_push_back(u3::new(0b010)),
            CellPath(0b1_010_111)
        );
        assert_eq!(
            CellPath(0b1_000_111).with_push_back(u3::new(0b010)),
            CellPath(0b1_010_000_111)
        );
        assert_eq!(
            CellPath(0b1_010_000_111).with_push_back(u3::new(0b000)),
            CellPath(0b1_000_010_000_111)
        );
    }

    #[test]
    fn test_pop() {
        let mut path = CellPath(0b1);
        assert_eq!(path.pop(), None);
        assert_eq!(path, CellPath(0b1));

        path = CellPath(0b1_000);
        assert_eq!(path.pop(), Some(u3::new(0b000)));
        assert_eq!(path, CellPath(0b1));

        path = CellPath(0b1_000_010);
        assert_eq!(path.pop(), Some(u3::new(0b000)));
        assert_eq!(path, CellPath(0b1_010));

        path = CellPath(0b1_010_010);
        assert_eq!(path.pop(), Some(u3::new(0b010)));
        assert_eq!(path, CellPath(0b1_010));

        path = CellPath(0b1_111_010);
        assert_eq!(path.pop(), Some(u3::new(0b111)));
        assert_eq!(path, CellPath(0b1_010));

        path = CellPath(0b1_111_110);
        assert_eq!(path.pop(), Some(u3::new(0b111)));
        assert_eq!(path, CellPath(0b1_110));

        path = CellPath(0b1_000_111_110);
        assert_eq!(path.pop(), Some(u3::new(0b000)));
        assert_eq!(path, CellPath(0b1_111_110));

        path = CellPath(0b1_000_101_001);
        assert_eq!(path.pop(), Some(u3::new(0b000)));
        assert_eq!(path, CellPath(0b1_101_001));
    }

    #[test]
    fn test_pop_back() {
        let path = CellPath(0b1);
        assert_eq!(path.pop_back(), None);

        let path = CellPath(0b1_000);
        assert_eq!(path.pop_back(), Some(u3::new(0b000)));

        let path = CellPath(0b1_000_010);
        assert_eq!(path.pop_back(), Some(u3::new(0b000)));

        let path = CellPath(0b1_010_110);
        assert_eq!(path.pop_back(), Some(u3::new(0b010)));

        let path = CellPath(0b1_111_010);
        assert_eq!(path.pop_back(), Some(u3::new(0b111)));

        let path = CellPath(0b1_111_010);
        assert_eq!(path.pop_back(), Some(u3::new(0b111)));

        let path = CellPath(0b1_000_011_111_010);
        assert_eq!(path.pop_back(), Some(u3::new(0b000)));
    }

    #[test]
    fn test_get_aabb() {
        let aabb = DAabb::from_minmax(
            DVec3::splat(0.),
            DVec3::splat(24.),
        );

        assert_eq!(CellPath::new().get_aabb(aabb), aabb);

        assert_eq!(
            CellPath(0b1_000).get_aabb(aabb),
            DAabb::from_minmax(DVec3::splat(0.), DVec3::splat(12.))
        );
        assert_eq!(
            CellPath(0b1_000_000).get_aabb(aabb),
            DAabb::from_minmax(DVec3::splat(0.), DVec3::splat(6.))
        );
        assert_eq!(
            CellPath(0b1_000_000_000).get_aabb(aabb),
            DAabb::from_minmax(DVec3::splat(0.), DVec3::splat(3.))
        );
        assert_eq!(
            CellPath(0b1_001).get_aabb(aabb),
            DAabb::from_minmax(dvec3(12., 0., 0.), dvec3(24., 12., 12.))
        );
        assert_eq!(
            CellPath(0b1_100).get_aabb(aabb),
            DAabb::from_minmax(dvec3(0., 0., 12.), dvec3(12., 12., 24.))
        );
        assert_eq!(
            CellPath(0b1_010).get_aabb(aabb),
            DAabb::from_minmax(dvec3(0., 12., 0.), dvec3(12., 24., 12.))
        );
        assert_eq!(
            CellPath(0b1_010_000).get_aabb(aabb),
            DAabb::from_minmax(dvec3(0., 12., 0.), dvec3(6., 18., 6.))
        );
        assert_eq!(
            CellPath(0b1_010_111).get_aabb(aabb),
            DAabb::from_minmax(dvec3(6., 18., 6.), dvec3(12., 24., 12.))
        );
        assert_eq!(
            CellPath(0b1_000_111).get_aabb(aabb),
            DAabb::from_minmax(dvec3(6., 6., 6.), dvec3(12., 12., 12.))
        );
    }
}
