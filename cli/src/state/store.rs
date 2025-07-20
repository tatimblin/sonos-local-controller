use sonos::System;
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
    pub system: Option<Arc<System>>,
    pub active_speaker_uuid: Option<String>,
    pub locked_speaker_uuid: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            view: ViewType::Startup,
            status_message: "loading...".to_owned(),
            topology: None,
            system: None,
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
    discovery_system: Arc<Mutex<Option<System>>>,
}

impl Store {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(AppState::default())),
            discovery_system: Arc::new(Mutex::new(None)),
        }
    }

    pub fn new_with_system(system: System) -> Self {
        let store = Self::new();
        store.set_discovery_system(system);

        // Create command system immediately
        match System::new() {
            Ok(command_system) => {
                let system_arc = Arc::new(command_system);
                store.dispatch(AppAction::SetSystem(system_arc));
            }
            Err(e) => {
                log::error!(
                    "Failed to create command system during store initialization: {:?}",
                    e
                );
            }
        }

        store
    }

    pub fn set_discovery_system(&self, system: System) {
        let mut discovery_system = self.discovery_system.lock().unwrap();
        *discovery_system = Some(system);
    }

    pub fn with_discovery_system<F, T>(&self, f: F) -> Option<T>
    where
        F: FnOnce(&mut System) -> T,
    {
        let mut discovery_system = self.discovery_system.lock().unwrap();
        if let Some(ref mut system) = *discovery_system {
            Some(f(system))
        } else {
            None
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::{topology_item::TopologyItem, topology_list::TopologyList};
    use sonos::SpeakerCommand;

    fn create_test_topology() -> TopologyList {
        TopologyList {
            items: vec![
                TopologyItem::Speaker {
                    uuid: "speaker1".to_string(),
                },
                TopologyItem::Speaker {
                    uuid: "speaker2".to_string(),
                },
            ],
        }
    }

    #[test]
    fn test_store_new_creates_default_state() {
        let store = Store::new();

        store.with_state(|state| {
            assert_eq!(state.view, ViewType::Startup);
            assert_eq!(state.status_message, "loading...");
            assert!(state.topology.is_none());
            assert!(state.system.is_none());
            assert!(state.active_speaker_uuid.is_none());
            assert!(state.locked_speaker_uuid.is_none());
        });
    }

    #[test]
    fn test_store_dispatch_updates_state() {
        let store = Store::new();

        // Test status message update
        store.dispatch(AppAction::SetStatusMessage("Test message".to_string()));
        store.with_state(|state| {
            assert_eq!(state.status_message, "Test message");
        });

        // Test topology update
        let topology = create_test_topology();
        store.dispatch(AppAction::SetTopology(topology));
        store.with_state(|state| {
            assert!(state.topology.is_some());
            assert_eq!(state.view, ViewType::Control);
        });
    }

    #[test]
    fn test_store_speaker_display_state() {
        let store = Store::new();

        // Set active speaker
        store.dispatch(AppAction::SetActiveSpeaker("speaker1".to_string()));
        store.with_state(|state| {
            assert_eq!(
                state.get_speaker_display_state("speaker1"),
                SpeakerDisplayState::Active
            );
            assert_eq!(
                state.get_speaker_display_state("speaker2"),
                SpeakerDisplayState::Normal
            );
        });

        // Set locked speaker
        store.dispatch(AppAction::ToggleSpeakerLock("speaker2".to_string()));
        store.with_state(|state| {
            assert_eq!(
                state.get_speaker_display_state("speaker1"),
                SpeakerDisplayState::Active
            );
            assert_eq!(
                state.get_speaker_display_state("speaker2"),
                SpeakerDisplayState::Locked
            );
        });

        // Set same speaker as both active and locked
        store.dispatch(AppAction::SetActiveSpeaker("speaker2".to_string()));
        store.with_state(|state| {
            assert_eq!(
                state.get_speaker_display_state("speaker2"),
                SpeakerDisplayState::ActiveAndLocked
            );
        });
    }

    #[test]
    fn test_store_send_command_without_system() {
        let store = Store::new();

        // Try to send command without system
        store.dispatch(AppAction::SendSpeakerCommand {
            uuid: "speaker1".to_string(),
            command: SpeakerCommand::Play,
        });

        store.with_state(|state| {
            assert_eq!(state.status_message, "System not ready");
        });
    }

    #[test]
    fn test_app_state_speaker_checks() {
        let mut state = AppState::default();

        // Test with no speakers set
        assert!(!state.is_speaker_active("speaker1"));
        assert!(!state.is_speaker_locked("speaker1"));
        assert_eq!(
            state.get_speaker_display_state("speaker1"),
            SpeakerDisplayState::Normal
        );

        // Set active speaker
        state.active_speaker_uuid = Some("speaker1".to_string());
        assert!(state.is_speaker_active("speaker1"));
        assert!(!state.is_speaker_active("speaker2"));

        // Set locked speaker
        state.locked_speaker_uuid = Some("speaker2".to_string());
        assert!(state.is_speaker_locked("speaker2"));
        assert!(!state.is_speaker_locked("speaker1"));

        // Test display states
        assert_eq!(
            state.get_speaker_display_state("speaker1"),
            SpeakerDisplayState::Active
        );
        assert_eq!(
            state.get_speaker_display_state("speaker2"),
            SpeakerDisplayState::Locked
        );
        assert_eq!(
            state.get_speaker_display_state("speaker3"),
            SpeakerDisplayState::Normal
        );
    }
}
