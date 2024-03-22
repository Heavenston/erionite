use either::Either::{ self, Left, Right };
use itertools::Itertools;
use num_traits::int::PrimInt;
use num_traits::identities::Zero;
use std::{fmt::Debug, ops::Add};

use crate::svo;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InnerStatInt<T> {
    pub min: T,
    pub max: T,
    pub average: T,
}

impl<T: Default + Debug + PrimInt> svo::InternalData for InnerStatInt<T> {
    
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatInt<T: Default + Debug + PrimInt>(pub T);

impl<T: Default + Debug + PrimInt> svo::Data for StatInt<T> {
    type Internal = InnerStatInt<T>;
}

impl<T: Default + Debug + PrimInt> From<StatInt<T>> for svo::LeafCell<StatInt<T>> {
    fn from(value: StatInt<T>) -> Self {
        svo::LeafCell {
            data: value
        }
    }
}

impl<T: Default + Debug + PrimInt> svo::MergeableData for StatInt<T> {
    fn can_merge(
        _this: &InnerStatInt<T>,
        children: [&Self; 8]
    ) -> bool {
        children.iter().all_equal()
    }

    fn merge(
        this: InnerStatInt<T>,
        children: [Self; 8]
    ) -> Option<Self> {
        if !Self::can_merge(&this, children.each_ref()) {
            return None;
        }

        Some(Self(children[0].0))
    }
}

impl<T: Default + Debug + PrimInt> svo::AggregateData for StatInt<T> {
    fn aggregate<'a>(
        d: [svo::EitherDataRef<Self>; 8]
    ) -> InnerStatInt<T> {
        InnerStatInt {
            min: d.into_iter().map(|x| match x {
                Left(x) => x.min,
                Right(x) => x.0,
            }).min().unwrap_or_default(),
            max: d.into_iter().map(|x| match x {
                Left(x) => x.max,
                Right(x) => x.0,
            }).max().unwrap_or_default(),
            average: d.into_iter().map(|x| match x {
                Left(x) => x.max,
                Right(x) => x.0,
            }).fold(
                T::zero(),
                |a, ref b| a.checked_add(b).unwrap_or_default()
            ) / T::from(8).unwrap(),
        }
    }
}

