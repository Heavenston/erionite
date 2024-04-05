use bevy::math::DVec3;
use svo::TerrainCellKind;

use super::*;

#[derive(Debug, Clone)]
pub struct SphereGenerator {
    pub radius: f64,
}

impl Generator for SphereGenerator {
    fn generate_chunk(
        &self,
        root_aabb: DAabb, path: svo::CellPath,
        subdivs: u32,
    ) -> svo::TerrainCell {
        let aabb = path.get_aabb(root_aabb);
        let radius = self.radius;

        let mut svo = svo::svo_from_sdf(move |sp| {
            let dist = sp.distance(DVec3::ZERO) - radius;
            let material = if dist < 0. {
                TerrainCellKind::Stone
            } else {
                TerrainCellKind::Air
            };

            svo::SdfSample { dist, material }
        }, subdivs, aabb);

        svo.update_all();
        // let rs = svo.simplify();
        // log::trace!("Simplified svo: {rs}");
        svo
    }
}
