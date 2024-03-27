use rand::prelude::*;
use noise::NoiseFn;
use utils::DAabb;

#[derive(Default, Clone, Copy, PartialEq)]
struct DistanceNoise {
    center: [f64; 3],
}

impl NoiseFn<f64, 3> for DistanceNoise {
    fn get(&self, point: [f64; 3]) -> f64 {
        self.center.iter().enumerate().map(|(i, v)| (v - point[i]).powi(2))
            .sum::<f64>().sqrt()
    }
}

pub trait Generator: Send + Sync {
    fn generate_chunk(
        &self,
        aabb: DAabb, path: svo::CellPath,
        subdivs: u32,
    ) -> svo::TerrainCell;
}

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

        let heigth_noise = HybridMulti::<Perlin>::new(r.gen())
            .set_frequency(1.)
            .set_octaves(10);
        let heigth_noise = ScalePoint::new(heigth_noise)
            .set_scale(1. / 70.);
        let heigth_noise = ScaleBias::new(heigth_noise)
            .set_scale(20.);
        let heigth_noise = Add::new(
            Add::new(
                distance_noise,
                Constant::new(-self.radius),
            ),
            heigth_noise,
        );

        let disp_noise = HybridMulti::<Perlin>::new(r.gen())
            .set_frequency(1.)
            .set_octaves(20);
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

        let mut svo = svo::svo_from_sdf(move |&sp| {
            let spa = [sp.x, sp.y, sp.z].map(|x| x);

            let dist = final_noise.get(spa);

            let mut material = svo::TerrainCellKind::Air;
            if dist <= 0. {
                let stone_sample = stone_noise.get(spa);
                let stone_darker_sample = stone_darker_noise.get(spa);
                let max = [stone_sample, stone_darker_sample].map(ordered_float::OrderedFloat)
                    .into_iter().max().unwrap().0;
                if max == stone_sample {
                    material = svo::TerrainCellKind::Stone;
                }
                else {
                    material = svo::TerrainCellKind::StoneDarker;
                }
            }

            svo::SdfSample { dist, material }
        }, subdivs, aabb);

        svo.simplify();
        svo
    }
}

