use std::ops::DerefMut;

use super::*;
use crate::*;

/// A cell that uses Boxes as Pointers to sub cells
pub type MutRefCell<'a, D> = Cell<D, MutRefPtr<'a, D>>;

#[derive(Debug)]
pub struct MutRefPtr<'a, D>(pub &'a mut Cell<D, MutRefPtr<'a, D>>)
    where D: Data
;

impl<'a, D: Data> From<&'a mut MutRefCell<'a, D>> for MutRefPtr<'a, D> {
    fn from(value: &'a mut MutRefCell<'a, D>) -> Self {
        Self(value)
    }
}

impl<'a, D: Data> Deref for MutRefPtr<'a, D> {
    type Target = Cell<D, MutRefPtr<'a, D>>;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<'a, D: Data> DerefMut for MutRefPtr<'a, D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl<'a, D: Data> SvoPtr<D> for MutRefPtr<'a, D> { }

impl<'a, D: Data> MutableSvoPtr<D> for MutRefPtr<'a, D> {
    fn make_mut(&mut self) -> &mut Cell<D, Self> {
        self.0
    }
}
