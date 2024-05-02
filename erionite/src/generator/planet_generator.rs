use bevy::math::DVec3;
use ordered_float::OrderedFloat;
use rand::prelude::*;

use super::*;

#[derive(Debug, Clone)]
pub struct PlanetGenerator {
    pub radius: f64,
    pub seed: i64,
}

impl Generator for PlanetGenerator {
    fn generate_chunk(
        &self,
        root_aabb: DAabb, path: &svo::CellPath,
        subdivs: u32,
    ) -> svo::TerrainCell {
        use noise::*;

        let radius = self.radius;
        let aabb = path.get_aabb(root_aabb);
        let mut r = SmallRng::seed_from_u64(self.seed as u64);

        let distance_noise = DistanceNoise::default();

        let heigth_noise = Fbm::<Simplex>::new(r.gen())
            .set_frequency(0.00025)
            // .set_lacunarity(1.5)
            .set_persistence(0.58)
            .set_octaves(10);
        let heigth_noise = ScaleBias::new(heigth_noise)
            .set_scale(300.);
        let heigth_noise = Add::new(
            Add::new(
                distance_noise,
                Constant::new(-self.radius),
            ),
            heigth_noise,
        );

        let disp_noise = Fbm::<Simplex>::new(r.gen())
            .set_frequency(1.)
            .set_octaves(2);
        let disp_noise = ScalePoint::new(disp_noise)
            .set_scale(1. / 50.);

        let final_noise = Add::new(
            heigth_noise, disp_noise
        );

        let stone_noise = ScalePoint::new(
            Perlin::new(r.gen())
        ).set_scale(1. / 100.);
        let stone_darker_noise = ScalePoint::new(
            Perlin::new(r.gen())
        ).set_scale(1. / 100.);
        let special_noise = ScalePoint::new(
            Perlin::new(r.gen())
        ).set_scale(1. / 1.);

        let special_big_noise = ScalePoint::new(
            Perlin::new(r.gen())
        ).set_scale(1. / 1000.);

        let svo = svo::svo_from_sdf(
            move |aabb| {
                (!aabb.fully_contained_in_sphere(DVec3::ZERO, radius - 300.)) &&
                aabb.touching_sphere(DVec3::ZERO, radius + 300.)
            },
            move |&sp| {
                let spa = [sp.x, sp.y, sp.z].map(|x| x);

                let planet_dist_squared = spa.iter().map(|x| x*x).sum::<f64>();

                let is_under = planet_dist_squared < (self.radius - 300.).powi(2);
                let is_above = planet_dist_squared > (self.radius + 300.).powi(2);

                let dist = if is_under || is_above {
                    planet_dist_squared.sqrt() - self.radius
                }
                else {
                    final_noise.get(spa)
                };

                let mut material = svo::TerrainCellKind::Air;
                if dist <= 0. {
                    let special = if special_big_noise.get(spa) < 0. {
                        svo::TerrainCellKind::Pink
                    } else {
                        svo::TerrainCellKind::Blue
                    };
                    material = [
                        (svo::TerrainCellKind::Stone, stone_noise.get(spa)),
                        (svo::TerrainCellKind::StoneDarker, stone_darker_noise.get(spa)),
                        (special, special_noise.get(spa)),
                    ].into_iter().max_by_key(|(_, v)| OrderedFloat(*v)).unwrap().0;
                }

                svo::SdfSample { dist, material }
            },
            subdivs,
            aabb,
        );

        // let rs = svo.simplify();
        // log::trace!("Simplified svo: {rs}");
        let mut svo = svo;
        svo.update_all();
        svo
    }
}

