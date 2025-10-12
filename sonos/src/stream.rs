use crate::models::StateChange;
use std::sync::mpsc;

pub struct EventStream {
  rx: mpsc::Receiver<StateChange>,
}

impl EventStream {
  pub fn new() -> (Self, mpsc::Sender<StateChange>) {
    let (tx, rx) = mpsc::channel();
    (Self { rx }, tx)
  }

  pub fn next(&self) -> Option<StateChange> {
    self.rx.recv().ok()
  }

  pub fn try_next(&self) -> Option<StateChange> {
    self.rx.try_recv().ok()
  }
}
