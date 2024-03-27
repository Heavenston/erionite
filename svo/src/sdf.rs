use bevy_math::DVec3;
use utils::DAabb;

use crate as svo;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SdfSample {
    pub dist: f64,
    pub material: svo::TerrainCellKind,
}

const SPLIT_MIN_DELTA: f64 = 0.01;
const CORNER_INDICES: [usize; 8] = [
    0, 1, 2, 3, 4, 5, 6, 7
];
const CORNERS: [DVec3; 8] = [
    DVec3::new(0., 0., 0.),
    DVec3::new(1., 0., 0.),
    DVec3::new(0., 1., 0.),
    DVec3::new(1., 1., 0.),
    DVec3::new(0., 0., 1.),
    DVec3::new(1., 0., 1.),
    DVec3::new(0., 1., 1.),
    DVec3::new(1., 1., 1.),
];

fn svo_from_sdf_inner<F>(
    sample: &F, max_subdiv: u32,
    aabb: DAabb,
    corners: [DVec3; 8],
    corners_samples: [SdfSample; 8],
) -> svo::TerrainCell
    where F: Fn(&DVec3) -> SdfSample + Send + Sync
{
    let mut assocs: [(usize, usize); 56] = [(0,0);56];
    {
        let mut i = 0;
        (0..8).for_each(|x| {
            (x..8).for_each(|y| {
                assocs[i] = (x, y);
                i += 1;
            });
        });
    }

    let edges = assocs.map(|(ia, ib)| {
        (ia, ib, (corners[ia] + corners[ib]) / 2.)
    });

    let diagonal = (aabb.size.x.powi(2) + aabb.size.y.powi(2) + aabb.size.z.powi(2))
        .sqrt();
    let has_geometry = corners_samples.iter().map(|x| x.dist).reduce(f64::min)
        .unwrap() < diagonal;

    let should_split = max_subdiv != 0
        && has_geometry
        && edges.into_iter().any(|(ia, ib, el)| {
            let sample = sample(&el);
            let predicted = (
                  corners_samples[ia].dist
                + corners_samples[ib].dist
            ) / 2.;
            (sample.dist - predicted).abs() > SPLIT_MIN_DELTA
        })
    ;

    if should_split {
        let se = aabb.size / 2.;
        let children = (0..8).map(move |ci| {
            let corner = CORNERS[ci];
            let new_origin = aabb.position + corner * se;
            let new_corners = CORNERS.map(|x| x * se + new_origin);
            let new_corners_samples = CORNER_INDICES.map(|ci2| {
                if ci == ci2 {
                    corners_samples[ci]
                }
                else {
                    sample(&new_corners[ci2])
                }
            });
            svo_from_sdf_inner(
                sample, max_subdiv - 1,
                DAabb { position: aabb.position + corner * se, size: se },
                new_corners,
                new_corners_samples
            )
        }).collect::<Vec<_>>();
        svo::InternalCell::from_children(children.try_into().unwrap()).into()
    }
    else {
        let middle = aabb.position + aabb.size / 2.;
        let sample = sample(&middle);

        svo::LeafCell::new(svo::TerrainCellData {
            kind: sample.material,
            distance: sample.dist as f32,
        }).into()
    }
}

pub fn svo_from_sdf<F>(
    sample: F, max_subdiv: u32,
    aabb: DAabb,
) -> svo::TerrainCell
    where F: Fn(&DVec3) -> SdfSample + Send + Sync
{
    let corners = CORNERS.map(|x| x * aabb.size);
    let corners_sample = corners.map(|p| sample(&p));
    svo_from_sdf_inner(
        &sample,
        max_subdiv,
        aabb,
        corners,
        corners_sample
    )
}
