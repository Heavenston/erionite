use std::fmt::Debug;

use utils::AsVecExt;

use super::*;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct PackedCellLevel<D> {
    data: Box<[D]>,
}

impl<D> PackedCellLevel<D> {
    fn new_filled(depth: u32, data: D) -> Self
        where D: Clone
    {
        Self {
            data: {
                let size = 8usize.pow(depth);
                let mut vec = Vec::with_capacity(size);
                for _ in 0..size.saturating_sub(1) {
                    vec.push(data.clone());
                }
                // avoids unecessary clone
                if size > 0 {
                    vec.push(data);
                }
                vec.into_boxed_slice()
            }
        }
    }

    fn split_sub(&self, comp: u3) -> Self
        where D: Clone
    {
        let sub_count = self.data.len() / 8;
        let sub_i: usize = CellPath::new().with_push_back(comp).index().try_into().unwrap();

        let range = sub_count*sub_i..sub_count*(sub_i+1);
        let slice = self.data[range].iter().cloned().collect::<Box<[D]>>();
        PackedCellLevel { data: slice }
    }
}

fn path_index(path: CellPath) -> usize {
    return path.index().try_into().unwrap();
}

fn level_size(depth: u32) -> u32 {
    2u32.pow(depth).pow(3)
}

// TODO: Test
pub fn path_to_depth_and_pos(cell_path: CellPath) -> (u32, UVec3) {
    let mut pos = UVec3::splat(0);
    for comp in cell_path {
        pos *= 2;
        pos += comp.as_uvec();
    }
    (cell_path.len(), pos)
}

/// Gives indices and coordinates to all cells in levels of given depth
/// in the order they are in memory.
pub struct PackedIndexIterator {
    depth: u32,
    index: usize,
}

impl PackedIndexIterator {
    pub fn new(depth: u32) -> Self {
        Self {
            depth,
            index: 0,
        }
    }
}

impl<'a> Iterator for PackedIndexIterator {
    type Item = (usize, CellPath);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == level_size(self.depth) as usize {
            return None;
        }

        let path = CellPath::from_index(self.index as _, self.depth);
        let current = (self.index, path);

        self.index += 1;

        Some(current)
    }
}

/// See [PackedCellLevelMutx]
pub struct PackedCellLevelRef<'a, D> {
    level: &'a PackedCellLevel<D>,
    depth: u32,
}

impl<'a, D> PackedCellLevelRef<'a, D> {
    pub fn depth(&self) -> u32 {
        self.depth
    }

    pub fn index(&self, path: CellPath) -> usize {
        assert_eq!(
            path.len(), self.depth,
            "Wrong cellpath ({path:?}) depth for accessing level ({})",
            self.depth,
        );
        path_index(path)
    }

    pub fn raw_array(&self) -> &'a [D] {
        &self.level.data
    }

    pub fn get(&self, path: CellPath) -> &'a D {
        &self.level.data[self.index(path)]
    }
}

impl<'a, D> IntoIterator for PackedCellLevelRef<'a, D> {
    type Item = (&'a D, CellPath);
    type IntoIter = impl Iterator<Item = Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        PackedIndexIterator::new(self.depth)
            .map(|(index, path)| (&self.level.data[index], path))
    }
}

/// Annoying level of indirection to get mutable access to the internal packed
/// cell level with two benefits:
///  1. Depth isn't stored in the level as this is redundant information (minimal)
///  2. Prevent lib misuse of replacing the whole level with one of the wrong depth (main reason)
pub struct PackedCellLevelMut<'a, D> {
    level: &'a mut PackedCellLevel<D>,
    depth: u32,
}

impl<'a, D> PackedCellLevelMut<'a, D> {
    pub fn depth(&self) -> u32 {
        self.depth
    }

    pub fn index(&self, path: CellPath) -> usize {
        assert_eq!(path.len(), self.depth);
        path_index(path)
    }

    pub fn raw_array(&self) -> &[D] {
        &self.level.data
    }

    pub fn raw_array_mut(&mut self) -> &mut [D] {
        &mut self.level.data
    }

    pub fn get(&self, path: CellPath) -> &D {
        &self.level.data[self.index(path)]
    }

    pub fn get_mut(&mut self, path: CellPath) -> &mut D {
        &mut self.level.data[self.index(path)]
    }
}

/// Compacted version of a full svo
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct PackedCell<D: Data> {
    #[serde(bound(
        serialize = "D::Internal: serde::Serialize",
        deserialize = "D::Internal: for<'a> serde::Deserialize<'a>",
    ))]
    levels: Box<[PackedCellLevel<D::Internal>]>,
    /// There is always as leaf level so depth >= 1
    leaf_level: PackedCellLevel<D>,
}

impl<D: Data> PackedCell<D> {
    pub fn new_filled(depth: u32, internal_data: D::Internal, data: D) -> Self {
        let levels = (0..depth)
            .map(|level| PackedCellLevel::new_filled(level, internal_data.clone()))
            .collect::<Box<[_]>>();

        Self {
            levels,
            leaf_level: PackedCellLevel::new_filled(depth, data),
        }
    }

    pub fn new_default(depth: u32) -> Self {
        Self::new_filled(depth, Default::default(), Default::default())
    }

    /// used for update_{all, on_path}
    /// updates a single cell
    // TODO: Maybe optimize to not recompute indices from the path everytime ?
    fn update_cell(&mut self, path: CellPath)
        where D: AggregateData
    {
        assert!(path.len() < self.depth(), "only internal cells can be updated");
        
        let children = path.children().map(|child| self.get(child));
        let new_data = D::aggregate(children);

        // path's level isn't the leaf one so we must have an internal data
        *self.get_mut(path).unwrap_left() = new_data;
    }

    /// Re-Aggregate all cells of all levels
    pub fn update_all(&mut self)
        where D: AggregateData
    {
        for leveli in (0..self.depth()).rev() {
            for (_, path) in PackedIndexIterator::new(leveli) {
                self.update_cell(path);
            }
        }
    }

    /// Like [update_all] but only for cells on given path
    pub fn update_on_path(&mut self, path: CellPath)
        where D: AggregateData
    {
        if path.len() < self.depth() {
            self.update_cell(path);
        }
        path.parents().for_each(|parent| self.update_cell(parent));
    }

    pub fn internal_level<'a>(&'a self, depth: u32) -> PackedCellLevelRef<'a, D::Internal> {
        // not debug_assert as this assert optimizes away levels indexing check
        assert!(
            (depth as usize) >= self.levels.len(),
            "Depth is out of internal cells range (to get leaf node use leaf_level)",
        );
        PackedCellLevelRef {
            level: &self.levels[depth as usize],
            depth,
        }
    }

    pub fn internal_level_mut(&mut self, depth: u32) -> PackedCellLevelMut<'_, D::Internal> {
        // not debug_assert as this assert optimizes away levels indexing check
        assert!(
            (depth as usize) >= self.levels.len(),
            "Depth is out of internal cells range (to get leaf node use leaf_level)",
        );
        PackedCellLevelMut {
            level: &mut self.levels[depth as usize],
            depth,
        }
    }

    pub fn leaf_level(&self) -> PackedCellLevelRef<'_, D> {
        PackedCellLevelRef { depth: self.depth(), level: &self.leaf_level }
    }

    pub fn leaf_level_mut(&mut self) -> PackedCellLevelMut<'_, D> {
        PackedCellLevelMut { depth: self.depth(), level: &mut self.leaf_level }
    }

    /// Like leaf_level and internal_level but for homogeneous svos
    pub fn level(&self, depth: u32) -> PackedCellLevelRef<'_, D>
        where D: Data<Internal = D>
    {
        if depth < self.depth() {
            self.internal_level(depth)
        }
        else if depth == self.depth() {
            self.leaf_level()
        }
        else {
            panic!("Depth is out of range");
        }
    }

    /// Like leaf_level_mut and internal_level_mut but for homogeneous svos.
    pub fn level_mut(&mut self, depth: u32) -> PackedCellLevelMut<'_, D>
        where D: Data<Internal = D>
    {
        if depth < self.depth() {
            self.internal_level_mut(depth)
        }
        else if depth == self.depth() {
            self.leaf_level_mut()
        }
        else {
            panic!("Depth is out of range");
        }
    }

    /// Like using self.internal_level or self.leaf_level but has different
    /// lifetime requirements.
    pub fn get(&self, path: CellPath) -> EitherDataRef<'_, D> {
        if path.len() < self.depth() {
            Either::Left(&self.levels[path.len() as usize].data[path_index(path)])
        }
        else if path.len() == self.depth() {
            Either::Right(&self.leaf_level.data[path_index(path)])
        }
        else {
            panic!("Depth is out of range");
        }
    }

    /// Like using self.internal_level_mut or self.leaf_level_mut but has different
    /// lifetime requirements.
    pub fn get_mut(&mut self, path: CellPath) -> EitherDataMut<'_, D> {
        if path.len() < self.depth() {
            Either::Left(
                &mut self.levels[path.len() as usize].data[path_index(path)]
            )
        }
        else if path.len() == self.depth() {
            Either::Right(
                &mut self.leaf_level.data[path_index(path)]
            )
        }
        else {
            panic!("Depth is out of range");
        }
    }

    /// If there is only one leaf the depth is 0
    pub fn depth(&self) -> u32 {
        self.levels.len() as u32
    }

    /// Splitts the given 
    pub fn split(&self) -> [PackedCell<D>; 8] {
        if self.depth() == 0 {
            return [
                self.clone(), self.clone(),
                self.clone(), self.clone(),
                self.clone(), self.clone(),
                self.clone(), self.clone(),
            ];
        }

        CellPath::components().map(|comp| {
            let levels = self.levels.iter().skip(1)
                .map(|level| level.split_sub(comp))
                .collect_vec();
            let leaf_level = self.leaf_level.split_sub(comp);

            PackedCell {
                levels: levels.into_boxed_slice(),
                leaf_level,
            }
        })
    }
}

impl<D> Default for PackedCell<D>
    where D: Data
{
    fn default() -> Self {
        Self::new_default(0)
    }
}

impl<D: Data> Into<Cell<D>> for PackedCell<D> {
    fn into(self) -> Cell<D> {
        Cell::Packed(self)
    }
}
