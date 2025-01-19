use std::sync::{ Arc, Mutex };
use crate::types::*;
use sonos::Speaker;

use super::reducers;

pub struct AppState {
  pub view: View,
  pub speakers: Vec<Speaker>,
  pub selected_speaker: Option<usize>,
}

impl Default for AppState {
  fn default() -> Self {
    Self {
      view: View::Startup,
      speakers: Vec::new(),
      selected_speaker: None,
    }
  }
}

pub struct Store {
  state: Arc<Mutex<AppState>>,
}

impl Store {
  pub fn new() -> Self {
    Self {
      state: Arc::new(Mutex::new(AppState::default())),
    }
  }

  pub fn dispatch(&self, action: AppAction) {
    let mut state = self.state.lock().unwrap();
    reducers::app_reducer(&mut state, action);
  }

  pub fn with_state<F, T>(&self, f: F) -> T
  where 
    F: FnOnce(&AppState) -> T
  {
    let state = self.state.lock().unwrap();
    f(&state)
  }
}
