mod task;
pub use task::*;

pub fn spawn<T, F>(f: F) -> Task<T>
    where T: Send + Sync + 'static,
          F: FnOnce() -> T + Send + Sync + 'static,
{
    let task = Task::new();
    let handle = task.handle();

    rayon::spawn(move || {
        if handle.canceled() {
            return;
        }
        let out = f();
        handle.finish(out);
    });

    task
}
