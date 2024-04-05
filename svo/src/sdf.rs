use bevy_math::DVec3;
use utils::DAabb;

use crate::{self as svo, PackedCell};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SdfSample {
    pub dist: f64,
    pub material: svo::TerrainCellKind,
}

pub fn svo_from_sdf<F>(
    sample: F, max_subdiv: u32,
    aabb: DAabb,
) -> svo::TerrainCell
    where F: Fn(&DVec3) -> SdfSample + Send + Sync
{
    let mut data = PackedCell::<svo::TerrainCellData>::new_default(max_subdiv);

    let width = 2f64.powi(max_subdiv as i32);
    for (index, pos, _) in svo::PackedIndexIterator::new(max_subdiv) {
        let npos = aabb.position + (pos.as_dvec3() / width) * aabb.size;
        let s = sample(&npos);
        data.leaf_level_mut().raw_array_mut()[index] = svo::TerrainCellData {
            kind: s.material,
            distance: s.dist as f32,
        };
    }

    data.update_all();
    data.into()
}
