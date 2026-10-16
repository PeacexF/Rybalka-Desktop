//! Owns the [`World`] on a dedicated thread and steps it at a fixed rate.

use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use rybalka_core::content::Content;
use rybalka_core::sim::TICK_DT;
use rybalka_core::{Snapshot, World};
use tauri::ipc::Channel;

type Job = Box<dyn FnOnce(&mut World) + Send>;

enum Command {
    Subscribe(Channel<Snapshot>),
    Run(Job),
}

/// Handle stored in Tauri state; sends work to the game thread.
pub struct GameHandle {
    tx: Sender<Command>,
}

impl GameHandle {
    pub fn spawn(content: Arc<Content>, seed: u64) -> Self {
        let (tx, rx) = mpsc::channel();
        thread::Builder::new()
            .name("game".into())
            .spawn(move || run(World::new(content, seed), rx))
            .expect("failed to spawn game thread");
        Self { tx }
    }

    pub fn subscribe(&self, ch: Channel<Snapshot>) -> Result<(), String> {
        self.tx
            .send(Command::Subscribe(ch))
            .map_err(|_| GONE.to_string())
    }

    /// Run `f` on the game thread between ticks and return its result.
    pub fn with_world<R: Send + 'static>(
        &self,
        f: impl FnOnce(&mut World) -> R + Send + 'static,
    ) -> Result<R, String> {
        let (tx, rx) = mpsc::sync_channel(1);
        self.tx
            .send(Command::Run(Box::new(move |w| {
                let _ = tx.send(f(w));
            })))
            .map_err(|_| GONE.to_string())?;
        rx.recv().map_err(|_| GONE.to_string())
    }
}

const GONE: &str = "game thread is gone";

fn run(mut world: World, rx: Receiver<Command>) {
    let tick = Duration::from_secs_f64(TICK_DT);
    let mut subscriber: Option<Channel<Snapshot>> = None;
    let mut next = Instant::now() + tick;

    loop {
        // Handle commands until the next tick is due.
        loop {
            let timeout = next.saturating_duration_since(Instant::now());
            match rx.recv_timeout(timeout) {
                Ok(Command::Subscribe(ch)) => subscriber = Some(ch),
                Ok(Command::Run(job)) => job(&mut world),
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }

        world.step();
        let snapshot = world.snapshot();
        if let Some(ch) = &subscriber
            && ch.send(snapshot).is_err()
        {
            subscriber = None;
        }

        next += tick;
        // If we fell far behind (e.g. machine slept), don't try to catch up.
        let now = Instant::now();
        if next + tick * 5 < now {
            next = now + tick;
        }
    }
}
