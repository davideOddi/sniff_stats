use std::{sync::mpsc::{Receiver, Sender},sync::{Arc, Mutex}, thread::{self, JoinHandle}};
use notify::RecommendedWatcher;

use crate::model::PacketData;

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

pub struct ThreadWithState<T> {
    pub base: Thread,
    pub state: Arc<Mutex<T>>,
}

impl<T> ThreadWithState<T>
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

        return ThreadWithState {
            base: thread,
            state,
        }
    }

    pub fn join(self) {
        self.base.join();
    }

}

pub struct ThreadWorker {
    pub base: Thread,
}

impl ThreadWorker {
    pub fn join(self) {
        self.base.join();
    }

    pub fn new(
        name: &str,
        receiver: Arc<Mutex<Receiver<(String, String)>>>,
        packet_sender: Arc<std::sync::mpsc::Sender<Vec<PacketData>>>,
        worker_fn: Arc<dyn Fn(String, String) -> Result<Vec<PacketData>, Box<dyn std::error::Error>> + Send + Sync + 'static>,
    ) -> Self {
        let thread_name = name.to_string();
        let thread_name_clone = thread_name.clone();
        let worker_fn_clone = Arc::clone(&worker_fn);
        let packet_sender_clone = Arc::clone(&packet_sender);
        let receiver_clone = Arc::clone(&receiver);

        let job = move || {
            loop {
                let maybe_job = {
                    let lock = receiver_clone.lock().unwrap();
                    lock.recv()
                };

                match maybe_job {
                    Ok((input, output)) => {
                        match worker_fn_clone(input.clone(), output.clone()) {
                            Ok(packets) => {
                                if let Err(e) = packet_sender_clone.send(packets) {
                                    eprintln!("[{}] Errore invio packet: {}", thread_name, e);
                                    break;
                                }
                            }
                            Err(e) => {
                                eprintln!("[{}] Errore job {} → {}: {}", thread_name, input, output, e);
                            }
                        }
                    }
                    Err(_) => {
                        println!("[{}] Nessun altro job. Terminazione.", thread_name);
                        break;
                    }
                }
            }
        };

        let thread = Thread::new(&thread_name_clone, job);
        Self { base: thread }
    }
}