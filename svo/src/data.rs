
use std::fmt::Debug;
use either::Either;

pub trait InternalData: Debug + Sized + Default {
}

pub trait Data: Debug + Sized + Default {
    type Internal: InternalData;
}

#[allow(type_alias_bounds)]
pub type EitherData<D: Data> = Either<D::Internal, D>;
#[allow(type_alias_bounds)]
pub type EitherDataRef<'a, D: Data> = Either<&'a D::Internal, &'a D>;
#[allow(type_alias_bounds)]
pub type EitherDataMut<'a, D: Data> = Either<& 'a mut D::Internal, & 'a mut D>;

impl Data for () {
    type Internal = ();
}

impl InternalData for () {  }

impl SplittableData for () {
    fn split(self) -> (Self::Internal, [Self; 8]) {
        ((), [(); 8])
    }
}

pub trait SplittableData: Data {
    fn split(self) -> (Self::Internal, [Self; 8]);
}

pub trait MergeableData: Data {
    fn can_merge(
        this: &Self::Internal,
        children: [&Self; 8]
    ) -> bool;
    fn merge(
        this: Self::Internal,
        children: [Self; 8]
    ) -> Self;
}

pub trait AggregateData: Data {
    fn aggregate<'a>(
        children: [EitherDataRef<Self>; 8]
    ) -> Self::Internal;
}

impl<D: Data<Internal = ()>> AggregateData for D {
    fn aggregate<'a>(
        _d: [EitherDataRef<D>; 8]
    ) -> () { }
}
