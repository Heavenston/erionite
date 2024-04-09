use std::{cell::UnsafeCell, sync::{atomic::{self, AtomicBool}, Arc}};

#[derive(Debug)]
struct TaskInner<T> {
    canceled: AtomicBool,
    out: UnsafeCell<Option<T>>,
    done: AtomicBool,
}

unsafe impl<T: Sync> Sync for TaskInner<T> { }

#[derive(Debug)]
pub struct Task<T> {
    inner: Arc<TaskInner<T>>,
}

impl<T> Task<T> {
    pub fn is_finished(&self) -> bool {
        self.inner.done.load(atomic::Ordering::Relaxed)
    }

    pub fn join(&mut self) -> T {
        assert!(self.is_finished());
        unsafe{&mut *self.inner.out.get()}.take().expect("Cannot join multiple times")
    }
}

impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        self.inner.canceled.store(true, atomic::Ordering::Relaxed);
    }
}

pub fn spawn<T, F>(f: F) -> Task<T>
    where T: Send + Sync + 'static,
          F: FnOnce() -> T + Send + Sync + 'static,
{
    let inner = Arc::new(TaskInner::<T> {
        canceled: AtomicBool::new(false),
        out: Default::default(),
        done: AtomicBool::new(false),
    });
    let inner_ = Arc::clone(&inner);

    rayon::spawn(move || {
        if inner_.canceled.load(atomic::Ordering::Relaxed) {
            return;
        }
        let out = f();
        unsafe{
            *inner_.out.get() = Some(out);
        };
        inner_.done.store(true, atomic::Ordering::Relaxed);
    });

    Task {
        inner,
    }
}
