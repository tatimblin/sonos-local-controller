use sonos::{SpeakerCommand, System};
use std::sync::Arc;

use crate::{topology::topology_list::TopologyList, views::ViewType};

use super::store::AppState;

pub enum AppAction {
    SetStatusMessage(String),
    SetTopology(TopologyList),
    SetSystem(Arc<System>),
    SendSpeakerCommand { uuid: String, command: SpeakerCommand },
    SetActiveSpeaker(String),
    ToggleSpeakerLock(String),
}

impl std::fmt::Debug for AppAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppAction::SetStatusMessage(message) => {
                f.debug_tuple("SetStatusMessage").field(message).finish()
            }
            AppAction::SetTopology(topology) => {
                f.debug_tuple("SetTopology").field(topology).finish()
            }
            AppAction::SetSystem(_) => {
                f.debug_tuple("SetSystem")
                    .field(&"Arc<System>")
                    .finish()
            },
            AppAction::SendSpeakerCommand { uuid, command } => {
                f.debug_struct("SendSpeakerCommand")
                    .field("uuid", uuid)
                    .field("command", command)
                    .finish()
            }
            AppAction::SetActiveSpeaker(uuid) => {
                f.debug_tuple("SetActiveSpeaker").field(uuid).finish()
            }
            AppAction::ToggleSpeakerLock(uuid) => {
                f.debug_tuple("ToggleSpeakerLock").field(uuid).finish()
            }
        }
    }
}

pub fn app_reducer(state: &mut AppState, action: AppAction) {
    match action {
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
        AppAction::SendSpeakerCommand { uuid, command } => {
            log::debug!("SendSpeakerCommand action received: {} -> {:?}", uuid, command);
            
            // Execute the command if we have a system reference
            if let Some(system) = &state.system {
                match system.send_command_to_speaker(&uuid, command) {
                    Ok(()) => {
                        log::debug!("Successfully sent command to speaker {}", uuid);
                    }
                    Err(e) => {
                        log::error!("Failed to send command to speaker {}: {:?}", uuid, e);
                        state.status_message = format!("Command failed: {}", e);
                    }
                }
            } else {
                log::error!("Cannot send command: System not initialized");
                state.status_message = "System not ready".to_string();
            }
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

    #[test]
    fn test_set_system_action() {
        use sonos::System;
        use std::sync::Arc;

        let mut state = AppState::default();
        assert!(state.system.is_none());

        // Create a mock system (this will fail in practice but tests the action handling)
        let system = Arc::new(System::new().unwrap_or_else(|_| panic!("Failed to create system for test")));
        app_reducer(&mut state, AppAction::SetSystem(system.clone()));

        assert!(state.system.is_some());
    }

    #[test]
    fn test_send_speaker_command_without_system() {
        let mut state = AppState::default();
        let initial_status = state.status_message.clone();

        app_reducer(&mut state, AppAction::SendSpeakerCommand {
            uuid: "test-uuid".to_string(),
            command: SpeakerCommand::Play,
        });

        // Should update status message when system is not available
        assert_ne!(state.status_message, initial_status);
        assert_eq!(state.status_message, "System not ready");
    }

    #[test]
    fn test_set_system_action_updates_state() {
        let mut state = AppState::default();
        assert!(state.system.is_none());

        // Create a system for testing
        match System::new() {
            Ok(system) => {
                let system_arc = Arc::new(system);
                app_reducer(&mut state, AppAction::SetSystem(system_arc.clone()));
                
                assert!(state.system.is_some());
                // Verify it's the same Arc by comparing pointer addresses
                assert!(Arc::ptr_eq(state.system.as_ref().unwrap(), &system_arc));
            }
            Err(_) => {
                // Skip test if system creation fails (no network/hardware available)
                println!("Skipping test_set_system_action_updates_state - System::new() failed");
            }
        }
    }

    #[test]
    fn test_send_speaker_command_with_different_commands() {
        let mut state = AppState::default();
        
        // Test each command type updates status appropriately when no system is available
        let commands = vec![
            SpeakerCommand::Play,
            SpeakerCommand::Pause,
            SpeakerCommand::SetVolume(50),
            SpeakerCommand::AdjustVolume(5),
        ];

        for command in commands {
            state.status_message = "initial".to_string();
            
            app_reducer(&mut state, AppAction::SendSpeakerCommand {
                uuid: "test-speaker".to_string(),
                command,
            });

            assert_eq!(state.status_message, "System not ready");
        }
    }

    #[test]
    fn test_send_speaker_command_preserves_other_state() {
        let mut state = AppState::default();
        state.active_speaker_uuid = Some("active-speaker".to_string());
        state.locked_speaker_uuid = Some("locked-speaker".to_string());
        state.topology = Some(create_test_topology_with_speakers());
        
        app_reducer(&mut state, AppAction::SendSpeakerCommand {
            uuid: "test-uuid".to_string(),
            command: SpeakerCommand::Play,
        });

        // Other state should be preserved
        assert_eq!(state.active_speaker_uuid, Some("active-speaker".to_string()));
        assert_eq!(state.locked_speaker_uuid, Some("locked-speaker".to_string()));
        assert!(state.topology.is_some());
    }

    #[test]
    fn test_set_status_message_action() {
        let mut state = AppState::default();
        let test_message = "Test status message";
        
        app_reducer(&mut state, AppAction::SetStatusMessage(test_message.to_string()));
        
        assert_eq!(state.status_message, test_message);
    }

    #[test]
    fn test_set_topology_switches_to_control_view() {
        let mut state = AppState::default();
        assert_eq!(state.view, ViewType::Startup);
        assert!(state.topology.is_none());
        
        let topology = create_test_topology_with_speakers();
        app_reducer(&mut state, AppAction::SetTopology(topology));
        
        assert_eq!(state.view, ViewType::Control);
        assert!(state.topology.is_some());
    }
}