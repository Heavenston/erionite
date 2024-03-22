use arbitrary_int::*;
use godot::builtin::{
    Vector3, Aabb, meta::{ToGodot, GodotConvert, FromGodot, ConvertError},
    PackedByteArray
};
use itertools::Itertools;
use std::sync::Arc;

type CellPathInner = u128;

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct CellPath(CellPathInner);
impl CellPath {
    pub const MAX_CAPACITY: u32 = CellPathInner::BITS.div_floor(3);

    pub fn new() -> Self {
        Self(0x1)
    }

    pub const fn capacity(&self) -> u32 {
        Self::MAX_CAPACITY
    }

    #[doc(alias = "depth")]
    pub const fn len(&self) -> u32 {
        debug_assert!(
            self.0.leading_zeros() < CellPathInner::BITS,
            "invalid inner value"
        );
        let start_bit = CellPathInner::BITS - self.0.leading_zeros() - 1;
        debug_assert!(start_bit % 3 == 0, "invalid inner value");
        return start_bit / 3;
    }

    #[doc(alias = "len")]
    pub const fn depth(&self) -> u32 {
        self.len()
    }

    pub fn with_push(mut self, v: u3) -> Self {
        self.push(v);
        self
    }

    pub fn push(&mut self, v: u3) {
        assert!(self.capacity() > self.len());

        self.0 <<= 3;
        self.0 |= CellPathInner::from(v.value());
    }

    pub fn push_back(&mut self, v: u3) {
        assert!(self.capacity() > self.len());

        let marker_bit = CellPathInner::BITS - self.0.leading_zeros() - 1;
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

        let marker_bit = CellPathInner::BITS - self.0.leading_zeros() - 1;
        Some(u3::extract_u128(self.0, (marker_bit - 3) as usize))
    }

    pub fn pop(&mut self) -> Option<u3> {
        if self.len() == 0 {
            return None;
        }

        let marker_bit = CellPathInner::BITS - self.0.leading_zeros() - 1;
        let x = marker_bit - 3;
        let val = u3::extract_u128(self.0, x as usize);

        // remove last bits
        self.0 &= !(CellPathInner::MAX << x);
        // add marker bit
        self.0 |= 1 << x;

        Some(val)
    }
    
    pub fn parent(self) -> Option<Self> {
        if self.len() == 0
        { return None; }

        Some(Self(self.0 >> 3))
    }

    /// Returns an iterator over all parents, from the deepest to the root
    pub fn parents(mut self) -> impl Iterator<Item = Self> {
        std::iter::from_fn(move || { self = self.parent()?; Some(self) })
    }

    pub fn get_aabb(self, mut root: Aabb) -> Aabb {
        for x in self {
            let x = x.value();
            let diff = Vector3::new(
                if (x & 0b001) == 0 { 0. } else { 1. },
                if (x & 0b010) == 0 { 0. } else { 1. },
                if (x & 0b100) == 0 { 0. } else { 1. },
            );
            root.size /= 2.;
            root.position += root.size * diff;
        }
        root
    }

    pub fn neighbor(self, dx: i8, dy: i8, dz: i8) -> Option<Self> {
        assert!(
            dx >= -1 && dx <= 1 &&
            dy >= -1 && dy <= 1 &&
            dz >= -1 && dz <= 1
        );

        let mut new = self;

        for (d, i) in [(dx, 0), (dy, 1), (dz, 2)].into_iter() {
            if d == 0
            { continue; }
            let mut diff: CellPathInner = 0;
            let bit: CellPathInner = 1 << i;
            loop {
                if (diff / 3) >= self.len().into() {
                    return None;
                }
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
    pub fn neighbors(self) -> impl Iterator<Item = ((i8, i8, i8), Self)> {
        (-1..=1).flat_map(move |x| (-1..=1)
            .flat_map(move |y| (-1..=1)
            .map(move |z| (x, y, z))))
            // excluding itself
            .filter(|&(x, y, z)| x != 0 || y != 0 || z != 0)
            .filter_map(move |(x, y, z)| self.neighbor(x, y, z)
                .map(|xx| ((x, y, z), xx)))
    }

    pub fn children(self) -> [Self; 8] {
        [
            self.with_push(u3::new(0b000)),
            self.with_push(u3::new(0b001)),
            self.with_push(u3::new(0b010)),
            self.with_push(u3::new(0b011)),
            self.with_push(u3::new(0b100)),
            self.with_push(u3::new(0b101)),
            self.with_push(u3::new(0b110)),
            self.with_push(u3::new(0b111)),
        ]
    }

    pub fn all_iter(depth: usize) -> impl Iterator<Item = Self> {
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
        let marker_bit = CellPathInner::BITS - self.0.leading_zeros() - 1;

        self.0 & !(CellPathInner::MAX << marker_bit)
    }

    /// Return a new CellPath with only the first [depth] elements of self
    /// Panics if [depth] is higher than [len](Self::len)
    /// the exact inverse of [reparent]
    pub fn take_depth(&self, depth: u32) -> Self {
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

    pub fn extended(self, other: Self) -> Self {
        assert!(Self::MAX_CAPACITY > self.len() + other.len());
        Self((self.0 << (other.len() * 3)) | other.index())
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

impl Iterator for CellPath {
    type Item = u3;

    fn next(&mut self) -> Option<Self::Item> {
        self.pop()
    }
}

impl GodotConvert for CellPath {
    type Via = PackedByteArray;
}

impl ToGodot for CellPath {
    fn to_godot(&self) -> Self::Via {
        PackedByteArray::from(self.0.to_be_bytes().as_slice())
    }
}

impl FromGodot for CellPath {
    fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
        Ok(Self(CellPathInner::from_be_bytes(
            via.as_slice().try_into().map_err(|x| ConvertError::with_cause(x))?
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_neighbor() {
        let path = CellPath::new()
            .with_push(u3::new(0b000));
        assert_eq!(path.neighbor(0, 0, 0), Some(path));
        assert_eq!(path.neighbor(0, 1, 0), Some(CellPath(0b1_010)));
        assert_eq!(path.neighbor(1, 1, 0), Some(CellPath(0b1_011)));
        assert_eq!(path.neighbor(1, 1, 1), Some(CellPath(0b1_111)));
        assert_eq!(path.neighbor(-1, 1, 1), None);

        let path = CellPath(0b1010);
        assert_eq!(path.neighbor(0, 0, 0), Some(path));
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
        assert_eq!(path_a.extended(path_b), CellPath(0b1_000_000_000_000));
        let path_a = CellPath(0b1_110_011);
        let path_b = CellPath(0b1_100_101);
        assert_eq!(path_a.extended(path_b), CellPath(0b1_110_011_100_101));
        assert_eq!(path_b.extended(path_a), CellPath(0b1_100_101_110_011));
    }
}
