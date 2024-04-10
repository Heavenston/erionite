use std::sync::{atomic::{self, AtomicBool}, Arc, Mutex, Weak};

#[derive(Debug)]
struct TaskInner<T> {
    out: Mutex<Option<T>>,
    done: AtomicBool,
}

#[derive(Debug, Clone)]
pub struct TaskHandle<T> {
    inner: Weak<TaskInner<T>>,
}

impl<T> TaskHandle<T> {
    pub fn canceled(&self) -> bool {
        self.inner.strong_count() == 0
    }

    pub fn finished(&self) -> bool {
        let Some(i) = self.inner.upgrade()
        else { return true; };
        i.done.load(atomic::Ordering::Relaxed)
    }

    pub fn finish(&self, val: T) -> bool {
        let Some(i) = self.inner.upgrade()
        else { return false; };
        if i.done.load(atomic::Ordering::Relaxed) {
            return false;
        }
        let mut lock = i.out.lock().unwrap();
        if lock.is_some() {
            return false;
        }

        *lock = Some(val);
        i.done.store(true, atomic::Ordering::Relaxed);
        
        true
    }
}

/// Drop = cancel
#[derive(Debug)]
pub struct Task<T> {
    inner: Arc<TaskInner<T>>,
}

impl<T> Task<T> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(TaskInner::<T> {
                out: Default::default(),
                done: AtomicBool::new(false),
            }),
        }
    }

    pub fn handle(&self) -> TaskHandle<T> {
        TaskHandle { inner: Arc::downgrade(&self.inner) }
    }

    pub fn finished(&self) -> bool {
        self.inner.done.load(atomic::Ordering::Relaxed)
    }

    pub fn join(self) -> T {
        assert!(self.finished());

        self.inner.out.lock().unwrap().take()
            .expect("marked as finished")
    }
}
