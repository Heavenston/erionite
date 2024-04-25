use bevy::math::DVec3;
use svo::TerrainCellKind;

use super::*;

#[derive(Debug, Clone)]
pub struct SphereGenerator {
    pub radius: f64,
    pub material: svo::TerrainCellKind,
}

impl Generator for SphereGenerator {
    fn generate_chunk(
        &self,
        root_aabb: DAabb, path: &svo::CellPath,
        subdivs: u32,
    ) -> svo::TerrainCell {
        let aabb = path.get_aabb(root_aabb);
        let radius = self.radius;

        let global_material = self.material;

        let mut svo = svo::svo_from_sdf(move |_| true, move |sp| {
            let dist = sp.length() - radius;
            let material = if dist < 0. {
                global_material
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
