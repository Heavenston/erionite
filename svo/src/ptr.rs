mod arc;
pub use arc::*;
mod boxed;
pub use boxed::*;
mod bumpbox;
pub use bumpbox::*;

use std::ops::Deref;

use super::*;

pub trait SvoPtr<D: Data>: Sized + Deref<Target = Cell<D, Self>> { }

pub trait MutableSvoPtr<D: Data>: SvoPtr<D> {
    /// Explicit DerefMut as it can be costly like with Arc::make_mut
    fn make_mut(&mut self) -> &mut Cell<D, Self>;
}

pub trait OwnedSvoPtr<D: Data>: SvoPtr<D> {
    fn new(value: Cell<D, Self>) -> Self;
    fn into_inner(self) -> Cell<D, Self>;
}
