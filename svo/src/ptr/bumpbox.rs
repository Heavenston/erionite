use super::*;
use crate::*;

use bumpalo::boxed::Box as BumpBox;

/// A cell that uses Boxes as Pointers to sub cells
pub type BumpCell<'a, D> = Cell<D, BumpBoxPtr<'a, D>>;

#[derive(Debug)]
pub struct BumpBoxPtr<'a, D>(pub BumpBox<'a, Cell<D, BumpBoxPtr<'a, D>>>)
    where D: Data
;

impl<'a, D: Data> From<BumpBox<'a, BumpCell<'a, D>>> for BumpBoxPtr<'a, D> {
    fn from(value: BumpBox<'a, BumpCell<'a, D>>) -> Self {
        Self(value)
    }
}

impl<'a, D: Data> Deref for BumpBoxPtr<'a, D> {
    type Target = Cell<D, BumpBoxPtr<'a, D>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, D: Data> SvoPtr<D> for BumpBoxPtr<'a, D> { }

impl<'a, D: Data> MutableSvoPtr<D> for BumpBoxPtr<'a, D> {
    fn make_mut(&mut self) -> &mut Cell<D, Self> {
        &mut self.0
    }
}
