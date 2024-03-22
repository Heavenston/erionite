use either::Either::{ Left, Right };
use itertools::Itertools;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct InnerStatBool {
    pub any: bool,
    pub all: bool,
}

impl crate::InternalData for InnerStatBool {
    
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatBool(pub bool);

impl crate::Data for StatBool {
    type Internal = InnerStatBool;
}

impl From<StatBool> for crate::LeafCell<StatBool> {
    fn from(value: StatBool) -> Self {
        crate::LeafCell {
            data: value
        }
    }
}

impl crate::MergeableData for StatBool {
    fn can_merge(
        _this: &InnerStatBool,
        children: [&Self; 8]
    ) -> bool {
        children.iter().all_equal()
    }

    fn merge(
        this: InnerStatBool,
        children: [Self; 8]
    ) -> Option<Self> {
        if !Self::can_merge(&this, children.each_ref()) {
            return None;
        }

        Some(Self(children[0].0))
    }
}

impl crate::AggregateData for StatBool {
    fn aggregate<'a>(
        d: [crate::EitherDataRef<Self>; 8]
    ) -> InnerStatBool {
        InnerStatBool {
            any: d.iter().any(|x| match x {
                Left(l) => l.any,
                Right(l) => l.0,
            }),
            all: d.iter().all(|x| match x {
                Left(l) => l.all,
                Right(l) => l.0,
            }),
        }
    }
}
