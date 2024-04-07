#![allow(unused_imports)]

mod planet_generator;
pub use planet_generator::*;
mod sphere_generator;
pub use sphere_generator::*;

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
