
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
        self.entities.len() > 10
    }

    fn split(self) -> (Self::Internal, [Self; 8]) {
        let mut children = svo::CellPath::components().map(|_| SvoData {
            remaining_allowed_depth: self.remaining_allowed_depth.saturating_sub(1),
            ..default()
        });

        for mut entity in self.entities {
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
            let comp = u3::new(comp);
            let sub_origin = comp.as_uvec().as_dvec3() / 2.;
            entity.pos = (entity.pos - sub_origin) * 2.;
            children[comp.value() as usize].entities.push(entity);
        }

        let internal = SvoData::aggregate(
            children.each_ref().map(|leaf| Either::Right(leaf))
        );

        (internal, children)
    }
}

impl svo::MergeableData for SvoData {
    fn should_auto_merge(
        this: &Self::Internal,
        _children: [&Self; 8]
    ) -> bool {
        this.count < 100
    }

    fn merge(
        _this: Self::Internal,
        children: [Self; 8]
    ) -> Self {
        Self {
            remaining_allowed_depth: children.iter()
                .map(|c| c.remaining_allowed_depth).max().unwrap_or_default() + 1,
            entities: children.into_iter().flat_map(|x| x.entities).collect(),
        }
    }
}

impl svo::InternalData for SvoInternalData {
    
}
