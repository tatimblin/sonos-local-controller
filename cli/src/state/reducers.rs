use sonos::{Speaker, System};
use std::sync::Arc;

use crate::{topology::topology_list::TopologyList, views::ViewType};

use super::store::AppState;

pub enum AppAction {
    AddSpeaker(Speaker),
    AdjustVolume(i8),
    SetStatusMessage(String),
    SetTopology(TopologyList),
    SetSystem(Arc<System>),
    SetActiveSpeaker(String),
    ToggleSpeakerLock(String),
    ClearActiveSelection,
    ClearSelection,
}

impl std::fmt::Debug for AppAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppAction::AddSpeaker(speaker) => f.debug_tuple("AddSpeaker").field(speaker).finish(),
            AppAction::AdjustVolume(adjustment) => {
                f.debug_tuple("AdjustVolume").field(adjustment).finish()
            }
            AppAction::SetStatusMessage(message) => {
                f.debug_tuple("SetStatusMessage").field(message).finish()
            }
            AppAction::SetTopology(topology) => {
                f.debug_tuple("SetTopology").field(topology).finish()
            }
            AppAction::SetSystem(_) => f.debug_tuple("SetSystem").field(&"Arc<System>").finish(),
            AppAction::SetActiveSpeaker(uuid) => {
                f.debug_tuple("SetActiveSpeaker").field(uuid).finish()
            }
            AppAction::ToggleSpeakerLock(uuid) => {
                f.debug_tuple("ToggleSpeakerLock").field(uuid).finish()
            }
            AppAction::ClearActiveSelection => f.debug_tuple("ClearActiveSelection").finish(),
            AppAction::ClearSelection => f.debug_tuple("ClearSelection").finish(),
        }
    }
}

pub fn app_reducer(state: &mut AppState, action: AppAction) {
    match action {
        AppAction::AddSpeaker(_speaker) => {}
        AppAction::AdjustVolume(_adjustment) => {}
        AppAction::SetStatusMessage(message) => {
            state.status_message = message;
        }
        AppAction::SetTopology(topology) => {
            log::debug!("SetTopology action received, switching to Control view");
            state.topology = Some(topology);
            state.view = ViewType::Control;
        }
        AppAction::SetSystem(system) => {
            log::debug!("SetSystem action received");
            state.system = Some(system);
        }
        AppAction::SetActiveSpeaker(uuid) => {
            log::debug!("SetActiveSpeaker action received: {}", uuid);
            state.active_speaker_uuid = Some(uuid);
        }
        AppAction::ToggleSpeakerLock(uuid) => {
            log::debug!("ToggleSpeakerLock action received: {}", uuid);

            // Toggle the lock state - if currently locked to this speaker, unlock it
            // If locked to a different speaker or not locked, lock to this speaker
            let is_currently_locked =
                state.locked_speaker_uuid.as_ref().map(|s| s.as_str()) == Some(&uuid);

            if is_currently_locked {
                state.locked_speaker_uuid = None;
            } else {
                // Ensure single selection constraint - automatically unlock any previously locked speaker
                state.locked_speaker_uuid = Some(uuid);
            }
        }
        AppAction::ClearActiveSelection => {
            log::debug!("ClearActiveSelection action received");
            state.active_speaker_uuid = None;
            state.locked_speaker_uuid = None;
        }
        AppAction::ClearSelection => {
            log::debug!("ClearSelection action received");
            state.active_speaker_uuid = None;
            state.locked_speaker_uuid = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::{topology_item::TopologyItem, topology_list::TopologyList};

    fn create_test_topology_with_speakers() -> TopologyList {
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
    fn test_set_active_speaker_with_valid_uuid() {
        let mut state = AppState::default();
        state.topology = Some(create_test_topology_with_speakers());

        app_reducer(
            &mut state,
            AppAction::SetActiveSpeaker("speaker1".to_string()),
        );

        assert_eq!(state.active_speaker_uuid, Some("speaker1".to_string()));
    }

    #[test]
    fn test_set_active_speaker_always_sets_uuid() {
        let mut state = AppState::default();
        state.topology = Some(create_test_topology_with_speakers());

        app_reducer(
            &mut state,
            AppAction::SetActiveSpeaker("any_speaker".to_string()),
        );

        assert_eq!(state.active_speaker_uuid, Some("any_speaker".to_string()));
    }

    #[test]
    fn test_toggle_speaker_lock_with_valid_uuid() {
        let mut state = AppState::default();
        state.topology = Some(create_test_topology_with_speakers());

        // Lock speaker1
        app_reducer(
            &mut state,
            AppAction::ToggleSpeakerLock("speaker1".to_string()),
        );
        assert_eq!(state.locked_speaker_uuid, Some("speaker1".to_string()));

        // Unlock speaker1
        app_reducer(
            &mut state,
            AppAction::ToggleSpeakerLock("speaker1".to_string()),
        );
        assert_eq!(state.locked_speaker_uuid, None);
    }

    #[test]
    fn test_toggle_speaker_lock_always_works() {
        let mut state = AppState::default();
        state.topology = Some(create_test_topology_with_speakers());

        app_reducer(
            &mut state,
            AppAction::ToggleSpeakerLock("any_speaker".to_string()),
        );

        assert_eq!(state.locked_speaker_uuid, Some("any_speaker".to_string()));
    }

    #[test]
    fn test_single_selection_constraint() {
        let mut state = AppState::default();
        state.topology = Some(create_test_topology_with_speakers());

        // Lock first speaker
        app_reducer(
            &mut state,
            AppAction::ToggleSpeakerLock("speaker1".to_string()),
        );
        assert_eq!(state.locked_speaker_uuid, Some("speaker1".to_string()));

        // Lock second speaker - should replace the first
        app_reducer(
            &mut state,
            AppAction::ToggleSpeakerLock("speaker2".to_string()),
        );
        assert_eq!(state.locked_speaker_uuid, Some("speaker2".to_string()));
    }

    #[test]
    fn test_clear_active_selection() {
        let mut state = AppState::default();
        state.topology = Some(create_test_topology_with_speakers());
        state.active_speaker_uuid = Some("speaker1".to_string());
        state.locked_speaker_uuid = Some("speaker2".to_string());

        app_reducer(&mut state, AppAction::ClearActiveSelection);

        assert_eq!(state.active_speaker_uuid, None);
        assert_eq!(state.locked_speaker_uuid, None);
    }

    #[test]
    fn test_set_topology_preserves_selections() {
        let mut state = AppState::default();
        state.active_speaker_uuid = Some("speaker1".to_string());
        state.locked_speaker_uuid = Some("speaker2".to_string());

        let new_topology = create_test_topology_with_speakers();
        app_reducer(&mut state, AppAction::SetTopology(new_topology));

        // Selections are preserved since we removed validation
        assert_eq!(state.active_speaker_uuid, Some("speaker1".to_string()));
        assert_eq!(state.locked_speaker_uuid, Some("speaker2".to_string()));
    }
}
