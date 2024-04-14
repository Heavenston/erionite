use std::sync::{atomic::{self, AtomicBool, AtomicU32}, Arc, Mutex};
use atomic::Ordering::Relaxed;

#[derive(Debug)]
struct LockedTaskInner<T> {
    out: Option<T>,
}

impl<T> Default for LockedTaskInner<T> {
    fn default() -> Self {
        Self {
            out: None,
        }
    }
}

#[derive(Debug)]
struct TaskShared<T> {
    inner: Mutex<LockedTaskInner<T>>,
    done: AtomicBool,

    owner_count: AtomicU32,
}

#[derive(Debug, Clone)]
pub struct TaskHandle<T> {
    shared: Arc<TaskShared<T>>,
}

impl<T> TaskHandle<T> {
    pub fn canceled(&self) -> bool {
        self.shared.owner_count.load(Relaxed) == 0
    }

    pub fn finished(&self) -> bool {
        self.canceled() || self.shared.done.load(Relaxed)
    }

    pub fn upgrade(&self) -> Option<Task<T>> {
        let mut owner_count = self.shared.owner_count.load(Relaxed);
        loop {
            if owner_count == 0 {
                return None;
            }
            match self.shared.owner_count.compare_exchange(
                owner_count, owner_count+1, Relaxed, Relaxed
            ) {
                Ok(_) => break,
                Err(e) => owner_count = e,
            }
        }

        Some(Task {
            inner: Arc::clone(&self.shared),
        })
    }

    /// This doesn't check if the task is cancelled or not
    /// returns true if the task wasn't already finished before
    pub fn try_finish(&self, val: T) -> bool {
        if self.shared.done.load(Relaxed) {
            return false;
        }
        let mut lock = self.shared.inner.lock().unwrap();
        if lock.out.is_some() {
            return false;
        }

        lock.out = Some(val);
        self.shared.done.store(true, Relaxed);
        
        true
    }

    /// Paniking version of [try_finish]
    pub fn finish(&self, val: T) {
        assert!(self.try_finish(val), "task already finished");
    }
}

/// can be cloned
/// all clones droped -> task cancel
#[derive(Debug)]
pub struct Task<T> {
    inner: Arc<TaskShared<T>>,
}

impl<T> Task<T> {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(TaskShared::<T> {
                inner: Default::default(),
                done: AtomicBool::new(false),
                owner_count: AtomicU32::new(1),
            }),
        }
    }

    pub fn handle(&self) -> TaskHandle<T> {
        TaskHandle { shared: Arc::clone(&self.inner) }
    }

    pub fn finished(&self) -> bool {
        self.inner.done.load(Relaxed)
    }

    /// The task will never be cancelled
    pub fn detach(&self) {
        self.inner.owner_count.fetch_add(1, Relaxed);
    }

    pub fn try_join(&self) -> Option<T> {
        if !self.finished() {
            return None;
        }

        self.inner.inner.lock().unwrap().out.take()
    }
}

impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        self.inner.owner_count.fetch_sub(1, Relaxed);
    }
}

impl<T> Clone for Task<T> {
    fn clone(&self) -> Self {
        self.inner.owner_count.fetch_add(1, Relaxed);
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

pub trait OptionTaskExt<T> {
    /// Util function to take the task out of the option if it is finished
    /// and urn try_join on it
    fn take_if_finished(&mut self) -> Option<T>;
}

impl<T> OptionTaskExt<T> for Option<Task<T>> {
    fn take_if_finished(&mut self) -> Option<T> {
        if self.as_ref().is_some_and(|t| t.finished()) {
            return self.take().and_then(|t| t.try_join());
        }

        None
    }
}
