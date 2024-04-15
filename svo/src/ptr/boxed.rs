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

impl<D> From<Box<Cell<D, BoxPtr<D>>>> for BoxPtr<D>
    where D: Data,
{
    fn from(value: Box<Cell<D, BoxPtr<D>>>) -> Self {
        Self(value)
    }
}

impl<D> From<Cell<D, BoxPtr<D>>> for BoxPtr<D>
    where D: Data,
{
    fn from(value: Cell<D, BoxPtr<D>>) -> Self {
        Self(Box::new(value))
    }
}

impl<D> Deref for BoxPtr<D>
    where D: Data,
{
    type Target = Cell<D, BoxPtr<D>>;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<D> SvoPtr<D> for BoxPtr<D>
    where D: Data,
{
    fn new(value: Cell<D, Self>) -> Self {
        BoxPtr(Box::new(value))
    }

    fn make_mut(&mut self) -> &mut Cell<D, Self> {
        &mut *self.0
    }

    fn into_inner(self) -> Cell<D, Self> {
        *self.0
    }
}

