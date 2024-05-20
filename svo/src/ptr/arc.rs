use super::*;
use crate::*;

#[derive(Debug, Clone)]
pub struct ArcPtr<D: Data>(pub Arc<Cell<D, ArcPtr<D>>>);

impl<D> From<Arc<Cell<D, ArcPtr<D>>>> for ArcPtr<D>
    where D: Data + Clone,
          D::Internal: Clone,
{
    fn from(value: Arc<Cell<D, ArcPtr<D>>>) -> Self {
        Self(value)
    }
}

impl<D> From<Cell<D, ArcPtr<D>>> for ArcPtr<D>
    where D: Data + Clone,
          D::Internal: Clone,
{
    fn from(value: Cell<D, ArcPtr<D>>) -> Self {
        Self(Arc::new(value))
    }
}

impl<D: Data> Deref for ArcPtr<D> {
    type Target = Cell<D, ArcPtr<D>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<D: Data> SvoPtr<D> for ArcPtr<D> { }

impl<D> MutableSvoPtr<D> for ArcPtr<D>
    where D: Data + Clone,
          D::Internal: Clone,
{
    fn make_mut(&mut self) -> &mut Cell<D, Self> {
        Arc::make_mut(&mut self.0)
    }
}

impl<D> OwnedSvoPtr<D> for ArcPtr<D>
    where D: Data + Clone,
          D::Internal: Clone,
{
    fn new(value: Cell<D, Self>) -> Self {
        ArcPtr(Arc::new(value))
    }

    fn into_inner(self) -> Cell<D, Self> {
        Arc::try_unwrap(self.0).unwrap_or_else(|arc| (*arc).clone())
    }
}
