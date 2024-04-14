use std::{any::Any, ops::Deref, sync::{atomic::{self, AtomicBool, AtomicU32}, Arc, Mutex, MutexGuard}};
use atomic::Ordering::Relaxed;

#[derive(derivative::Derivative)]
#[derivative(Debug, Default(bound = ""))]
struct LockedTaskInner<T> {
    /// used to make a task dependant of another and such keep it alive
    /// so they are only here for ther destructors to run when the task is cancelled
    parents: Vec<Box<dyn Any + Send + Sync + 'static>>,

    #[derivative(Debug="ignore")]
    thens: Vec<Box<dyn FnOnce(&T) + Send + Sync + 'static>>,
    out: Option<T>,
}

#[derive(Debug)]
pub struct TaskShared<T> {
    inner: Mutex<LockedTaskInner<T>>,
    done: AtomicBool,

    owner_count: AtomicU32,
}

impl<T> TaskShared<T> {
    pub fn finished(&self) -> bool {
        self.done.load(Relaxed)
    }

    pub fn add_parent<O: Send + Sync + 'static>(&self, parent: Task<O>) {
        let mut lock = self.inner.lock().unwrap();
        if self.owner_count.load(Relaxed) == 0 {
            return;
        }
        lock.parents.push(Box::new(parent) as _);
    }

    /// The given function is called when the task is finished by the
    /// thread that calls [TaskHandle::finish], or by this thread right now if the task
    /// is already finished and the inner value has not been taken by
    /// a call to [Task::try_join]
    pub fn then(&self, then: impl FnOnce(&T) + Send + Sync + 'static) {
        if self.finished() {
            if let Some(v) = &self.inner.lock().unwrap().out {
                then(v);
            }

            return
        }

        self.inner.lock().unwrap().thens.push(Box::new(then));
    }

    pub fn peek<'a>(&'a self) -> Option<impl Deref<Target = T> + 'a> {
        struct InnerRef<'a, T> {
            guard: MutexGuard<'a, LockedTaskInner<T>>,
        }

        impl<'a, T> Deref for InnerRef<'a, T> {
            type Target = T;

            fn deref(&self) -> &Self::Target {
                &self.guard.out.as_ref().expect("checked before")
            }
        }

        let guard = self.inner.lock().unwrap();
        if guard.out.is_none() {
            return None;
        }
        Some(InnerRef { guard })
    }
}

#[derive(Debug, Clone)]
pub struct TaskHandle<T> {
    shared: Arc<TaskShared<T>>,
}

impl<T> TaskHandle<T> {
    // would not make sense for Task to have this method as if a Task exist it
    // is not canceled
    pub fn canceled(&self) -> bool {
        self.shared.owner_count.load(Relaxed) == 0
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
            shared: Arc::clone(&self.shared),
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

        for then in lock.thens.drain(..) {
            then(&val);
        }

        lock.out = Some(val);
        self.shared.done.store(true, Relaxed);
        
        true
    }

    /// Paniking version of [try_finish]
    pub fn finish(&self, val: T) {
        assert!(self.try_finish(val), "task already finished");
    }

    /// Util function,
    /// Like [Self::task] but returns a task that finishes with the then
    /// function's return value.
    ///
    /// Sets the new task's parent as this one to prevent cancelation of it.
    pub fn then_task<Out: Send + Sync + 'static>(
        &self, then: impl FnOnce(&T) -> Out + Send + Sync + 'static
    ) -> Task<Out>
        where T: Send + Sync + 'static
    {
        let task = Task::new();
        let handle = task.handle();
        if let Some(parent) = self.upgrade() {
            task.add_parent(parent);
        }
        self.then(move |val| {
            let res = then(val);
            handle.finish(res);
        });
        task
    }
}

impl<T> Deref for TaskHandle<T> {
    type Target = TaskShared<T>;

    fn deref(&self) -> &Self::Target {
        &*self.shared
    }
}

/// can be cloned
/// all clones dropped -> task cancelled
#[derive(Debug)]
pub struct Task<T> {
    shared: Arc<TaskShared<T>>,
}

impl<T> Task<T> {
    pub fn new() -> Self {
        Self {
            shared: Arc::new(TaskShared::<T> {
                inner: Default::default(),
                done: AtomicBool::new(false),
                owner_count: AtomicU32::new(1),
            }),
        }
    }

    pub fn handle(&self) -> TaskHandle<T> {
        TaskHandle { shared: Arc::clone(&self.shared) }
    }

    /// The task will never be cancelled
    pub fn detach(&self) {
        self.shared.owner_count.fetch_add(1, Relaxed);
    }

    pub fn try_join(&self) -> Option<T> {
        if !self.finished() {
            return None;
        }

        self.shared.inner.lock().unwrap().out.take()
    }

    /// Util function,
    /// Like [Self::task] but returns a task that finishes with the then
    /// function's return value.
    ///
    /// Sets the new task's parent as this one to prevent cancelation of it.
    pub fn then_task<Out: Send + Sync + 'static>(
        &self, then: impl FnOnce(&T) -> Out + Send + Sync + 'static
    ) -> Task<Out>
        where T: Send + Sync + 'static
    {
        let task = Task::new();
        let handle = task.handle();
        task.add_parent(self.clone());
        self.then(move |val| {
            let res = then(val);
            handle.finish(res);
        });
        task
    }
}

impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        let mut lock = self.inner.lock().unwrap();
        let owners = self.shared.owner_count.fetch_sub(1, Relaxed) - 1;
        if owners == 0 {
            // drop all parents
            lock.parents = vec![];
        }
    }
}

impl<T> Clone for Task<T> {
    fn clone(&self) -> Self {
        self.shared.owner_count.fetch_add(1, Relaxed);
        Self {
            shared: Arc::clone(&self.shared),
        }
    }
}

impl<T> Deref for Task<T> {
    type Target = TaskShared<T>;

    fn deref(&self) -> &Self::Target {
        &*self.shared
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple() {
        let task = Task::<u32>::new();

        assert!(!task.finished());

        let handle = task.handle();

        assert!(!handle.finished());
        assert!(!handle.canceled());
        assert!(!task.finished());

        assert_eq!(task.peek().map(|v| *v), None);
        assert_eq!(handle.peek().map(|v| *v), None);

        assert!(handle.try_finish(5));

        assert_eq!(task.peek().map(|v| *v), Some(5));
        assert_eq!(handle.peek().map(|v| *v), Some(5));

        assert!(handle.finished());
        assert!(!handle.canceled());
        assert!(task.finished());

        assert!(!handle.try_finish(10));

        assert_eq!(task.peek().map(|v| *v), Some(5));
        assert_eq!(handle.peek().map(|v| *v), Some(5));

        assert!(handle.finished());
        assert!(!handle.canceled());
        assert!(task.finished());

        assert_eq!(task.try_join(), Some(5));

        assert_eq!(task.peek().map(|v| *v), None);
        assert_eq!(handle.peek().map(|v| *v), None);
    }

    #[test]
    fn test_cancellation() {
        let task = Task::<u32>::new();

        let handle = task.handle();

        assert!(!handle.canceled());
        assert!(!handle.finished());
        assert!(handle.upgrade().is_some());
        assert_eq!(handle.peek().map(|v| *v), None);

        drop(task);

        assert!(handle.canceled());
        assert!(!handle.finished());
        assert!(handle.upgrade().is_none());
        assert_eq!(handle.peek().map(|v| *v), None);

        assert!(handle.try_finish(5));

        assert!(handle.canceled());
        assert!(handle.finished());
        assert!(handle.upgrade().is_none());
        assert_eq!(handle.peek().map(|v| *v), Some(5));
    }

    #[test]
    fn test_then_simple() {
        let task1 = Task::<&'static str>::new();
        let task2 = Task::<&'static str>::new();
        let task3 = Task::<&'static str>::new();
        let task4 = Task::<&'static str>::new();

        task1.then({
            let handle = task2.handle();
            move |&val1| {
                assert_eq!(val1, "task1");
                handle.finish("task2");
            }
        });

        assert_eq!(task1.peek().map(|x| *x), None);
        assert_eq!(task2.peek().map(|x| *x), None);
        assert_eq!(task3.peek().map(|x| *x), None);

        task1.handle().finish("task1");

        assert_eq!(task1.peek().map(|x| *x), Some("task1"));
        assert_eq!(task2.peek().map(|x| *x), Some("task2"));
        assert_eq!(task3.peek().map(|x| *x), None);

        task2.then({
            let handle = task3.handle();
            move |&val2| {
                assert_eq!(val2, "task2");
                handle.finish("task3");
            }
        });

        assert_eq!(task1.peek().map(|x| *x), Some("task1"));
        assert_eq!(task2.peek().map(|x| *x), Some("task2"));
        assert_eq!(task3.peek().map(|x| *x), Some("task3"));

        let _ = task1.try_join();

        assert_eq!(task1.peek().map(|x| *x), None);
        assert_eq!(task2.peek().map(|x| *x), Some("task2"));
        assert_eq!(task3.peek().map(|x| *x), Some("task3"));
        assert_eq!(task4.peek().map(|x| *x), None);

        task1.then({
            let handle = task4.handle();
            move |&val1| {
                assert_eq!(val1, "task1");
                handle.finish("task4");
            }
        });

        assert_eq!(task1.peek().map(|x| *x), None);
        assert_eq!(task2.peek().map(|x| *x), Some("task2"));
        assert_eq!(task3.peek().map(|x| *x), Some("task3"));
        assert_eq!(task4.peek().map(|x| *x), None);
    }

    #[test]
    fn test_then_task() {
        let task1 = Task::<u32>::new();
        let handle1 = task1.handle();
        let task2 = task1.then_task(|s| format!("{s} v2"));

        handle1.finish(5);

        assert_eq!(task1.peek().map(|x| *x), Some(5));
        assert_eq!(task2.peek().map(|x| x.clone()), Some("5 v2".to_string()));

        let task3 = task1.then_task(|s| format!("{s} v3"));

        assert_eq!(task3.peek().map(|x| x.clone()), Some("5 v3".to_string()));
    }

    #[test]
    fn test_then_task_handle() {
        let task1 = Task::<u32>::new();
        let handle1 = task1.handle();
        let task2 = task1.handle().then_task(|s| format!("{s} v2"));

        handle1.finish(5);

        assert_eq!(task1.peek().map(|x| *x), Some(5));
        assert_eq!(task2.peek().map(|x| x.clone()), Some("5 v2".to_string()));

        let task3 = task1.handle().then_task(|s| format!("{s} v3"));

        assert_eq!(task3.peek().map(|x| x.clone()), Some("5 v3".to_string()));
    }

    #[test]
    fn test_then_task_cancel() {
        let task1 = Task::<u32>::new();
        let handle1 = task1.handle();
        let task2 = task1.then_task(|s| format!("{s} v2"));

        assert!(!handle1.canceled());
        drop(task1);
        assert!(!handle1.canceled());
        drop(task2);
        assert!(handle1.canceled());
    }
}
