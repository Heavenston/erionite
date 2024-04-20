use bevy_math::DVec3;
use half::f16;
use utils::DAabb;

use crate::{self as svo};

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
    let mut packed_data = svo::TerrainPackedCell::new_default(max_subdiv);

    let width = 2f64.powi(max_subdiv as i32);
    for (index, path) in svo::PackedIndexIterator::new(max_subdiv) {
        let pos = path.get_pos();
        let npos = aabb.position + (pos.as_dvec3() / width) * aabb.size;
        let s = sample(&npos);
        packed_data.leaf_level_mut().raw_array_mut()[index] =  svo::TerrainCellData {
            kind: s.material,
            distance: f16::from_f64(s.dist),
        };
    }

    packed_data.update_all();
    packed_data.into()
}
