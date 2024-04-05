#![feature(int_roundings)]
#![feature(array_try_map)]
#![feature(iter_array_chunks)]
#![feature(impl_trait_in_assoc_type)]

mod sdf;
pub use sdf::*;
mod cell_path;
pub use cell_path::*;
mod stat_bool;
pub use stat_bool::*;
mod stat_int;
pub use stat_int::*;
mod terrain;
pub use terrain::*;
mod data;
pub use data::*;
mod packed;
pub use packed::*;

pub mod mesh_generation;

use std::fmt::Debug;
use std::sync::Arc;

use either::Either;
use arbitrary_int::*;
use itertools::Itertools;
use bevy_math::UVec3;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct InternalCell<D: Data> {
    #[serde(bound(
        serialize = "D: serde::Serialize",
        deserialize = "D: for<'a> serde::Deserialize<'a>",
    ))]
    pub children: [Arc<Cell<D>>; 8],
    #[serde(bound(
        serialize = "D::Internal: serde::Serialize",
        deserialize = "D::Internal: for<'a> serde::Deserialize<'a>",
    ))]
    pub data: D::Internal,
}

impl<D: Data> InternalCell<D> {
    pub fn from_children(children: [impl Into<Arc<Cell<D>>>; 8]) -> Self
        where D: AggregateData
    {
        let mut this = Self {
            children: children.map(Into::into),
            data: D::Internal::default(),
        };
        this.shallow_update();
        this
    }
    
    pub fn get_child(&self, pos: u3) -> &Arc<Cell<D>> {
        &self.children[usize::from(pos.value())]
    }

    pub fn get_child_mut(&mut self, pos: u3) -> &mut Cell<D> {
        Arc::make_mut(&mut self.children[usize::from(pos.value())])
    }

    pub fn new_full(data: D) -> Self
        where D: AggregateData
    {
        let mut this = Self {
            children: [
                Arc::new(LeafCell::new(data.clone()).into()),
                Arc::new(LeafCell::new(data.clone()).into()),
                Arc::new(LeafCell::new(data.clone()).into()),
                Arc::new(LeafCell::new(data.clone()).into()),
                Arc::new(LeafCell::new(data.clone()).into()),
                Arc::new(LeafCell::new(data.clone()).into()),
                Arc::new(LeafCell::new(data.clone()).into()),
                Arc::new(LeafCell::new(data.clone()).into()),
            ],
            data: Default::default(),
        };
        this.shallow_update();
        this
    }

    pub fn iter_children(&self) -> impl Iterator<Item = &Arc<Cell<D>>> {
        self.children.iter()
    }

    pub fn iter_children_mut(&mut self) -> impl Iterator<Item = &mut Cell<D>> {
        self.children.iter_mut().map(Arc::make_mut)
    }

    /// Updates the current average information only based on its direct children
    pub fn shallow_update(&mut self)
        where D: AggregateData
    {
        self.data = D::aggregate(self.children.each_ref().map(|x| x.data()));
    }
}

impl<D: Data> Into<Cell<D>> for InternalCell<D> {
    fn into(self) -> Cell<D> {
        Cell::Internal(self)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct LeafCell<D: Data> {
    pub data: D,
}

impl<D: Data> LeafCell<D> {
    pub fn new(data: D) -> Self {
        Self { data }
    }
}

impl<D: Data> Into<Cell<D>> for LeafCell<D> {
    fn into(self) -> Cell<D> {
        Cell::Leaf(self)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum Cell<D: Data> {
    #[serde(bound(
        serialize = "InternalCell<D>: serde::Serialize",
        deserialize = "InternalCell<D>: for<'a> serde::Deserialize<'a>",
    ))]
    Internal(InternalCell<D>),
    Leaf(LeafCell<D>),
    /// see [PackedCell]'s docs
    #[serde(bound(
        serialize = "PackedCell<D>: serde::Serialize",
        deserialize = "PackedCell<D>: for<'a> serde::Deserialize<'a>",
    ))]
    Packed(PackedCell<D>),
}

impl<D: Data> Cell<D> {
    // pub fn is_inner(&self) -> bool {
    //     match self {
    //         Self::Internal(_) => true,
    //         _ => false,
    //     }
    // }

    // pub fn is_leaf(&self) -> bool {
    //     match self {
    //         Self::Leaf(_) => true,
    //         _ => false,
    //     }
    // }
    // pub fn try_inner(&self) -> Option<&InternalCell<D>> {
    //     match self {
    //         Self::Internal(i) => Some(i),
    //         _ => None,
    //     }
    // }

    // pub fn try_leaf(&self) -> Option<&LeafCell<D>> {
    //     match self {
    //         Self::Leaf(i) => Some(i),
    //         _ => None,
    //     }
    // }

    // pub fn try_inner_mut(&mut self) -> Option<&mut InternalCell<D>> {
    //     match self {
    //         Self::Internal(i) => Some(i),
    //         _ => None,
    //     }
    // }

    // pub fn try_leaf_mut(&mut self) -> Option<&mut LeafCell<D>> {
    //     match self {
    //         Self::Leaf(i) => Some(i),
    //         _ => None,
    //     }
    // }

    // pub fn as_inner(&self) -> &InternalCell<D> {
    //     match self {
    //         Self::Internal(i) => i,
    //         _ => panic!("as_inner but not an inner"),
    //     }
    // }

    // pub fn as_leaf(&self) -> &LeafCell<D> {
    //     match self {
    //         Self::Leaf(i) => i,
    //         _ => panic!("as_leaf but not an leaf"),
    //     }
    // }
    
    // pub fn as_inner_mut(&mut self) -> &mut InternalCell<D> {
    //     match self {
    //         Self::Internal(i) => i,
    //         _ => panic!("as_inner but not an inner"),
    //     }
    // }

    // pub fn as_leaf_mut(&mut self) -> &mut LeafCell<D> {
    //     match self {
    //         Self::Leaf(i) => i,
    //         _ => panic!("as_leaf but not an leaf"),
    //     }
    // }

    // pub fn unwrap_inner(self) -> InternalCell<D> {
    //     match self {
    //         Cell::Internal(i) => i,
    //         _ => panic!("unwrap_inner but not a inner"),
    //     }
    // }

    // pub fn unwrap_leaf(self) -> LeafCell<D> {
    //     match self {
    //         Cell::Leaf(i) => i,
    //         _ => panic!("unwrap_leaf but not a leaf"),
    //     }
    // }

    pub fn data(&self) -> Either<&D::Internal, &D> {
        match self {
            Cell::Internal(i) => Either::Left(&i.data),
            Cell::Leaf(l)     => Either::Right(&l.data),

            Cell::Packed(p)   => p.get(CellPath::new()),
        }
    }

    pub fn data_mut(&mut self) -> Either<&mut D::Internal, &mut D> {
        match self {
            Cell::Internal(i) => Either::Left(&mut i.data),
            Cell::Leaf(l)     => Either::Right(&mut l.data),

            Cell::Packed(p)   => p.get_mut(CellPath::new()),
        }
    }

    /// returns false when merging is impossible (always the case for leaf cells and packed cells)
    /// and true when mering was successfull
    pub fn try_merge(&mut self) -> bool
        where D: MergeableData
    {
        let Self::Internal(inner) = self
        else { return true; };

        let Some(x) = inner.children.each_ref().try_map(|x| x.data().right())
        else { return false; };

        if !D::can_merge(&inner.data, x)
        { return false; }

        let Some(taken) = inner.children
            .each_mut()
            .try_map(|x| Arc::make_mut(x).data_mut().right().map(std::mem::take))
        else { unreachable!(); };
        
        *self = LeafCell::new(
            D::merge(std::mem::take(&mut inner.data), taken)
        ).into();
        
        true
    }

    /// If the current cell is a leaf node (or a packed leaf node)
    /// adds a new internal cell using clones of the cell's data
    /// returns true if a split happened, false otherwise
    pub fn split(&mut self) -> bool
        where D: AggregateData
    {
        let Some(data) = self.data_mut().right().map(std::mem::take)
        else { return false; };
        *self = InternalCell::new_full(data).into();
        true
    }

    pub fn full_split(&mut self, depth: usize)
        where D: AggregateData
    {
        if depth == 0 {
            return;
        }
        self.split();
        self.iter_children_mut().for_each(|c| c.full_split(depth - 1));
    }

    /// Follows the given path, until a leaf or packed cell is reached
    pub fn follow_path(&self, mut path: CellPath) -> (CellPath, &Self) {
        let Some(x) = path.pop()
            else { return (path, self); };

        match self {
            Cell::Internal(i) => {
                let (p, s) = i.get_child(x).follow_path(path);
                (p.with_push_back(x), s)
            },
            Cell::Leaf(_) | Cell::Packed(_) => (path, self),
        }
    }

    /// mut version of [follow_path](Self::follow_path)
    pub fn follow_path_mut(&mut self, mut path: CellPath) -> (CellPath, &mut Self) {
        let Some(x) = path.pop()
            else { return (path, self); };

        match self {
            Cell::Internal(i) => {
                let (p, s) = i.get_child_mut(x).follow_path_mut(path);
                (p.with_push_back(x), s)
            },
            Cell::Leaf(_) | Cell::Packed(_) => (path, self),
        }
    }

    /// Same as [follow_path_mut](Self::follow_path_mut) but also splits
    /// any leaf nodes that it comes accross.
    /// Note that the path may still not follow the full path if a packed cell
    /// is reached (so, if the returned cellpath isn't == to path,
    /// the returned cell is guarenteed to be a packed cell)
    pub fn follow_path_and_split(&mut self, mut path: CellPath) -> (CellPath, &mut Self)
        where D: AggregateData
    {
        let Some(child) = path.pop()
            else { return (path, self); };

        self.split();
        match self {
            Cell::Leaf(_) => unreachable!("split should convert leafs to internals"),
            Cell::Internal(i) => i.get_child_mut(child).follow_path_and_split(path),
            Cell::Packed(_) => (path, self),
        }
    }

    pub fn map_all<F>(&mut self, update: &mut F)
        where F: FnMut(EitherDataMut<D>) -> ()
    {
        match self {
            Cell::Internal(_) | Cell::Leaf(_) => {
                self.iter_children_mut()
                    .for_each(|x| x.map_all(update));
                update(self.data_mut())
            },
            Cell::Packed(p) => {
                for leveli in 0..p.depth() {
                    for (_, _, path) in PackedIndexIterator::new(leveli) {
                        update(p.get_mut(path));
                    }
                }
            },
        }
    }

    /// Updates all the internal data of all internal cells
    pub fn update_all(&mut self)
        where D: AggregateData
    {
        match self {
            Cell::Internal(i) => {
                i.iter_children_mut().for_each(Self::update_all);
                i.shallow_update();
            },
            Cell::Leaf(_) => (),
            Cell::Packed(p) => {
                p.update_all();
            },
        }
    }

    /// Like update all but only updates cells that are in the given path
    ///
    /// if path is goes deeper than the cell the rest of the path is ignored
    pub fn update_on_path(&mut self, mut path: CellPath)
        where D: AggregateData
    {
        match self {
            Cell::Internal(i) => {
                if let Some(comp) = path.pop() {
                    i.get_child_mut(comp).update_on_path(path);
                }
                i.shallow_update();
            },
            Cell::Leaf(_) => (),
            Cell::Packed(p) => {
                if path.len() > p.depth() {
                    path = path.take(p.depth());
                }
                p.update_on_path(path);
            },
        }
    }

    pub fn iter_children(&self) -> impl Iterator<Item = &Arc<Cell<D>>> {
        match self {
            Cell::Internal(i) => Either::Left(i.children.iter()),
            Cell::Leaf(_) => Either::Right(std::iter::empty()),
            Cell::Packed(_) => Either::Right(std::iter::empty()),
        }
    }

    pub fn iter_children_mut(&mut self) -> impl Iterator<Item = &mut Cell<D>> {
        match self {
            Cell::Internal(i) => Either::Left(i.children.iter_mut().map(Arc::make_mut)),
            Cell::Leaf(_) => Either::Right(std::iter::empty()),
            Cell::Packed(_) => Either::Right(std::iter::empty()),
        }
    }

    /// A single leaf has depth 0, an inner with all leaf children has leaf 1
    pub fn depth(&self) -> u32 {
        match self {
            Cell::Internal(i) => i.iter_children().map(|x| x.depth()).max().unwrap_or(0) + 1,
            Cell::Leaf(_) => 0,
            Cell::Packed(p) => p.depth(),
        }
    }

    pub fn new_with_depth(depth: u32, data: D) -> Self
        where D: AggregateData
    {
        if depth == 0 {
            return LeafCell::new(data).into();
        }
        let child = Arc::new(Self::new_with_depth(depth - 1, data.clone()));

        let mut this = InternalCell {
            children: [
                child.clone(), child.clone(),
                child.clone(), child.clone(),
                child.clone(), child.clone(),
                child.clone(), child.clone(),
            ],
            data: Default::default(),
        };
        this.shallow_update();
        this.into()
    }

    /// Calls [try_merge](Self::try_merge) on all children from bottom to up
    /// recursively and returns the numper of successfull merges
    pub fn simplify(&mut self) -> usize
        where D: MergeableData
    {
        let total: usize =
            self.iter_children_mut().map(|c| c.simplify()).sum();
        total + if self.try_merge() { 1 } else { 0 }
    }

    /// Same as [simplify](Self::simplify) but only traverse nodes in
    /// the given path
    pub fn simplify_on_path(&mut self, mut path: CellPath) -> usize
        where D: MergeableData
    {
        let mut total = 0;
        match self {
            Cell::Internal(i) => {
                if let Some(comp) = path.pop() {
                    total += i.get_child_mut(comp).simplify_on_path(path);
                }
            },
            Cell::Leaf(_) | Cell::Packed(_) => (),
        }

        if self.try_merge() {
            total += 1;
        }
        total
    }

    pub fn iter(&self) -> SvoIterator<'_, D> {
        self.into_iter()
    }
}

impl<D: Data> Default for Cell<D> {
    fn default() -> Self {
        Self::Leaf(LeafCell::new(D::default()))
    }
}

impl<'a, D: Data> IntoIterator for &'a Cell<D> {
    type Item = <SvoIterator<'a, D> as Iterator>::Item;
    type IntoIter = SvoIterator<'a, D>;

    fn into_iter(self) -> Self::IntoIter {
        SvoIterator::new(self)
    }
}

impl<D: Data> From<D> for Cell<D> {
    fn from(data: D) -> Self {
        Cell::Leaf(LeafCell { data })
    }
}

pub struct SvoIterItem<'a, D> {
    pub path: CellPath,
    pub data: &'a D,
}

pub struct SvoIterator<'a, D: Data> {
    cell: Vec<(CellPath, &'a InternalCell<D>, u3)>,
    current_leaf: Option<(CellPath, &'a Cell<D>)>,
    packed_iterator: Option<PackedIndexIterator>,
}

impl<'a, D: Data> SvoIterator<'a, D> {
    pub fn new(cell: &'a Cell<D>) -> Self {
        Self {
            cell: vec![],
            current_leaf: Some((CellPath::new(), cell)),
            packed_iterator: None,
        }
    }
}

impl<'a, D: Data> Iterator for SvoIterator<'a, D> {
    type Item = SvoIterItem<'a, D>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.current_leaf.as_ref() {
                Some(&(path, Cell::Internal(i))) => {
                    self.cell.push((path, i, u3::new(0b000)));
                },
                Some(&(path, Cell::Leaf(l))) => {
                    self.current_leaf.take();
                    return Some(SvoIterItem {
                        path,
                        data: &l.data,
                    });
                },
                Some(&(path, Cell::Packed(p))) => 'branch: {
                    let Some((_, _, child_path)) = self.packed_iterator
                        .get_or_insert_with(|| PackedIndexIterator::new(p.depth()))
                        .next()
                    else {
                        self.current_leaf.take();
                        self.packed_iterator = None;
                        break 'branch;
                    };
                    return Some(SvoIterItem {
                        path: path.extended(child_path),
                        data: &p.leaf_level().get(child_path),
                    });
                },
                None => (),
            }

            let Some((last_path, last_cell, child_i)) = self.cell.last_mut()
            else {
                return None;
            };

            let child = last_cell.get_child(*child_i);
            let child_path = last_path.with_push(*child_i);

            if *child_i == u3::MAX {
                self.cell.pop();
            }
            else {
                *child_i += u3::new(1);
            }

            self.current_leaf = Some((child_path, child));
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default, Clone, Copy, PartialEq, Eq)]
    struct SumData(pub i32);

    impl Debug for SumData {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }

    impl PartialEq<i32> for SumData {
        fn eq(&self, other: &i32) -> bool {
            self.0 == *other
        }
    }

    impl Data for SumData {
        type Internal = SumData;
    }
    impl InternalData for SumData {  }

    impl AggregateData for SumData {
        fn aggregate<'a>(
            children: [EitherDataRef<Self>; 8]
        ) -> Self::Internal {
            SumData(children.iter().map(|x| x.into_inner().0).sum())
        }
    }

    fn mc(val: i32) -> Cell<SumData> {
        LeafCell::new(SumData(val)).into()
    }

    #[test]
    pub fn test_update_all_unpacked() {
        let mut cell: Cell<_> = InternalCell::new_full(SumData(1)).into();
        assert_eq!(*cell.data().into_inner(), 8);
        if let Cell::Internal(as_internal) = &mut cell {
            as_internal.iter_children_mut().enumerate().for_each(|(i, v)| {
                v.data_mut().into_inner().0 = i as i32+1;
            });
        }
        assert_eq!(*cell.data().into_inner(), 8);
        cell.update_all();
        assert_eq!(*cell.data().into_inner(), (1..=8).sum::<i32>());
        if let Cell::Internal(as_internal) = &mut cell {
            as_internal.get_child_mut(u3::new(0)).data_mut().into_inner().0 = 5;
        }
        assert_eq!(*cell.data().into_inner(), (1..=8).sum::<i32>());
        cell.update_all();
        assert_eq!(*cell.data().into_inner(), (1..=8).sum::<i32>() + 4);
    }

    #[test]
    pub fn test_update_all_packed() {
        let mut cell: Cell<_> = PackedCell::<SumData>::new_filled(
            1, SumData(69), SumData(42)
        ).into();
        println!("{cell:?}");

        assert_eq!(*cell.data().into_inner(), 69);
        if let Cell::Packed(packed) = &mut cell {
            for (i, val) in packed.leaf_level_mut().raw_array_mut().iter_mut().enumerate() {
                *val = SumData(i as i32+1);
            }
        }
        assert_eq!(*cell.data().into_inner(), 69);
        cell.update_all();
        assert_eq!(*cell.data().into_inner(), (1..=8).sum::<i32>());
    }

    #[test]
    pub fn test_update_all_packed_l2() {
        let mut cell: Cell<_> = PackedCell::<SumData>::new_filled(
            2, SumData(69), SumData(42)
        ).into();
        println!("{cell:?}");

        assert_eq!(*cell.data().into_inner(), 69);
        if let Cell::Packed(packed) = &mut cell {
            for (i, val) in packed.leaf_level_mut().raw_array_mut().iter_mut().enumerate() {
                *val = SumData(i as i32+1);
            }
        }
        assert_eq!(*cell.data().into_inner(), 69);
        println!("{cell:?}");
        cell.update_all();
        println!("{cell:?}");
        assert_eq!(*cell.data().into_inner(), (1..=64).sum::<i32>());
    }

    #[test]
    pub fn test_iterator_unpacked() {
        let cell: Cell<_> = mc(0);
        assert_eq!(cell.iter().map(|i| *i.data).collect_vec(), vec![0]);
        assert_eq!(cell.iter().map(|i| i.path).collect_vec(), vec![CellPath::new()]);

        let cell2: Cell<_> = InternalCell::from_children([
            mc(1), mc(2), mc(3), mc(4),
            mc(5), mc(6), mc(7), mc(8),
        ]).into();

        assert_eq!(
            cell2.iter().map(|i| *i.data).collect_vec(),
            vec![1, 2, 3, 4, 5, 6, 7, 8]
        );
        assert_eq!(
            cell2.iter().map(|i| i.path).collect_vec(),
            Vec::from(CellPath::new().children())
        );
    }

    #[test]
    pub fn test_iterator_packed() {
        let mut packed = PackedCell::<SumData>::new_default(1);
        for (i, val) in packed.leaf_level_mut().raw_array_mut().iter_mut().enumerate() {
            *val = SumData(i as i32+1);
        }
        println!("{packed:?}");
        let cell: Cell<_> = packed.into();

        assert_eq!(
            cell.iter().map(|i| *i.data).collect_vec(),
            vec![1, 2, 3, 4, 5, 6, 7, 8]
        );
        assert_eq!(
            cell.iter().map(|i| i.path).collect_vec(),
            Vec::from(CellPath::new().children())
        );
    }

    #[test]
    pub fn test_iterator_packed_l2() {
        let mut packed = PackedCell::<SumData>::new_default(2);
        for (i, val) in packed.leaf_level_mut().raw_array_mut().iter_mut().enumerate() {
            *val = SumData(i as i32+1);
        }
        println!("{packed:?}");
        let cell: Cell<_> = packed.into();

        assert_eq!(
            cell.iter().map(|i| *i.data).collect_vec(),
            (1..=64).collect_vec(),
        );
        assert_eq!(
            cell.iter().map(|i| i.path).collect_vec(),
            PackedIndexIterator::new(2).map(|p| p.2).collect_vec(),
        );
    }
}
