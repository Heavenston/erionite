use super::*;
use crate::*;

/// A cell that uses Boxes as Pointers to sub cells
pub type RefCell<'a, D> = Cell<D, RefPtr<'a, D>>;

#[derive(Debug)]
pub struct RefPtr<'a, D>(pub &'a Cell<D, RefPtr<'a, D>>)
    where D: Data
;

impl<'a, D: Data> From<&'a RefCell<'a, D>> for RefPtr<'a, D> {
    fn from(value: &'a RefCell<'a, D>) -> Self {
        Self(value)
    }
}

impl<'a, D: Data> Deref for RefPtr<'a, D> {
    type Target = Cell<D, RefPtr<'a, D>>;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<'a, D: Data> SvoPtr<D> for RefPtr<'a, D> { }
