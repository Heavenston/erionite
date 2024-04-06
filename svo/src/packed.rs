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
}

fn level_cell_index(depth: u32, pos: UVec3) -> usize {
    let width = 2u32.pow(depth);
    debug_assert!(
        pos.x <= width && pos.y <= width && pos.z <= width,
        "Coordinates ({pos:?}) are out of range for depth {depth} (width is {width})"
    );
    (pos.x + width * pos.y + width.pow(2) * pos.z) as usize
}

fn path_index(path: CellPath) -> usize {
    let mut pos = UVec3::new(0, 0, 0);
    for comp in path {
        pos *= 2;
        pos += comp.as_uvec();
    }
    return level_cell_index(path.len(), pos);
}

fn level_width(depth: u32) -> u32 {
    2u32.pow(depth)
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
    coords: UVec3,
    index: usize,

    path: [CellPath; 3],
}

impl PackedIndexIterator {
    pub fn new(depth: u32) -> Self {
        Self {
            depth,
            coords: UVec3::splat(0),
            index: 0,
            path: [(0..depth).fold(
                CellPath::new(),
                |path, _| path.with_push(u3::new(0b000))
            ); 3],
        }
    }
}

impl<'a> Iterator for PackedIndexIterator {
    type Item = (usize, UVec3, CellPath);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index == level_size(self.depth) as usize {
            return None;
        }

        let current = (self.index, self.coords, self.path[0]);

        let width = level_width(self.depth);

        self.index += 1;

        // 0 for X, 1 for Y, 2 for Z
        let mut neighbor = 0;

        self.coords.x += 1;
        if self.coords.x == width {
            self.coords.x = 0; self.coords.y += 1;
            neighbor = 1;
        }
        if self.coords.y == width {
            self.coords.y = 0; self.coords.z += 1;
            neighbor = 2;
        }
        // z coords cannot overflow the width as we know index isn't at the end

        match neighbor {
            _ if self.index == level_size(self.depth) as usize => (),
            0 => {
                self.path[0] = self.path[0].neighbor(1, 0, 0).unwrap();
            },
            1 => {
                self.path[1] = self.path[1].neighbor(0, 1, 0).unwrap();
                self.path[0] = self.path[1];
            },
            2 => {
                self.path[2] = self.path[2].neighbor(0, 0, 1).unwrap();
                self.path[0] = self.path[2];
                self.path[1] = self.path[2];
            }

            _ => unreachable!(),
        }

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

    pub fn index_from_pos(&self, pos: UVec3) -> usize {
        level_cell_index(self.depth, pos)
    }

    pub fn raw_array(&self) -> &'a [D] {
        &self.level.data
    }

    pub fn get(&self, path: CellPath) -> &'a D {
        &self.level.data[self.index(path)]
    }

    pub fn get_from_pos(&self, pos: UVec3) -> &'a D {
        &self.level.data[self.index_from_pos(pos)]
    }
}

impl<'a, D> IntoIterator for PackedCellLevelRef<'a, D> {
    type Item = (&'a D, UVec3, CellPath);
    type IntoIter = impl Iterator<Item = Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        PackedIndexIterator::new(self.depth)
            .map(|(index, coords, path)| (&self.level.data[index], coords, path))
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

    pub fn index_from_pos(&self, pos: UVec3) -> usize {
        level_cell_index(self.depth, pos)
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

    pub fn get_from_pos(&self, pos: UVec3) -> &D {
        &self.level.data[self.index_from_pos(pos)]
    }

    pub fn get_mut_from_pos(&mut self, pos: UVec3) -> &mut D {
        &mut self.level.data[self.index_from_pos(pos)]
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
            for (_, _, path) in PackedIndexIterator::new(leveli) {
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
}

impl<D: Data> Into<Cell<D>> for PackedCell<D> {
    fn into(self) -> Cell<D> {
        Cell::Packed(self)
    }
}
