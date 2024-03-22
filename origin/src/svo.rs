mod cell_path;
pub use cell_path::*;
mod stat_bool;
pub use stat_bool::*;
mod stat_int;
pub use stat_int::*;
mod terrain;
use either::Either;
pub use terrain::*;

use serde::{Deserialize, Serialize};
use arbitrary_int::*;
use godot::builtin::{
    Vector3, Aabb, meta::{ToGodot, GodotConvert, FromGodot, ConvertError},
    PackedByteArray
};
use itertools::Itertools;
use std::{fmt::Debug, mem::take};
use std::sync::Arc;

pub trait InternalData: Debug + Sized + Default + Clone {
}

impl InternalData for () {  }

pub trait Data: Debug + Sized + Default + Clone {
    type Internal: InternalData;
}

#[allow(type_alias_bounds)]
pub type EitherData<D: Data> = Either<D::Internal, D>;
#[allow(type_alias_bounds)]
pub type EitherDataRef<'a, D: Data> = Either<&'a D::Internal, &'a D>;
#[allow(type_alias_bounds)]
pub type EitherDataMut<'a, D: Data> = Either<& 'a mut D::Internal, & 'a mut D>;

impl Data for () {
    type Internal = ();
}

pub trait MergeableData: Data {
    fn can_merge(
        this: &Self::Internal,
        children: [&Self; 8]
    ) -> bool;
    fn merge(
        this: Self::Internal,
        children: [Self; 8]
    ) -> Option<Self>;
}

pub trait AggregateData: Data {
    fn aggregate<'a>(
        children: [EitherDataRef<Self>; 8]
    ) -> Self::Internal;
}

impl<D: Data<Internal = ()>> AggregateData for D {
    fn aggregate<'a>(
        _d: [EitherDataRef<D>; 8]
    ) -> () { }
}

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
}

impl<D: Data> Cell<D> {
    pub fn is_inner(&self) -> bool {
        match self {
            Self::Internal(_) => true,
            _ => false,
        }
    }

    pub fn is_leaf(&self) -> bool {
        match self {
            Self::Leaf(_) => true,
            _ => false,
        }
    }
    pub fn try_inner(&self) -> Option<&InternalCell<D>> {
        match self {
            Self::Internal(i) => Some(i),
            _ => None,
        }
    }

    pub fn try_leaf(&self) -> Option<&LeafCell<D>> {
        match self {
            Self::Leaf(i) => Some(i),
            _ => None,
        }
    }

    pub fn try_inner_mut(&mut self) -> Option<&mut InternalCell<D>> {
        match self {
            Self::Internal(i) => Some(i),
            _ => None,
        }
    }

    pub fn try_leaf_mut(&mut self) -> Option<&mut LeafCell<D>> {
        match self {
            Self::Leaf(i) => Some(i),
            _ => None,
        }
    }

    pub fn as_inner(&self) -> &InternalCell<D> {
        match self {
            Self::Internal(i) => i,
            _ => panic!("as_inner but not an inner"),
        }
    }

    pub fn as_leaf(&self) -> &LeafCell<D> {
        match self {
            Self::Leaf(i) => i,
            _ => panic!("as_leaf but not an leaf"),
        }
    }
    
    pub fn as_inner_mut(&mut self) -> &mut InternalCell<D> {
        match self {
            Self::Internal(i) => i,
            _ => panic!("as_inner but not an inner"),
        }
    }

    pub fn as_leaf_mut(&mut self) -> &mut LeafCell<D> {
        match self {
            Self::Leaf(i) => i,
            _ => panic!("as_leaf but not an leaf"),
        }
    }

    pub fn unwrap_inner(self) -> InternalCell<D> {
        match self {
            Cell::Internal(i) => i,
            _ => panic!("unwrap_inner but not a inner"),
        }
    }

    pub fn unwrap_leaf(self) -> LeafCell<D> {
        match self {
            Cell::Leaf(i) => i,
            _ => panic!("unwrap_leaf but not a leaf"),
        }
    }

    pub fn data(&self) -> Either<&D::Internal, &D> {
        match self {
            Cell::Internal(i) => Either::Left(&i.data),
            Cell::Leaf(l) => Either::Right(&l.data),
        }
    }

    pub fn data_mut(&mut self) -> Either<&mut D::Internal, &mut D> {
        match self {
            Cell::Internal(i) => Either::Left(&mut i.data),
            Cell::Leaf(l) => Either::Right(&mut l.data),
        }
    }

    /// returns false when merging is impossible
    /// and true when mering was successfull
    ///
    /// returns true if and only if after this call the cell is a leaf ->
    /// returns false if and only if after this call the cell is an internal cell
    pub fn try_merge(&mut self) -> bool
        where D: MergeableData
    {
        if !self.is_inner()
        { return true; }
        let inner = self.as_inner_mut();
        let Some(x) = inner.children.each_ref().try_map(|x| x.try_leaf())
            else { return false; };
        if !D::can_merge(&inner.data, x.map(|y| &y.data))
        { return false; }

        let merged = D::merge(
            take(&mut inner.data),
            inner.children.each_mut()
            .map(|x| take(Arc::make_mut(x)).unwrap_leaf().data)
        ).expect("try merged returned true but merge failed");
        *self = LeafCell::new(merged).into();
        
        true
    }

    /// replaces the current leaf node (panics if not leaf node) with an internal
    /// node with children of the same (cloned) data
    pub fn split(&mut self)
        where D: AggregateData
    {
        let l = self.as_leaf().clone();
        *self = InternalCell::new_full(l.data).into();
    }

    pub fn full_split(&mut self, depth: usize)
        where D: AggregateData
    {
        if depth == 0 {
            return;
        }
        if self.is_leaf() {
            self.split();
        }
        self.iter_children_mut().for_each(|c| c.full_split(depth - 1));
    }

    /// Follows the given path, stoping if a leaf is reached or the path
    /// is finished
    pub fn follow_path(&self, mut path: CellPath) -> (CellPath, &Self) {
        let Some(x) = path.pop()
            else { return (path, self); };
        if self.is_inner() {
            let (p, s) = self.as_inner().get_child(x).follow_path(path);
            (p.with_push_back(x), s)
        }
        else {
            (path, self)
        }
    }

    /// mut version of [follow_path](Self::follow_path)
    pub fn follow_path_mut(&mut self, mut path: CellPath) -> (CellPath, &mut Self) {
        let Some(x) = path.pop()
            else { return (path, self); };
        if self.is_inner() {
            let (p, s) = 
                self.as_inner_mut().get_child_mut(x).follow_path_mut(path);
            (p.with_push_back(x), s)
        }
        else {
            (path, self)
        }
    }

    /// Same as [follow_path_mut](Self::follow_path_mut) but also splits
    /// any leaf nodes that it comes accross.
    /// Effectively making sure that we fully follow the given path.
    pub fn follow_path_and_split(&mut self, mut path: CellPath) -> &mut Self
        where D: AggregateData
    {
        let Some(x) = path.pop()
            else { return self; };
        if self.is_leaf() {
            self.split();
        }

        self.as_inner_mut().get_child_mut(x).follow_path_and_split(path)
    }

    pub fn update_all_data<F>(&mut self, update: &mut F)
        where F: FnMut(Either<&mut D::Internal, &mut D>) -> ()
    {
        if self.is_leaf() {
            update(Either::Right(&mut self.as_leaf_mut().data));
            return;
        }
        update(Either::Left(&mut self.as_inner_mut().data));
        self.iter_children_mut()
            .for_each(|x| x.update_all_data(update));
    }

    pub fn iter_children(&self) -> impl Iterator<Item = &Arc<Cell<D>>> {
        self.as_inner().children.iter()
    }

    pub fn iter_children_mut(&mut self) -> impl Iterator<Item = &mut Cell<D>> {
        self.as_inner_mut().children.iter_mut().map(Arc::make_mut)
    }

    pub fn sample(
        &self, mut coords: Vector3, max_depth: u32
    ) -> Option<(CellPath, &Cell<D>)> {
        let mut curr_path = CellPath::new();

        if coords.x < 0. || coords.x > 1.
        || coords.y < 0. || coords.y > 1.
        || coords.z < 0. || coords.z > 1. {
            return None;
        }

        let mut curr_depth = 0;
        let mut curr = self;
        loop {
            if curr_depth >= max_depth {
                return Some((curr_path, curr));
            }
            let i = match curr {
                Cell::Internal(i) => i,
                Cell::Leaf(_) => return Some((curr_path, curr)),
            };
            let dd = [&mut coords.x, &mut coords.y, &mut coords.z].map(|x| {
                if *x <= 0.5 {
                    *x *= 2.;
                    0
                } else {
                    *x -= 0.5;
                    *x *= 2.;
                    1
                }
            });
            let new_path = u3::new(dd[0] | dd[1] << 1 | dd[2] << 2);
            curr_path.push(new_path);
            curr = i.get_child(new_path);
            curr_depth += 1;
        }
    }

    pub fn sample_mut(
        &mut self, mut coords: Vector3, max_depth: u32
    ) -> Option<(CellPath, &mut Cell<D>)> {
        let mut curr_path = CellPath::new();

        if coords.x < 0. || coords.x > 1.
        || coords.y < 0. || coords.y > 1.
        || coords.z < 0. || coords.z > 1. {
            return None;
        }

        let mut curr_depth = 0;
        let mut curr = self;
        loop {
            if curr_depth >= max_depth {
                return Some((curr_path, curr));
            }
            let i = match curr {
                Cell::Internal(i) => i,
                Cell::Leaf(_) => return Some((curr_path, curr)),
            };
            let dd = [&mut coords.x, &mut coords.y, &mut coords.z].map(|x| {
                if *x <= 0.5 {
                    *x *= 2.;
                    0
                } else {
                    *x -= 0.5;
                    *x *= 2.;
                    1
                }
            });
            let new_path = u3::new(dd[0] | dd[1] << 1 | dd[2] << 2);
            curr_path.push(new_path);
            curr = i.get_child_mut(new_path);
            curr_depth += 1;
        }
    }

    /// A single leaf has depth 0, an inner with all leaf children has leaf 1
    pub fn depth(&self) -> usize {
        if self.is_leaf() {
            return 0;
        }

        self.iter_children().map(|x| x.depth()).max()
            .map(|x| x + 1).unwrap()
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
        if self.is_leaf() { return 0; }
        let total: usize =
            self.iter_children_mut().map(|c| c.simplify()).sum();
        total + if self.try_merge() { 1 } else { 0 }
    }

    /// Same as [simplify](Self::simplify) but only traverse nodes in
    /// the given path
    pub fn simplify_on_path(&mut self, mut path: CellPath) -> usize
        where D: MergeableData
    {
        if self.is_leaf() { return 0; }
        let total = path.pop().map(|child|
            self.as_inner_mut().get_child_mut(child)
                .simplify_on_path(path)
        ).unwrap_or(0);
        total + if self.try_merge() { 1 } else { 0 }
    }

    /// Updates all the internal data of all internal cells in the path
    /// to the child
    pub fn updated_child(&mut self, mut child: CellPath)
        where D: AggregateData
    {
        let Some(internal) = self.try_inner_mut()
            else { return };
        let Some(x) = child.pop()
            else { return };
        internal.get_child_mut(x).updated_child(child);
        internal.shallow_update();
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
        SvoIterator {
            cell: vec![(CellPath::new(), self, u3::new(0))],
        }
    }
}

impl<D: Data> From<D> for Cell<D> {
    fn from(data: D) -> Self {
        Cell::Leaf(LeafCell { data })
    }
}

impl<D> GodotConvert for Cell<D>
    where D: Data
{
    type Via = PackedByteArray;
}

impl<D> FromGodot for Cell<D>
    where D: Data,
          Cell<D>: for<'a> Deserialize<'a>,
{
    fn try_from_godot(via: Self::Via) -> Result<Self, ConvertError> {
        bincode::deserialize(via.as_slice())
            .map_err(|_| ConvertError::new())
    }
}

impl<D> ToGodot for Cell<D>
    where D: Data,
          Cell<D>: Serialize,
{
    fn to_godot(&self) -> Self::Via {
        let rs = bincode::serialize(self).expect("serialization error");
        From::from(rs.as_slice())
    }
}

pub struct SvoIterItem<'a, D: Data> {
    pub path: CellPath,
    pub cell: &'a LeafCell<D>,
}

pub struct SvoIterator<'a, D: Data> {
    cell: Vec<(CellPath, &'a Cell<D>, u3)>,
}

impl<'a, D: Data> Iterator for SvoIterator<'a, D> {
    type Item = SvoIterItem<'a, D>;

    fn next(&mut self) -> Option<Self::Item> {
        let (path, cell, child_index) = self.cell.last_mut()?;
        let mut path = *path;
        let mut child_index = child_index;
        let mut cell = match cell {
            Cell::Internal(i) => i,
            Cell::Leaf(l) => {
                self.cell.pop();
                return Some(SvoIterItem {
                    path,
                    cell: l,
                });
            },
        };

        loop {
            path.push(*child_index);
            let tor = cell.get_child(*child_index);
            if *child_index == u3::MAX {
                self.cell.pop();
            }
            else {
                *child_index += u3::new(1);
            }

            match &**tor {
                Cell::Internal(i) => {
                    self.cell.push((
                        path, tor, u3::new(0)
                    ));
                    child_index = &mut self.cell.last_mut()?.2;
                    cell = i;
                },
                Cell::Leaf(l) => {
                    return Some(SvoIterItem {
                        path,
                        cell: l,
                    });
                }
            }
        }
    }
}
