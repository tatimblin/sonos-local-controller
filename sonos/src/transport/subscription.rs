use crate::models::{SpeakerId, StateChange};
use std::sync::mpsc;
use std::thread;

pub struct EventSubscriber {
  rx: mpsc::Receiver<StateChange>,
}

impl EventSubscriber {
  pub fn new() -> Self {
    let (_tx, rx) = mpsc::channel();

    Self { rx }
  }

  pub fn next_event(&self) -> Option<StateChange> {
    self.rx.recv().ok()
  }
}
