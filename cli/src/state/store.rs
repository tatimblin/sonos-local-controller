use std::sync::{ Arc, Mutex };
use crate::types::*;
use sonos::System;

use super::reducers::{ AppAction, app_reducer };

pub struct AppState {
  pub view: View,
  pub status_message: String,
  pub topology: Option<Topology>,
  pub system: Option<Arc<System>>,
  pub selected_speaker_uuid: Option<String>,
}

impl Default for AppState {
  fn default() -> Self {
    Self {
      view: View::Startup,
      status_message: "loading...".to_owned(),
      topology: None,
      system: None,
      selected_speaker_uuid: None,
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
    app_reducer(&mut state, action);
  }

  pub fn with_state<F, T>(&self, f: F) -> T where F: FnOnce(&AppState) -> T {
    let state = self.state.lock().unwrap();
    f(&state)
  }
}