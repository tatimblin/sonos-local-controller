use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{
    topology::{topology_item::TopologyItem, topology_list::TopologyList},
    views::ViewType,
};

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
    pub topology_ref: Option<HashMap<String, usize>>,
    pub highlight: Option<TopologyItem>,
    pub selected_speaker_ip: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            view: ViewType::Startup,
            status_message: "loading...".to_owned(),
            topology: None,
            topology_ref: None,
            highlight: None,
            selected_speaker_ip: None,
        }
    }
}

impl AppState {
    pub fn is_speaker_highlighted(&self, uuid: &str) -> bool {
        self.highlight.as_ref().map(|s| s.get_uuid()) == Some(uuid)
    }

    pub fn is_speaker_selected(&self, uuid: &str) -> bool {
        self.selected_speaker_ip.as_ref().map(|s| s.as_str()) == Some(uuid)
    }

    pub fn get_speaker_display_state(&self, uuid: &str) -> SpeakerDisplayState {
        match (
            self.is_speaker_highlighted(uuid),
            self.is_speaker_selected(uuid),
        ) {
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
