
use std::fmt::Debug;
use either::Either;

pub trait InternalData: Debug + Sized {
}

pub trait Data: Debug + Sized {
    type Internal: InternalData;
}

#[allow(type_alias_bounds)]
pub type EitherData<D: Data> = Either<D::Internal, D>;
#[allow(type_alias_bounds)]
pub type EitherDataRef<'a, D: Data> = Either<&'a D::Internal, &'a D>;
#[allow(type_alias_bounds)]
pub type EitherDataMut<'a, D: Data> = Either<& 'a mut D::Internal, & 'a mut D>;

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

impl Data for () {
    type Internal = ();
}

impl InternalData for () {  }

impl SplittableData for () {
    fn split(self) -> (Self::Internal, [Self; 8]) {
        ((), [(); 8])
    }
}

macro_rules! impl_tuple {
    (
        $($num:tt => $name:tt),*$(,)?
    ) => {
        impl<$($name),*> Data for ($($name),*,)
            where $($name: Data),*
        {
            type Internal = ($($name::Internal),*,);
        }

        impl<$($name),*> InternalData for ($($name),*,)
            where $($name: InternalData),*
        {  }

        impl<$($name),*> SplittableData for ($($name),*,)
            where $($name: SplittableData),*
        {
            fn split(self) -> (Self::Internal, [Self; 8]) {
                use ::itertools::Itertools;

                let vals = (
                    $(self.$num.split()),*,
                );

                let ints = (
                    $(vals.$num.1.into_iter().collect_tuple::<(
                        $name, $name, $name, $name,
                        $name, $name, $name, $name,
                    )>().expect("array of 8 elements")),*,
                );

                (
                    ($(vals.$num.0),*,),
                    [
                        ($(ints.$num.0),*,), ($(ints.$num.1),*,),
                        ($(ints.$num.2),*,), ($(ints.$num.3),*,),
                        ($(ints.$num.4),*,), ($(ints.$num.5),*,),
                        ($(ints.$num.6),*,), ($(ints.$num.7),*,),
                    ]
                )
            }
        }

        impl<$($name),*,> MergeableData for ($($name),*,)
            where $($name: MergeableData),*
        {
            fn can_merge(
                this: &Self::Internal,
                children: [&Self; 8]
            ) -> bool {
                $(
                    $name::can_merge(&this.$num, children.map(|x| &x.$num))
                )&&*
            }

            fn merge(
                this: Self::Internal,
                children: [Self; 8]
            ) -> Self {
                use ::itertools::Itertools;

                let children = children.into_iter().collect_tuple::<(
                    Self, Self, Self, Self,
                    Self, Self, Self, Self,
                )>().expect("array of 8 elements");

                let children = (
                    $([
                        children.0.$num, children.1.$num,
                        children.2.$num, children.3.$num,
                        children.4.$num, children.5.$num,
                        children.6.$num, children.7.$num,
                    ]),*,
                );

                ($($name::merge(this.$num, children.$num)),*,)
            }
        }

        impl<$($name),*> AggregateData for ($($name),*,)
            where $($name: AggregateData),*
        {
            fn aggregate<'a>(
                children: [EitherDataRef<Self>; 8]
            ) -> Self::Internal {
                ($(
                    $name::aggregate(children.map(|x| 
                        x.map_left(|x| &x.$num)
                         .map_right(|x| &x.$num)
                    ))
                ),*,)
            }
        }
    };
}

impl_tuple!(0 => A);
impl_tuple!(0 => A, 1 => B);
impl_tuple!(0 => A, 1 => B, 2 => C);
impl_tuple!(0 => A, 1 => B, 2 => C, 3 => D);
impl_tuple!(0 => A, 1 => B, 2 => C, 3 => D, 4 => E);
impl_tuple!(0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F);
impl_tuple!(0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G);
impl_tuple!(0 => A, 1 => B, 2 => C, 3 => D, 4 => E, 5 => F, 6 => G, 7 => H);
