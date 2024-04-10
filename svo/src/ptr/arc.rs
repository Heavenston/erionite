use super::*;
use crate::*;

#[derive(Debug, Clone)]
pub struct ArcPtr<D>(pub Arc<Cell<D, ArcPtr<D>>>)
    where D: Data + Clone,
          D::Internal: Clone,
;

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

impl<D> Deref for ArcPtr<D>
    where D: Data + Clone,
          D::Internal: Clone,
{
    type Target = Cell<D, ArcPtr<D>>;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<D> SvoPtr<D> for ArcPtr<D>
    where D: Data + Clone,
          D::Internal: Clone,
{
    fn new(value: Cell<D, Self>) -> Self {
        ArcPtr(Arc::new(value))
    }

    fn make_mut(&mut self) -> &mut Cell<D, Self> {
        Arc::make_mut(&mut self.0)
    }
}
