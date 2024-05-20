use super::*;

use std::cell::RefCell;
use bevy::{math::DVec3, prelude::*};
use svo::AggregateData as _;
use utils::DAabb;
use either::Either;
use arbitrary_int::*;

#[derive(Debug, Clone, Copy)]
pub(super) struct SvoEntityRepr {
    pub entity: Entity,
    pub global_pos: DVec3,
    pub mass: f64,
}

#[derive(Debug, Default, Clone)]
pub(super) struct SvoData {
    pub aabb: DAabb,
    pub entities: Vec<SvoEntityRepr>,
    pub remaining_allowed_depth: u8,
}

#[derive(Debug, Default, Clone, Copy)]
pub(super) struct SvoInternalData {
    pub aabb: DAabb,
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

        for cell in children.iter() {
            match cell {
                Either::Left(internal) => {
                    count += internal.count;
                    total_mass += internal.total_mass;
                    weighed_pos_sum += internal.center_of_mass * internal.total_mass;
                },
                Either::Right(leaf) => {
                    count += u32::try_from(leaf.entities.len()).expect("too much entities!!");
                    total_mass += leaf.entities.iter().map(|e| e.mass).sum::<f64>();
                    weighed_pos_sum += leaf.entities.iter().map(|e| e.global_pos * e.mass).sum::<DVec3>();
                },
            }
        }

        let aabb = children.iter()
            .map(|c| match c {
                Either::Left(l) => l.aabb,
                Either::Right(r) => r.aabb,
            })
            .reduce(|mut a, b| {a.expand_to_contain_aabb(b); a})
            .expect("non empty");

        SvoInternalData {
            aabb,
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
            static TARGET_VEC: RefCell<Vec<u3>> = const { RefCell::new(Vec::new()) };
        }

        let mut children = svo::CellPath::components().map(|comp| SvoData {
            aabb: self.aabb.octdivided(comp),
            remaining_allowed_depth: self.remaining_allowed_depth.saturating_sub(1),
            entities: vec![],
        });

        let half_size = self.aabb.size / 2.;
        let middle = self.aabb.position + half_size;

        TARGET_VEC.with(|targets| {
            let mut targets = targets.borrow_mut();
            targets.clear();
            targets.reserve(self.entities.len());

            let mut counts = [0usize; 8];
            self.entities.iter()
                .map(|entity| {
                    let mut comp = 0b000u8;
                    if entity.global_pos.x > middle.x {
                        comp |= 0b001;
                    }
                    if entity.global_pos.y > middle.y {
                        comp |= 0b010;
                    }
                    if entity.global_pos.z > middle.z {
                        comp |= 0b100;
                    }
                    counts[comp as usize] += 1;
                    u3::new(comp)
                })
                .collect_into(&mut *targets);

            for i in 0..8usize {
                children[i].entities.reserve_exact(counts[i]);
            }

            for (entity, comp) in self.entities.into_iter().zip(targets.iter()) {
                children[comp.value() as usize].entities.push(entity);
            }
        });

        let internal = SvoData::aggregate(
            children.each_ref().map(Either::Right)
        );

        (internal, children)
    }
}

impl svo::BorrowedMergeableData for SvoData {
    fn should_auto_merge(
        _this: &Self::Internal,
        children: [&Self; 8]
    ) -> bool {
        let small_count = children.iter()
            .filter(|data| data.entities.len() < SVO_LEAF_MIN_PARTICLE_COUNT)
            .count();

        // Should merge if all but one leaf is under the minimum
        // so that if there is 100 in one leaf but only 5 in the other its not worth splitting
        small_count >= 7
    }

    fn merge(
        this: &Self::Internal,
        children: [&Self; 8]
    ) -> Self {
        Self {
            aabb: this.aabb,
            remaining_allowed_depth: children.iter()
                .map(|p| p.remaining_allowed_depth)
                .max().unwrap_or_default() + 1,
            entities: children.into_iter()
                .flat_map(|data| data.entities.iter().copied())
                .collect(),
        }
    }
}
