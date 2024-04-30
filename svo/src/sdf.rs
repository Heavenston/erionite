use bevy_math::DVec3;
use half::f16;
use utils::DAabb;

use crate::{self as svo, CellPath, PackedCell};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SdfSample {
    pub dist: f64,
    pub material: svo::TerrainCellKind,
}

impl SdfSample {
    pub fn to_terrain(&self) -> svo::TerrainCellData {
        svo::TerrainCellData {
            kind: self.material,
            distance: f16::from_f64(self.dist),
        }
    }
}

fn svo_full<F>(
    sample: &mut F,
    max_subdiv: u32,
    aabb: DAabb,
) -> svo::TerrainCell
    where F: FnMut(&DVec3) -> SdfSample,
{
    let og_sample = sample(&aabb.min());
    let mut packed_cell = svo::TerrainPackedCell::new_leaf(og_sample.to_terrain());

    for depth in 1..(max_subdiv+1) {
        let length = 8usize.pow(depth);
        let mut data = vec![svo::TerrainCellData::default(); length];

        for (index, path) in svo::PackedIndexIterator::new(depth) {
            if index % 8 == 0 {
                data[index] = packed_cell.leaf_level().raw_array()[index/8];
                continue;
            }
            let pos = path.get_aabb(aabb).min();
            let sample = sample(&pos);
            data[index] = sample.to_terrain();
        }

        packed_cell.push_level(data.into_boxed_slice());
    }

    packed_cell.into()
}

fn svo_inner<F, HG>(
    has_geometry: &mut HG,
    sample: &mut F,
    max_subdiv: u32,
    aabb: DAabb,
) -> svo::TerrainCell
    where HG: FnMut(&DAabb) -> bool,
          F: FnMut(&DVec3) -> SdfSample,
{
    if !has_geometry(&aabb) {
        return svo_full(sample, 1, aabb);
    }
    if max_subdiv <= 3 {
        return svo_full(sample, max_subdiv.min(3), aabb);
    }

    let children = CellPath::components().map(|comp| {
        let comp_aabb = CellPath::new().with_push(comp).get_aabb(aabb);
        svo_inner(has_geometry, sample, max_subdiv-1, comp_aabb)
    });

    if children.iter().all(|c| matches!(c, svo::Cell::Packed(..))) {
        let packed_children = children.each_ref().map(|cell| match cell {
            svo::Cell::Packed(p) => p,
            _ => unreachable!("checked before"),
        });
        let root = *packed_children[0].get(&CellPath::new()).into_inner();
        if let Some(repacked) = PackedCell::new_repack(packed_children, root) {
            return repacked.into();
        }
    }

    svo::InternalCell::from_children(children).into()
}

pub fn svo_from_sdf<F, HG>(
    mut has_geometry: HG,
    mut sample: F,
    max_subdiv: u32,
    aabb: DAabb,
) -> svo::TerrainCell
    where HG: FnMut(&DAabb) -> bool,
          F: FnMut(&DVec3) -> SdfSample,
{
    svo_inner(&mut has_geometry, &mut sample, max_subdiv, aabb)
}
