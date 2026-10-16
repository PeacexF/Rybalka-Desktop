//! Owns the [`World`] on a dedicated thread and steps it at a fixed rate.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use rybalka_core::sim::TICK_DT;
use rybalka_core::{Snapshot, World};
use tauri::ipc::Channel;

pub enum Command {
    Subscribe(Channel<Snapshot>),
}

/// Handle stored in Tauri state; sends commands to the game thread.
pub struct GameHandle {
    tx: Sender<Command>,
}

impl GameHandle {
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel();
        thread::Builder::new()
            .name("game".into())
            .spawn(move || run(rx))
            .expect("failed to spawn game thread");
        Self { tx }
    }

    pub fn send(&self, cmd: Command) -> Result<(), String> {
        self.tx
            .send(cmd)
            .map_err(|_| "game thread is gone".to_string())
    }
}

fn run(rx: Receiver<Command>) {
    let tick = Duration::from_secs_f64(TICK_DT);
    let mut world = World::new();
    let mut subscriber: Option<Channel<Snapshot>> = None;
    let mut next = Instant::now() + tick;

    loop {
        // Drain commands until the next tick is due.
        loop {
            let timeout = next.saturating_duration_since(Instant::now());
            match rx.recv_timeout(timeout) {
                Ok(Command::Subscribe(ch)) => subscriber = Some(ch),
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }

        world.step();
        if let Some(ch) = &subscriber
            && ch.send(world.snapshot()).is_err()
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
