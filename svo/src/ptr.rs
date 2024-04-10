mod arc;
pub use arc::*;

use std::ops::Deref;

use super::*;

pub trait SvoPtr<D: Data>: Sized + Deref<Target = Cell<D, Self>> {
    fn new(value: Cell<D, Self>) -> Self;

    /// Explicit DerefMut as it can be costly like with Arc::make_mut
    fn make_mut(&mut self) -> &mut Cell<D, Self>;
}
