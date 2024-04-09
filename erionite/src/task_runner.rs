use std::sync::{atomic::{self, AtomicBool}, Arc, Mutex};

#[derive(Debug)]
struct TaskInner<T> {
    canceled: AtomicBool,
    out: Mutex<Option<T>>,
    done: AtomicBool,
}

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
        while Arc::strong_count(&self.inner) != 1 {
            std::hint::spin_loop();
        }

        Arc::get_mut(&mut self.inner)
            .expect("just confirmed we are the only one")
            .out.get_mut().unwrap().take()
            .expect("marked as finished")
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
        *inner_.out.lock().unwrap() = Some(out);
        inner_.done.store(true, atomic::Ordering::Relaxed);
    });

    Task {
        inner,
    }
}
