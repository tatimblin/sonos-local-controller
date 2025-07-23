use std::sync::{Arc, Mutex};

use crate::{topology::topology_list::TopologyList, views::ViewType};

use super::reducers::{app_reducer, AppAction};

#[derive(Debug, Clone, PartialEq)]
pub enum SpeakerDisplayState {
    Normal,
    Active,
    Locked,
    ActiveAndLocked,
}

pub struct AppState {
    pub view: ViewType,
    pub status_message: String,
    pub topology: Option<TopologyList>,
    pub active_speaker_uuid: Option<String>,
    pub locked_speaker_uuid: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            view: ViewType::Startup,
            status_message: "loading...".to_owned(),
            topology: None,
            active_speaker_uuid: None,
            locked_speaker_uuid: None,
        }
    }
}

impl AppState {
    pub fn is_speaker_active(&self, uuid: &str) -> bool {
        self.active_speaker_uuid.as_ref().map(|s| s.as_str()) == Some(uuid)
    }

    pub fn is_speaker_locked(&self, uuid: &str) -> bool {
        self.locked_speaker_uuid.as_ref().map(|s| s.as_str()) == Some(uuid)
    }

    pub fn get_speaker_display_state(&self, uuid: &str) -> SpeakerDisplayState {
        match (self.is_speaker_active(uuid), self.is_speaker_locked(uuid)) {
            (true, true) => SpeakerDisplayState::ActiveAndLocked,
            (true, false) => SpeakerDisplayState::Active,
            (false, true) => SpeakerDisplayState::Locked,
            (false, false) => SpeakerDisplayState::Normal,
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

    pub fn with_state<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&AppState) -> T,
    {
        let state = self.state.lock().unwrap();
        f(&state)
    }
}
