use either::Either::{ Left, Right };
use itertools::Itertools;
use num_traits::int::PrimInt;
use std::fmt::Debug;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InnerStatInt<T> {
    pub min: T,
    pub max: T,
    pub average: T,
}

impl<T: Default + Debug + PrimInt> crate::InternalData for InnerStatInt<T> {
    
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatInt<T: Default + Debug + PrimInt>(pub T);

impl<T: Default + Debug + PrimInt> crate::Data for StatInt<T> {
    type Internal = InnerStatInt<T>;
}

impl<T: Default + Debug + PrimInt> From<StatInt<T>> for crate::LeafCell<StatInt<T>> {
    fn from(value: StatInt<T>) -> Self {
        crate::LeafCell {
            data: value
        }
    }
}

impl<T: Default + Debug + PrimInt> crate::SplittableData for StatInt<T> {
    fn split(self) -> (Self::Internal, [Self; 8]) {
        (
            Self::Internal {
                min: self.0,
                max: self.0,
                average: self.0,
            },
            [self; 8],
        )
    }
}

impl<T: Default + Debug + PrimInt> crate::MergeableData for StatInt<T> {
    fn can_merge(
        _this: &InnerStatInt<T>,
        children: [&Self; 8]
    ) -> bool {
        children.iter().all_equal()
    }

    fn merge(
        _this: InnerStatInt<T>,
        children: [Self; 8]
    ) -> Self {
        Self(children[0].0)
    }
}

impl<T: Default + Debug + PrimInt> crate::AggregateData for StatInt<T> {
    fn aggregate<'a>(
        d: [crate::EitherDataRef<Self>; 8]
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

