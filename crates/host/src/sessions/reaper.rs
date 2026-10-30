use std::{
    sync::{OnceLock, mpsc},
    thread,
};

type Task = Box<dyn FnOnce() + Send>;

pub fn reap(task: impl FnOnce() + Send + 'static) {
    static TASKS: OnceLock<mpsc::Sender<Task>> = OnceLock::new();
    let tasks = TASKS.get_or_init(|| {
        let (sender, tasks) = mpsc::channel::<Task>();
        thread::Builder::new()
            .name("lucent-reaper".into())
            .spawn(move || {
                for task in tasks {
                    task();
                }
            })
            .expect("couldn't start the worker reaper");
        sender
    });
    let dispatch = tracing::dispatcher::get_default(Clone::clone);
    tasks
        .send(Box::new(move || {
            let _guard = tracing::dispatcher::set_default(&dispatch);
            task();
        }))
        .expect("worker reaper stopped");
}
