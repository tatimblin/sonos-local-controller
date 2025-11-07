use crate::StateChange;

pub struct TopologyChanges {
  changes: Vec<StateChange>,
}

impl TopologyChanges {
 pub fn new() -> Self {
    Self { changes: Vec::new() }
  }

  pub fn add(&mut self, change: StateChange) {
    self.changes.push(change);
  }

  pub fn into_vec(self) -> Vec<StateChange> {
    self.changes
  }

  fn len(&self) -> usize {
    self.changes.len()
  }
}
