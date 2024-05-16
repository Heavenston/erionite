#![feature(int_roundings)]
#![feature(array_try_map)]
#![feature(iter_array_chunks)]
#![feature(impl_trait_in_assoc_type)]
#![feature(new_uninit)]
#![feature(maybe_uninit_write_slice)]

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
mod ptr;
pub use ptr::*;

pub mod mesh_generation;

use std::fmt::Debug;
use std::sync::Arc;

use rayon::prelude::*;
use either::Either;
use arbitrary_int::*;
use itertools::Itertools;
use bevy_math::UVec3;

#[derive(Clone, Debug)]
pub struct InternalCell<D: Data, Ptr: SvoPtr<D> = Arc<Cell<D>>> {
    pub children: [Ptr; 8],
    pub data: D::Internal,
}

impl<D: Data, Ptr: SvoPtr<D>> InternalCell<D, Ptr> {
    pub fn from_children(children: [impl Into<Ptr>; 8]) -> Self
        where D: AggregateData
    {
        let children = children.map(Into::into);
        let data = D::aggregate(children.each_ref().map(|d| d.data()));
        Self {
            children: children.map(Into::into),
            data,
        }
    }
    
    #[inline]
    pub fn get_child(&self, pos: u3) -> &Ptr {
        &self.children[usize::from(pos.value())]
    }

    #[inline]
    pub fn get_child_mut(&mut self, pos: u3) -> &mut Cell<D, Ptr>
        where Ptr: MutableSvoPtr<D>,
    {
        self.children[usize::from(pos.value())].make_mut()
    }

    pub fn new_full(data: D::Internal, child: impl Into<Ptr>) -> Self
        where Ptr: Clone
    {
        let child = child.into();
        Self {
            children: [1,2,3,4,5,6,7,8].map(|_| child.clone()),
            data,
        }
    }

    pub fn iter_children(&self) -> impl Iterator<Item = &Ptr> {
        self.children.iter()
    }

    pub fn iter_children_mut(&mut self) -> impl Iterator<Item = &mut Cell<D, Ptr>>
        where Ptr: MutableSvoPtr<D>,
    {
        self.children.iter_mut().map(Ptr::make_mut)
    }

    /// Updates the current average information only based on its direct children
    pub fn shallow_update(&mut self)
        where D: AggregateData
    {
        self.data = D::aggregate(self.children.each_ref().map(|x| x.data()));
    }
}

impl<D: Data, Ptr: SvoPtr<D>> Into<Cell<D, Ptr>> for InternalCell<D, Ptr> {
    fn into(self) -> Cell<D, Ptr> {
        Cell::Internal(self)
    }
}

#[derive(Clone, Debug)]
pub struct LeafCell<D: Data> {
    pub data: D,
}

impl<D: Data> LeafCell<D> {
    pub fn new(data: D) -> Self {
        Self { data }
    }
}

impl<D: Data, Ptr: SvoPtr<D>> Into<Cell<D, Ptr>> for LeafCell<D> {
    fn into(self) -> Cell<D, Ptr> {
        Cell::Leaf(self)
    }
}

#[derive(Debug)]
pub enum Cell<D: Data, Ptr: SvoPtr<D> = ArcPtr<D>> {
    Internal(InternalCell<D, Ptr>),
    Leaf(LeafCell<D>),
    /// see [PackedCell]'s docs
    Packed(PackedCell<D>),
}

// Custom impl because the derive macro struggled with the generics
impl<D, Ptr> Clone for Cell<D, Ptr>
    where D: Data + Clone,
          D::Internal: Clone,
          Ptr: SvoPtr<D> + Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::Internal(i) => Self::Internal(i.clone()),
            Self::Leaf(l) => Self::Leaf(l.clone()),
            Self::Packed(p) => Self::Packed(p.clone()),
        }
    }
}

impl<D: Data, Ptr: SvoPtr<D>> Cell<D, Ptr> {
    pub fn data(&self) -> Either<&D::Internal, &D> {
        match self {
            Cell::Internal(i) => Either::Left(&i.data),
            Cell::Leaf(l)     => Either::Right(&l.data),

            Cell::Packed(p)   => p.get(&CellPath::new()),
        }
    }

    pub fn data_mut(&mut self) -> Either<&mut D::Internal, &mut D> {
        match self {
            Cell::Internal(i) => Either::Left(&mut i.data),
            Cell::Leaf(l)     => Either::Right(&mut l.data),

            Cell::Packed(p)   => p.get_mut(&CellPath::new()),
        }
    }

    pub fn has_children(&self) -> bool {
        match self {
            Cell::Internal(_) => true,
            Cell::Leaf(_) => false,
            Cell::Packed(p) => p.depth() > 0,
        }
    }

    /// returns false when merging is impossible (always the case for leaf cells and packed cells)
    /// and true when mering was successfull
    pub fn try_merge(&mut self) -> bool
        where D: MergeableData,
              Ptr: OwnedSvoPtr<D>,
    {
        let mut did_merge = false;
        utils::replace_with(self, |this| {
            let Self::Internal(InternalCell { data, children }) = this
            else {
                did_merge = true;
                return this;
            };

            let Some(children_datas) = children.each_ref()
                .try_map(|x| match &**x {
                    Cell::Leaf(l) => Some(&l.data),
                    _ => None,
                })
            else {
                did_merge = false;
                return Self::Internal(InternalCell { children, data });
            };

            if !D::should_auto_merge(&data, children_datas) {
                did_merge = false;
                return Self::Internal(InternalCell { children, data });
            }

            did_merge = true;

            let taken = children
                .map(|x| match x.into_inner() {
                    Cell::Leaf(l) => l.data,
                    _ => unreachable!("checked before"),
                });
        
            LeafCell::new(
                D::merge(data, taken)
            ).into()
        });
        
        did_merge
    }

    /// Like [Self::try_merge] but only for `D: Copy`, avoid the need for `Ptr: OwnedSvoPtr`
    pub fn try_merge_copy(&mut self) -> bool
        where D: MergeableData + Copy,
    {
        let mut did_merge = false;
        utils::replace_with(self, |this| {
            let Self::Internal(InternalCell { data, children }) = this
            else {
                did_merge = true;
                return this;
            };

            let Some(children_datas) = children.each_ref()
                .try_map(|x| match &**x {
                    Cell::Leaf(l) => Some(l.data),
                    _ => None,
                })
            else {
                did_merge = false;
                return Self::Internal(InternalCell { children, data });
            };

            if !D::should_auto_merge(&data, children_datas.each_ref()) {
                did_merge = false;
                return Self::Internal(InternalCell { children, data });
            }

            did_merge = true;

            LeafCell::new(
                D::merge(data, children_datas)
            ).into()
        });
        
        did_merge
    }

    /// [BorrowedMergeableData] version fo [Self::try_merge]
    pub fn try_merge_borrow(&mut self) -> bool
        where D: BorrowedMergeableData,
    {
        let mut did_merge = false;
        utils::replace_with(self, |this| {
            let Self::Internal(InternalCell { data, children }) = this
            else {
                did_merge = true;
                return this;
            };

            let Some(children_datas) = children.each_ref()
                .try_map(|x| match &**x {
                    Cell::Leaf(l) => Some(&l.data),
                    _ => None,
                })
            else {
                did_merge = false;
                return Self::Internal(InternalCell { children, data });
            };

            if !D::should_auto_merge(&data, children_datas) {
                did_merge = false;
                return Self::Internal(InternalCell { children, data });
            }

            did_merge = true;
        
            LeafCell::new(
                D::merge(&data, children_datas)
            ).into()
        });
        
        did_merge
    }

    /// Calls [try_merge](Self::try_merge) on all children from bottom to up
    /// recursively and returns the numper of successfull merges
    pub fn auto_merge(&mut self) -> usize
        where D: MergeableData,
              Ptr: MutableSvoPtr<D> + OwnedSvoPtr<D>,
    {
        let total: usize =
            self.iter_children_mut().map(|c| c.auto_merge()).sum();
        total + if self.try_merge() { 1 } else { 0 }
    }

    /// Like [Self::try_merge_copy] (see [Self::auto_merge])
    pub fn auto_merge_copy(&mut self) -> usize
        where D: MergeableData + Copy,
              Ptr: MutableSvoPtr<D>,
    {
        let total: usize =
            self.iter_children_mut().map(|c| c.auto_merge_copy()).sum();
        total + if self.try_merge_copy() { 1 } else { 0 }
    }

    /// Like [Self::try_merge_borrow] (see [Self::auto_merge])
    pub fn auto_merge_borrow(&mut self) -> usize
        where D: BorrowedMergeableData,
              Ptr: MutableSvoPtr<D>,
    {
        let total: usize =
            self.iter_children_mut().map(|c| c.auto_merge_borrow()).sum();
        total + if self.try_merge_borrow() { 1 } else { 0 }
    }

    /// Same as [auto_merge](Self::auto_merge) but only traverse cells in
    /// the given path
    pub fn auto_merge_on_path(&mut self, mut path: CellPath) -> usize
        where D: MergeableData,
              Ptr: MutableSvoPtr<D> + OwnedSvoPtr<D>,
    {
        let mut total = 0;
        match self {
            Cell::Internal(i) => {
                if let Some(comp) = path.pop_back() {
                    total += i.get_child_mut(comp).auto_merge_on_path(path);
                }
            },
            Cell::Leaf(_) | Cell::Packed(_) => (),
        }

        if self.try_merge() {
            total += 1;
        }
        total
    }

    /// If the current cell is a leaf node (or a packed leaf node)
    /// uses [D::split] to split the current cell
    /// returns true if a split happened, false otherwise
    pub fn split(&mut self) -> bool
        where D: SplittableData,
              Ptr: OwnedSvoPtr<D>,
    {
        let mut did = false;
        utils::replace_with(self, |this| {
            let leaf_data = match this {
                Cell::Internal(_) => {
                    did = false;
                    return this;
                },
                Cell::Leaf(l) => {
                    l.data
                },
                Cell::Packed(p) => match p.try_into_leaf() {
                    Ok(l) => l.data,
                    Err(p) => {
                        did = false;
                        return Cell::Packed(p);
                    }
                },
            };

            did = true;

            let (data, children) = leaf_data.split();

            InternalCell::<D, Ptr> {
                children: children
                    .map(|data| Ptr::new(LeafCell::new(data).into())),
                data,
            }.into()
        });
        did
    }

    /// Like [Self::split] but checks with [D::should_auto_split] before
    pub fn try_split(&mut self) -> bool
        where D: SplittableData,
              Ptr: OwnedSvoPtr<D>,
    {
        let mut did = false;
        utils::replace_with(self, |this| {
            let leaf_data = match this {
                Cell::Internal(_) => {
                    did = false;
                    return this;
                },
                Cell::Leaf(l) => {
                    l.data
                },
                Cell::Packed(p) => match p.try_into_leaf() {
                    Ok(l) => l.data,
                    Err(p) => {
                        did = false;
                        return Cell::Packed(p);
                    }
                },
            };

            if !leaf_data.should_auto_split() {
                return LeafCell { data: leaf_data }.into();
            }

            did = true;

            let (data, children) = leaf_data.split();

            InternalCell::<D, Ptr> {
                children: children
                    .map(|data| Ptr::new(LeafCell::new(data).into())),
                data,
            }.into()
        });
        did
    }

    /// Recursively splits the current cell until a full tree of the given depth
    /// is created
    /// 
    /// depth = 0 does nothing
    pub fn full_split(&mut self, depth: u32)
        where D: SplittableData,
              Ptr: MutableSvoPtr<D> + OwnedSvoPtr<D>,
    {
        if depth == 0 {
            return;
        }
        self.split();
        self.iter_children_mut().for_each(|c| c.full_split(depth - 1));
    }

    /// Calls try_split on the current cell, then continue the same process
    /// on all its children (new or old ones) until either max_depth is reached
    /// or try_split returns false on a leaf cell
    /// 
    /// max_depth = 0 does nothing
    pub fn auto_split(&mut self, max_depth: u32)
        where D: SplittableData,
              Ptr: MutableSvoPtr<D> + OwnedSvoPtr<D>,
    {
        if max_depth == 0 {
            return;
        }
        self.try_split();
        self.iter_children_mut().for_each(|c| c.auto_split(max_depth - 1));
    }

    /// Same as [auto_split](Self::auto_split) but only traverse cells in
    /// the given path
    pub fn auto_split_on_path(&mut self, mut path: CellPath) -> usize
        where D: SplittableData,
              Ptr: MutableSvoPtr<D> + OwnedSvoPtr<D>,
    {
        let mut total = 0;

        if self.try_split() {
            total += 1;
        }

        match self {
            Cell::Internal(i) => {
                if let Some(comp) = path.pop_back() {
                    total += i.get_child_mut(comp).auto_split_on_path(path);
                }
            },
            Cell::Leaf(_) | Cell::Packed(_) => (),
        }

        total
    }

    /// Replaces the current cell with the given `pref` function and then continues
    /// the same process with its (potentially new) children
    /// then calls `suff` function
    pub fn auto_replace_with<FP, FS>(
        &mut self,
        path: CellPath,
        pref: &mut FP,
        suff: &mut FS,
    )
        where FP: FnMut(&CellPath, Cell<D, Ptr>) -> Cell<D, Ptr>,
              FS: FnMut(&CellPath, Cell<D, Ptr>) -> Cell<D, Ptr>,
              Ptr: MutableSvoPtr<D>,
    {
        utils::replace_with(self, |cell| pref(&path, cell));
        self.iter_children_mut().zip(CellPath::components().into_iter())
            .for_each(|(child, comp)| {
                child.auto_replace_with(path.clone().with_push(comp), pref, suff);
            });
        utils::replace_with(self, |cell| suff(&path, cell));
    }

    pub fn par_auto_replace_with<FP, FS>(
        &mut self,
        path: CellPath,
        pref: &FP,
        suff: &FS,
    )
        where FP: Send + Sync + Fn(&CellPath, Cell<D, Ptr>) -> Cell<D, Ptr>,
              FS: Send + Sync + Fn(&CellPath, Cell<D, Ptr>) -> Cell<D, Ptr>,
              Ptr: Send + MutableSvoPtr<D>,
    {
        utils::replace_with(self, |cell| pref(&path, cell));
        if let Self::Internal(internal) = self {
            internal.children.as_mut_slice()
                .par_iter_mut()
                .zip(CellPath::components().into_par_iter())
                .for_each(|(child, comp)| {
                    child.make_mut()
                        .par_auto_replace_with(path.clone().with_push(comp), pref, suff);
                });
        }
        utils::replace_with(self, |cell| suff(&path, cell));
    }

    /// Makes sure the current cell is an internal cell by depending on the cell's
    /// kind:
    /// - for internal cells, *nothing* is done.
    /// - for leaf cells this works like [split](Self::split)
    /// - for packed cells, a new internal cell is created by using
    ///   [PackedCell::split].
    pub fn to_internal(&mut self) -> &mut InternalCell<D, Ptr>
        where D: SplittableData,
              Ptr: OwnedSvoPtr<D>,
    {
        if let Cell::Internal(i) = self {
            return i;
        }

        utils::replace_with(self, |this| {
            match this {
                Cell::Internal(_) => unreachable!(),
                Cell::Leaf(l) => {
                    let (data, children) = l.data.split();
                    InternalCell::<D, Ptr> {
                        children: children.map(|data| Ptr::new(LeafCell::new(data).into())),
                        data,
                    }.into()
                },
                Cell::Packed(p) => {
                    let (data, children) = p.split();
                    InternalCell::<D, Ptr> {
                        children: children
                            .map(|n| Ptr::new(n.into())),
                        data,
                    }.into()
                },
            }
        });

        let Cell::Internal(as_internal) = self
        else { panic!("Just set"); };
        as_internal
    }

    /// Follows the given path, until a leaf or packed cell is reached
    pub fn follow_path(&self, path: &CellPath) -> (CellPath, &Self) {
        let mut path = path.clone();
        let Some(x) = path.pop_back()
            else { return (CellPath::new(), self); };

        match self {
            Cell::Internal(i) => {
                let (p, s) = i.get_child(x).follow_path(&path);
                (p.with_push_back(x), s)
            },
            Cell::Leaf(_) | Cell::Packed(_) => (CellPath::new(), self),
        }
    }

    /// mut version of [follow_path](Self::follow_path)
    pub fn follow_path_mut(&mut self, path: &CellPath) -> (CellPath, &mut Self)
        where Ptr: MutableSvoPtr<D>,
    {
        let mut path = path.clone();
        let Some(x) = path.pop_back()
            else { return (CellPath::new(), self); };

        match self {
            Cell::Internal(i) => {
                let (p, s) = i.get_child_mut(x).follow_path_mut(&path);
                (p.with_push_back(x), s)
            },
            Cell::Leaf(_) | Cell::Packed(_) => (CellPath::new(), self),
        }
    }

    /// Follows the given path, using [to_internal] at each node
    pub fn follow_internal_path(&mut self, path: &CellPath) -> &mut Cell<D, Ptr>
        where D: SplittableData,
              Ptr: OwnedSvoPtr<D> + MutableSvoPtr<D>,
    {
        let mut path = path.clone();
        let Some(child) = path.pop_back()
            else { return self };

        self.to_internal()
            .get_child_mut(child)
            .follow_internal_path(&path)
    }

    /// Like [follow_path] but does continue into packed cells
    /// so only returns the acquired data
    /// If path is deeper than available the rest of the path is ignored
    pub fn get_path(&self, mut path: CellPath) -> EitherDataRef<D> {
        let mut current = self;
        loop {
            match current {
                Cell::Internal(i) => {
                    let Some(comp) = path.pop_back()
                    else { return current.data(); };

                    current = i.get_child(comp);
                },
                Cell::Leaf(l) => {
                    return Either::Right(&l.data);
                },
                Cell::Packed(p) => {
                    let clamped_path = if path.len() > p.depth() {
                        path.take(p.depth())
                    } else {
                        path
                    };
                    return p.get(&clamped_path);
                },
            }
        }
    }

    /// mut version of [get_path]
    pub fn get_path_mut(&mut self, mut path: CellPath) -> EitherDataMut<D>
        where Ptr: MutableSvoPtr<D>,
    {
        let mut current = self;
        loop {
            match current {
                Cell::Internal(i) => {
                    let Some(comp) = path.pop_back()
                    else { return Either::Left(&mut i.data); };

                    current = i.get_child_mut(comp);
                },
                Cell::Leaf(l) => {
                    return Either::Right(&mut l.data);
                },
                Cell::Packed(p) => {
                    return p.get_mut(&path);
                },
            }
        }
    }

    pub fn map_all<F>(&mut self, update: &mut F)
        where F: FnMut(EitherDataMut<D>) -> (),
              Ptr: MutableSvoPtr<D>,
    {
        match self {
            Cell::Internal(_) | Cell::Leaf(_) => {
                self.iter_children_mut()
                    .for_each(|x| x.map_all(update));
                update(self.data_mut())
            },
            Cell::Packed(p) => {
                for leveli in 0..p.depth() {
                    for (_, path) in PackedIndexIterator::new(leveli) {
                        update(p.get_mut(&path));
                    }
                }
            },
        }
    }

    /// Updates all the internal data of all internal cells
    pub fn update_all(&mut self)
        where D: AggregateData,
              Ptr: MutableSvoPtr<D>,
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
    pub fn update_on_path(&mut self, path: &CellPath)
        where D: AggregateData,
              Ptr: MutableSvoPtr<D>,
    {
        let mut path = path.clone();
        match self {
            Cell::Internal(i) => {
                if let Some(comp) = path.pop_back() {
                    i.get_child_mut(comp).update_on_path(&path);
                }
                i.shallow_update();
            },
            Cell::Leaf(_) => (),
            Cell::Packed(p) => {
                if path.len() > p.depth() {
                    path = path.take(p.depth());
                }
                p.update_on_path(&path);
            },
        }
    }

    pub fn iter_children(&self) -> impl Iterator<Item = &Ptr> {
        match self {
            Cell::Internal(i) => Either::Left(i.children.iter()),
            Cell::Leaf(_) => Either::Right(std::iter::empty()),
            Cell::Packed(_) => Either::Right(std::iter::empty()),
        }
    }

    pub fn iter_children_mut(&mut self) -> impl Iterator<Item = &mut Cell<D, Ptr>>
        where Ptr: MutableSvoPtr<D>
    {
        match self {
            Cell::Internal(i) => Either::Left(i.children.iter_mut().map(Ptr::make_mut)),
            Cell::Leaf(_) => Either::Right(std::iter::empty()),
            Cell::Packed(_) => Either::Right(std::iter::empty()),
        }
    }

    /// A single leaf has depth 0, an inner with all leaf children has leaf 1
    pub fn depth(&self) -> u32 {
        match self {
            Cell::Internal(i) =>
                i.iter_children()
                    .map(|x| x.depth())
                    .max().expect("always 8 children") + 1,
            Cell::Leaf(_) => 0,
            Cell::Packed(p) => p.depth(),
        }
    }

    /// The given data is used for leaf values, and the default for internal
    /// values.
    pub fn new_with_depth(depth: u32, data: D) -> Self
        where Ptr: Clone + OwnedSvoPtr<D>,
              D::Internal: Default,
    {
        if depth == 0 {
            return LeafCell::new(data).into();
        }
        let child = Ptr::new(Self::new_with_depth(depth - 1, data));

        InternalCell::<D, Ptr> {
            children: [
                child.clone(), child.clone(),
                child.clone(), child.clone(),
                child.clone(), child.clone(),
                child.clone(), child.clone(),
            ],
            data: Default::default(),
        }.into()
    }

    pub fn iter(&self) -> SvoIterator<'_, D, Ptr> {
        self.into_iter()
    }
}

impl<D: Data + Default, Ptr: SvoPtr<D>> Default for Cell<D, Ptr> {
    fn default() -> Self {
        Self::Leaf(LeafCell::new(D::default()))
    }
}

impl<'a, D: Data, Ptr: SvoPtr<D>> IntoIterator for &'a Cell<D, Ptr> {
    type Item = <SvoIterator<'a, D, Ptr> as Iterator>::Item;
    type IntoIter = SvoIterator<'a, D, Ptr>;

    fn into_iter(self) -> Self::IntoIter {
        SvoIterator::new(self)
    }
}

impl<D: Data, Ptr: SvoPtr<D>> From<D> for Cell<D, Ptr> {
    fn from(data: D) -> Self {
        Cell::<D, Ptr>::Leaf(LeafCell { data })
    }
}

pub struct SvoIterItem<'a, D> {
    pub path: CellPath,
    pub data: &'a D,
}

pub struct SvoIterator<'a, D: Data, Ptr: SvoPtr<D>> {
    cell: Vec<(CellPath, &'a InternalCell<D, Ptr>, u3)>,
    current_leaf: Option<(CellPath, &'a Cell<D, Ptr>)>,
    packed_iterator: Option<PackedIndexIterator>,
}

impl<'a, D: Data, Ptr: SvoPtr<D>> SvoIterator<'a, D, Ptr> {
    pub fn new(cell: &'a Cell<D, Ptr>) -> Self {
        Self {
            cell: vec![],
            current_leaf: Some((CellPath::new(), cell)),
            packed_iterator: None,
        }
    }
}

impl<'a, D: Data, Ptr: SvoPtr<D>> Iterator for SvoIterator<'a, D, Ptr> {
    type Item = SvoIterItem<'a, D>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.current_leaf.clone() {
                Some((path, Cell::Internal(i))) => {
                    self.cell.push((path, i, u3::new(0b000)));
                },
                Some((path, Cell::Leaf(l))) => {
                    self.current_leaf.take();
                    return Some(SvoIterItem {
                        path,
                        data: &l.data,
                    });
                },
                Some((path, Cell::Packed(p))) => 'branch: {
                    let Some((_, child_path)) = self.packed_iterator
                        .get_or_insert_with(|| PackedIndexIterator::new(p.depth()))
                        .next()
                    else {
                        self.current_leaf.take();
                        self.packed_iterator = None;
                        break 'branch;
                    };
                    return Some(SvoIterItem {
                        path: path.extended(&child_path),
                        data: &p.leaf_level().get(&child_path),
                    });
                },
                None => (),
            }

            let Some((last_path, last_cell, child_i)) = self.cell.last_mut()
            else {
                return None;
            };

            let child = last_cell.get_child(*child_i);
            let child_path = last_path.clone().with_push(*child_i);

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

    impl SplittableData for SumData {
        fn split(self) -> (Self::Internal, [Self; 8]) {
            (
                SumData(42),
                [self; 8]
            )
        }
    }

    fn mc(val: i32) -> Cell<SumData> {
        LeafCell::new(SumData(val)).into()
    }

    #[test]
    pub fn test_update_all_unpacked() {
        let mut cell: Cell<_> = InternalCell::new_full(
            SumData(1),
            Arc::new(LeafCell::new(SumData(1)).into()),
        ).into();
        cell.update_all();
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
            PackedIndexIterator::new(2).map(|p| p.1).collect_vec(),
        );
    }

    #[test]
    pub fn test_to_internal() {
        let mut c: Cell<_> = LeafCell::new(SumData(5)).into();
        assert_eq!(*c.data().into_inner(), 5);
        assert_eq!(c.depth(), 0);
        c.to_internal();
        assert_eq!(*c.data().into_inner(), 42);
        assert_eq!(c.depth(), 1);
        c.update_all();
        assert_eq!(*c.data().into_inner(), 8 * 5);
        assert_eq!(c.depth(), 1);
    }

    #[test]
    pub fn test_to_internal_packed() {
        let mut c: Cell<_> = PackedCell::new_filled(0, SumData(0), SumData(3))
            .into();
        assert_eq!(*c.data().into_inner(), 3);
        assert_eq!(c.depth(), 0);
        c.to_internal();
        assert_eq!(*c.data().into_inner(), 42);
        assert_eq!(c.depth(), 1);
        c.update_all();
        assert_eq!(*c.data().into_inner(), 8 * 3);
    }

    #[test]
    pub fn test_to_internal_packed_l2() {
        let mut c: Cell<_> = PackedCell::new_filled(1, SumData(0), SumData(3))
            .into();
        assert_eq!(*c.data().into_inner(), 0);
        assert_eq!(c.depth(), 1);
        println!("{c:#?}");
        c.to_internal();
        println!("{c:#?}");
        assert_eq!(*c.data().into_inner(), 0);
        assert_eq!(c.depth(), 1);
        c.update_all();
        assert_eq!(*c.data().into_inner(), 8i32 * 3);
    }
}
