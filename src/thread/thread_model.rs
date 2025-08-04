use std::{sync::mpsc::Receiver, sync::{Arc, Mutex}, thread::{self, JoinHandle}};
use notify::RecommendedWatcher;

pub struct Thread {
    pub name: String,
    process: Option<JoinHandle<()>>,
}

impl Thread {
    pub fn new<F>(name: &str, job: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        println!("Avvio thread: {}", name);
        let process = thread::Builder::new()
            .name(name.to_string())
            .spawn(job)
            .expect(&format!("Errore nell'avvio del thread {}", name));

        Self {
            name: name.to_string(),
            process: Some(process),
        }
    }

    pub fn join(self) {
        if let Some(process) = self.process {
            let _ = process.join();
        }
    }
}

pub struct ThreadWatcher {
    pub(crate) base: Thread,
    pub(crate) _watcher: RecommendedWatcher,
}

impl ThreadWatcher {
    pub fn join(self) {
        self.base.join();
    }
}

pub struct ThreadAggregator<T> {
    pub base: Thread,
    pub state: Arc<Mutex<T>>,
}

impl<T> ThreadAggregator<T>
where
    T: Send + 'static,
{
    pub fn new<Item, F>(
        name: &str,
        initial_state: T,
        receiver: Receiver<Vec<Item>>,
        update_fn: F,
    ) -> Self
    where
        Item: Send + 'static,
        F: Fn(&mut T, Vec<Item>) + Send + Sync + 'static,
    {
        let state = Arc::new(Mutex::new(initial_state));
        let thread_state = Arc::clone(&state);
        let thread_name = name.to_string();

        let thread = Thread::new(name, move || {
            while let Ok(batch) = receiver.recv() {
                let mut locked = thread_state.lock().unwrap();
                update_fn(&mut *locked, batch);
            }
            println!("[{}] Thread Terminato", thread_name);
        });

        return ThreadAggregator {
            base: thread,
            state,
        }
    }

    pub fn join(self) {
        self.base.join();
    }

}