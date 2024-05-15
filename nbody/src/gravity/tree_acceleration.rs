use super::*;

use std::cell::RefCell;
use bevy::{math::DVec3, prelude::*};
use svo::AggregateData as _;
use utils::AsVecExt;
use either::Either;
use arbitrary_int::*;

#[derive(Debug, Clone)]
pub(super) struct SvoEntityRepr {
    pub entity: Entity,
    /// Pos is relative to the cell it is in
    /// between (0., 0., 0.) and (1., 1., 1.)
    pub pos: DVec3,
    pub mass: f64,
}

#[derive(Debug, Default, Clone)]
pub(super) struct SvoData {
    pub entities: Vec<SvoEntityRepr>,
    pub remaining_allowed_depth: u8,
}

#[derive(Debug, Default, Clone, Copy)]
pub(super) struct SvoInternalData {
    pub count: u32,
    pub total_mass: f64,
    /// Relative to the AABB -> 0,0 for the min corner and 1,1 for the max corner
    pub center_of_mass: DVec3,
}

impl svo::Data for SvoData {
    type Internal = SvoInternalData;
}

impl svo::InternalData for SvoInternalData {
    
}

impl svo::AggregateData for SvoData {
    fn aggregate<'a>(
        children: [svo::EitherDataRef<Self>; 8]
    ) -> Self::Internal {
        let mut count = 0;
        let mut total_mass = 0f64;
        let mut weighed_pos_sum = DVec3::ZERO;

        for (comp, cell) in svo::CellPath::components().iter().zip(children.into_iter()) {
            let sub_cell_min = comp.as_uvec().as_dvec3() / 2.;
            match cell {
                Either::Left(internal) => {
                    count += internal.count;
                    total_mass += internal.total_mass;
                    weighed_pos_sum += internal.total_mass * (
                        internal.center_of_mass / 2. + sub_cell_min
                    );
                },
                Either::Right(leaf) => {
                    count += u32::try_from(leaf.entities.len()).expect("too much entities!!");
                    total_mass += leaf.entities.iter().map(|e| e.mass).sum::<f64>();
                    weighed_pos_sum += leaf.entities.iter()
                        .map(|e| e.pos / 2. + sub_cell_min)
                        .sum::<DVec3>();
                },
            }
        }

        SvoInternalData {
            total_mass,
            count,
            center_of_mass: weighed_pos_sum / total_mass,
        }
    }
}

impl svo::SplittableData for SvoData {
    fn should_auto_split(&self) -> bool {
        self.remaining_allowed_depth > 0 &&
        self.entities.len() > SVO_LEAF_MAX_PARTICLE_COUNT
    }

    fn split(self) -> (Self::Internal, [Self; 8]) {
        std::thread_local! {
            static TARGET_VEC: RefCell<Vec<u3>> = RefCell::new(Vec::new());
        }

        let mut children = svo::CellPath::components().map(|_| SvoData {
            remaining_allowed_depth: self.remaining_allowed_depth.saturating_sub(1),
            ..default()
        });

        TARGET_VEC.with(|targets| {
            let mut targets = targets.borrow_mut();
            targets.clear();

            let mut counts = [0usize; 8];
            self.entities.iter()
                .map(|entity| {
                    let mut comp = 0b000u8;
                    if entity.pos.x > 0.5 {
                        comp |= 0b001;
                    }
                    if entity.pos.y > 0.5 {
                        comp |= 0b010;
                    }
                    if entity.pos.z > 0.5 {
                        comp |= 0b100;
                    }
                    counts[comp as usize] += 1;
                    u3::new(comp)
                })
                .collect_into(&mut *targets);

            for i in 0..8usize {
                children[i].entities.reserve_exact(counts[i]);
            }

            for (mut entity, comp) in self.entities.into_iter().zip(targets.iter()) {
                let sub_origin = comp.as_uvec().as_dvec3() / 2.;
                entity.pos = (entity.pos - sub_origin) * 2.;
                children[comp.value() as usize].entities.push(entity);
            }
        });

        let internal = SvoData::aggregate(
            children.each_ref().map(|leaf| Either::Right(leaf))
        );

        (internal, children)
    }
}
