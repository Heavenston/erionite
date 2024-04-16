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
        root_aabb: DAabb, path: svo::CellPath,
        subdivs: u32,
    ) -> svo::TerrainCell {
        use noise::*;

        let aabb = path.get_aabb(root_aabb);
        let mut r = SmallRng::seed_from_u64(self.seed as u64);

        let distance_noise = DistanceNoise::default();

        let heigth_noise = Fbm::<Simplex>::new(r.gen())
            .set_frequency(0.0005)
            // .set_lacunarity(1.5)
            .set_persistence(0.6)
            .set_octaves(7);
        let heigth_noise = ScaleBias::new(heigth_noise)
            .set_scale(150.);
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
        let pink_noise = ScalePoint::new(
            Perlin::new(r.gen())
        ).set_scale(1. / 1.);

        let mut svo = svo::svo_from_sdf(move |&sp| {
            let spa = [sp.x, sp.y, sp.z].map(|x| x);

            let planet_dist = spa.iter().map(|x| x*x).sum::<f64>();

            if planet_dist < self.radius.powi(2) * 0.5 {
                return svo::SdfSample {
                    dist: planet_dist.sqrt() - self.radius,
                    material: svo::TerrainCellKind::Stone,
                };
            }
            if planet_dist > self.radius.powi(2) * 1.5 {
                return svo::SdfSample {
                    dist: planet_dist.sqrt() - self.radius,
                    material: svo::TerrainCellKind::Air,
                };
            }

            let dist = if false {
                planet_dist.sqrt() - self.radius
            }
            else {
                final_noise.get(spa)
            };

            let mut material = svo::TerrainCellKind::Air;
            if dist <= 0. {
                material = [
                    (svo::TerrainCellKind::Stone, stone_noise.get(spa)),
                    (svo::TerrainCellKind::StoneDarker, stone_darker_noise.get(spa)),
                    (svo::TerrainCellKind::Pink, pink_noise.get(spa)),
                ].into_iter().max_by_key(|(_, v)| OrderedFloat(*v)).unwrap().0;
            }

            svo::SdfSample { dist, material }
        }, subdivs, aabb);

        svo.update_all();
        // let rs = svo.simplify();
        // log::trace!("Simplified svo: {rs}");
        svo
    }
}

