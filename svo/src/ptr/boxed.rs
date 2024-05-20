use super::*;
use crate::*;

/// A cell that uses Boxes as Pointers to sub cells
pub type BoxCell<D> = Cell<D, BoxPtr<D>>;

#[derive(Debug)]
pub struct BoxPtr<D>(pub Box<Cell<D, BoxPtr<D>>>)
    where D: Data
;

impl<D> Clone for BoxPtr<D>
    where D: Data + Clone,
          D::Internal: Clone,
{
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<D: Data> From<Box<Cell<D, BoxPtr<D>>>> for BoxPtr<D> {
    fn from(value: Box<Cell<D, BoxPtr<D>>>) -> Self {
        Self(value)
    }
}

impl<D: Data> From<Cell<D, BoxPtr<D>>> for BoxPtr<D> {
    fn from(value: Cell<D, BoxPtr<D>>) -> Self {
        Self(Box::new(value))
    }
}

impl<D: Data> Deref for BoxPtr<D> {
    type Target = Cell<D, BoxPtr<D>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<D: Data> SvoPtr<D> for BoxPtr<D> { }

impl<D: Data> MutableSvoPtr<D> for BoxPtr<D> {
    fn make_mut(&mut self) -> &mut Cell<D, Self> {
        &mut self.0
    }
}

impl<D: Data> OwnedSvoPtr<D> for BoxPtr<D> {
    fn new(value: Cell<D, Self>) -> Self {
        BoxPtr(Box::new(value))
    }

    fn into_inner(self) -> Cell<D, Self> {
        *self.0
    }
}
