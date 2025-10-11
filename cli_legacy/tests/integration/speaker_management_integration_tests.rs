//! Integration tests for end-to-end speaker management system command flow
//!
//! These tests verify the complete flow from UI selection to command execution,
//! including error handling when speakers become unavailable and behavior when
//! System reference is not available.

use std::sync::Arc;
use cli::state::store::{Store, AppState};
use cli::state::reducers::AppAction;
use cli::views::control::ControlView;
use cli::types::{Topology, Group, SpeakerInfo, System, SpeakerManagerError};
use crossterm::event::{KeyCode, KeyEvent};

/// Helper function to create a test topology with multiple groups and speakers
fn create_comprehensive_test_topology() -> Topology {
    Topology {
        groups: vec![
            // Single speaker group
            Group {
                name: "Bedroom".to_string(),
                speakers: vec![
                    SpeakerInfo {
                        name: "Bedroom".to_string(),
                        uuid: "RINCON_BEDROOM_001".to_string(),
                        ip: "192.168.1.100".to_string(),
                        is_coordinator: true,
                    },
                ],
            },
            // Multi-speaker group
            Group {
                name: "Living Room".to_string(),
                speakers: vec![
                    SpeakerInfo {
                        name: "Living Room".to_string(),
                        uuid: "RINCON_LIVING_ROOM_001".to_string(),
                        ip: "192.168.1.101".to_string(),
                        is_coordinator: true,
                    },
                    SpeakerInfo {
                        name: "Kitchen".to_string(),
                        uuid: "RINCON_KITCHEN_001".to_string(),
                        ip: "192.168.1.102".to_string(),
                        is_coordinator: false,
                    },
                ],
            },
            // Another single speaker group
            Group {
                name: "Office".to_string(),
                speakers: vec![
                    SpeakerInfo {
                        name: "Office".to_string(),
                        uuid: "RINCON_OFFICE_001".to_string(),
                        ip: "192.168.1.103".to_string(),
                        is_coordinator: true,
                    },
                ],
            },
        ],
    }
}

/// Helper function to create a mock system for testing
fn create_mock_system() -> Arc<System> {
    Arc::new(System::new().expect("Failed to create mock system"))
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_complete_command_flow_with_speaker_selection() {
        // Create store and set up initial state
        let store = Store::new();
        let topology = create_comprehensive_test_topology();
        let system = create_mock_system();
        
        // Set up the state with topology and system
        store.dispatch(AppAction::SetTopology(topology));
        store.dispatch(AppAction::SetSystem(system));
        
        // Create control view
        let mut control_view = ControlView::new(&store);
        
        // Navigate to select a speaker (move down to first speaker in first group)
        let down_key = KeyEvent::from(KeyCode::Down);
        let result = control_view.handle_input(down_key, &store);
        assert!(result.is_ok(), "Navigation should succeed");
        
        // Verify selection state was updated
        store.with_state(|state| {
            assert!(state.selected_speaker_uuid.is_some(), "Speaker should be selected");
            // Should be the Bedroom speaker (first group's coordinator)
            assert_eq!(state.selected_speaker_uuid.as_ref().unwrap(), "RINCON_BEDROOM_001");
        });
        
        // Attempt to execute pause command
        let space_key = KeyEvent::from(KeyCode::Char(' '));
        let result = control_view.handle_input(space_key, &store);
        assert!(result.is_ok(), "Pause command input should be handled");
        
        // Verify status message was updated (command will fail due to mock system, but should be handled)
        store.with_state(|state| {
            assert!(state.status_message.contains("Pause failed") || state.status_message == "Paused");
        });
    }

    #[test]
    fn test_complete_command_flow_with_group_selection() {
        // Create store and set up initial state
        let store = Store::new();
        let topology = create_comprehensive_test_topology();
        let system = create_mock_system();
        
        // Set up the state with topology and system
        store.dispatch(AppAction::SetTopology(topology));
        store.dispatch(AppAction::SetSystem(system));
        
        // Create control view
        let mut control_view = ControlView::new(&store);
        
        // Navigate to select a group (stay on first item which should be a group)
        // First item should be the Bedroom group
        
        // Verify we're on a group by checking the selection
        store.with_state(|state| {
            // Initial selection should be on first item
            assert!(state.topology.is_some());
        });
        
        // Attempt to execute play command on group
        let enter_key = KeyEvent::from(KeyCode::Enter);
        let result = control_view.handle_input(enter_key, &store);
        assert!(result.is_ok(), "Play command input should be handled");
        
        // Verify status message was updated
        store.with_state(|state| {
            assert!(state.status_message.contains("Play failed") || state.status_message == "Playing");
        });
    }

    #[test]
    fn test_command_flow_with_no_system_reference() {
        // Create store with topology but no system
        let store = Store::new();
        let topology = create_comprehensive_test_topology();
        
        // Set topology but deliberately omit system
        store.dispatch(AppAction::SetTopology(topology));
        
        // Create control view
        let mut control_view = ControlView::new(&store);
        
        // Navigate to select a speaker
        let down_key = KeyEvent::from(KeyCode::Down);
        let result = control_view.handle_input(down_key, &store);
        assert!(result.is_ok(), "Navigation should succeed even without system");
        
        // Attempt to execute command without system reference
        let space_key = KeyEvent::from(KeyCode::Char(' '));
        let result = control_view.handle_input(space_key, &store);
        assert!(result.is_ok(), "Command input should be handled gracefully");
        
        // Verify error was handled gracefully
        store.with_state(|state| {
            // Should have an error message about no selection or system unavailable
            assert!(state.status_message.contains("failed") || state.status_message.contains("No selection"));
        });
    }

    #[test]
    fn test_command_flow_with_no_topology() {
        // Create store with system but no topology
        let store = Store::new();
        let system = create_mock_system();
        
        // Set system but deliberately omit topology
        store.dispatch(AppAction::SetSystem(system));
        
        // Create control view (should handle empty topology gracefully)
        let mut control_view = ControlView::new(&store);
        
        // Attempt navigation (should not crash)
        let down_key = KeyEvent::from(KeyCode::Down);
        let result = control_view.handle_input(down_key, &store);
        assert!(result.is_ok(), "Navigation should handle empty topology gracefully");
        
        // Attempt command execution
        let space_key = KeyEvent::from(KeyCode::Char(' '));
        let result = control_view.handle_input(space_key, &store);
        assert!(result.is_ok(), "Command should handle missing topology gracefully");
        
        // Verify error was handled
        store.with_state(|state| {
            assert!(state.status_message.contains("failed") || state.status_message.contains("No selection"));
        });
    }

    #[test]
    fn test_selection_persistence_across_topology_updates() {
        // Create store and set initial topology
        let store = Store::new();
        let initial_topology = create_comprehensive_test_topology();
        let system = create_mock_system();
        
        store.dispatch(AppAction::SetTopology(initial_topology));
        store.dispatch(AppAction::SetSystem(system));
        
        // Set a specific selection
        store.dispatch(AppAction::SetSelectedSpeakerUuid("RINCON_LIVING_ROOM_001".to_string()));
        store.dispatch(AppAction::SetSelectedGroupUuid("RINCON_LIVING_ROOM_001".to_string()));
        
        // Verify selection is set
        store.with_state(|state| {
            assert_eq!(state.selected_speaker_uuid.as_ref().unwrap(), "RINCON_LIVING_ROOM_001");
            assert_eq!(state.selected_group_uuid.as_ref().unwrap(), "RINCON_LIVING_ROOM_001");
        });
        
        // Update topology with same speakers (selection should persist)
        let updated_topology = Topology {
            groups: vec![
                Group {
                    name: "Living Room".to_string(),
                    speakers: vec![
                        SpeakerInfo {
                            name: "Living Room".to_string(),
                            uuid: "RINCON_LIVING_ROOM_001".to_string(),
                            ip: "192.168.1.101".to_string(),
                            is_coordinator: true,
                        },
                        SpeakerInfo {
                            name: "Kitchen".to_string(),
                            uuid: "RINCON_KITCHEN_001".to_string(),
                            ip: "192.168.1.102".to_string(),
                            is_coordinator: false,
                        },
                    ],
                },
                // Add new group
                Group {
                    name: "Bathroom".to_string(),
                    speakers: vec![
                        SpeakerInfo {
                            name: "Bathroom".to_string(),
                            uuid: "RINCON_BATHROOM_001".to_string(),
                            ip: "192.168.1.104".to_string(),
                            is_coordinator: true,
                        },
                    ],
                },
            ],
        };
        
        store.dispatch(AppAction::SetTopology(updated_topology));
        
        // Verify selection persisted since speaker still exists
        store.with_state(|state| {
            assert_eq!(state.selected_speaker_uuid.as_ref().unwrap(), "RINCON_LIVING_ROOM_001");
            assert_eq!(state.selected_group_uuid.as_ref().unwrap(), "RINCON_LIVING_ROOM_001");
        });
    }

    #[test]
    fn test_selection_clearing_when_speaker_becomes_unavailable() {
        // Create store and set initial topology
        let store = Store::new();
        let initial_topology = create_comprehensive_test_topology();
        let system = create_mock_system();
        
        store.dispatch(AppAction::SetTopology(initial_topology));
        store.dispatch(AppAction::SetSystem(system));
        
        // Set selection for a speaker that will be removed
        store.dispatch(AppAction::SetSelectedSpeakerUuid("RINCON_OFFICE_001".to_string()));
        store.dispatch(AppAction::SetSelectedGroupUuid("RINCON_OFFICE_001".to_string()));
        
        // Verify selection is set
        store.with_state(|state| {
            assert_eq!(state.selected_speaker_uuid.as_ref().unwrap(), "RINCON_OFFICE_001");
            assert_eq!(state.selected_group_uuid.as_ref().unwrap(), "RINCON_OFFICE_001");
        });
        
        // Update topology without the selected speaker
        let updated_topology = Topology {
            groups: vec![
                Group {
                    name: "Bedroom".to_string(),
                    speakers: vec![
                        SpeakerInfo {
                            name: "Bedroom".to_string(),
                            uuid: "RINCON_BEDROOM_001".to_string(),
                            ip: "192.168.1.100".to_string(),
                            is_coordinator: true,
                        },
                    ],
                },
                Group {
                    name: "Living Room".to_string(),
                    speakers: vec![
                        SpeakerInfo {
                            name: "Living Room".to_string(),
                            uuid: "RINCON_LIVING_ROOM_001".to_string(),
                            ip: "192.168.1.101".to_string(),
                            is_coordinator: true,
                        },
                    ],
                },
                // Office group removed
            ],
        };
        
        store.dispatch(AppAction::SetTopology(updated_topology));
        
        // Verify selection was cleared since speaker no longer exists
        store.with_state(|state| {
            assert!(state.selected_speaker_uuid.is_none());
            assert!(state.selected_group_uuid.is_none());
        });
    }

    #[test]
    fn test_multiple_navigation_and_command_cycles() {
        // Create store and set up state
        let store = Store::new();
        let topology = create_comprehensive_test_topology();
        let system = create_mock_system();
        
        store.dispatch(AppAction::SetTopology(topology));
        store.dispatch(AppAction::SetSystem(system));
        
        let mut control_view = ControlView::new(&store);
        
        // Perform multiple navigation cycles
        for cycle in 0..3 {
            // Navigate down several times
            for _ in 0..5 {
                let down_key = KeyEvent::from(KeyCode::Down);
                let result = control_view.handle_input(down_key, &store);
                assert!(result.is_ok(), "Navigation cycle {} should succeed", cycle);
            }
            
            // Try pause command
            let space_key = KeyEvent::from(KeyCode::Char(' '));
            let result = control_view.handle_input(space_key, &store);
            assert!(result.is_ok(), "Pause command cycle {} should be handled", cycle);
            
            // Navigate up several times
            for _ in 0..3 {
                let up_key = KeyEvent::from(KeyCode::Up);
                let result = control_view.handle_input(up_key, &store);
                assert!(result.is_ok(), "Up navigation cycle {} should succeed", cycle);
            }
            
            // Try play command
            let enter_key = KeyEvent::from(KeyCode::Enter);
            let result = control_view.handle_input(enter_key, &store);
            assert!(result.is_ok(), "Play command cycle {} should be handled", cycle);
        }
        
        // Verify system remains stable
        store.with_state(|state| {
            assert!(state.topology.is_some());
            assert!(state.system.is_some());
            // Status message should reflect the last command attempt
            assert!(!state.status_message.is_empty());
        });
    }

    #[test]
    fn test_concurrent_state_updates_during_command_execution() {
        // Create store and set up state
        let store = Store::new();
        let topology = create_comprehensive_test_topology();
        let system = create_mock_system();
        
        store.dispatch(AppAction::SetTopology(topology.clone()));
        store.dispatch(AppAction::SetSystem(system));
        
        let mut control_view = ControlView::new(&store);
        
        // Set initial selection
        let down_key = KeyEvent::from(KeyCode::Down);
        let result = control_view.handle_input(down_key, &store);
        assert!(result.is_ok());
        
        // Simulate concurrent state updates while command is being processed
        // Update status message
        store.dispatch(AppAction::SetStatusMessage("Concurrent update".to_string()));
        
        // Update selection
        store.dispatch(AppAction::SetSelectedSpeakerUuid("RINCON_LIVING_ROOM_001".to_string()));
        
        // Execute command
        let space_key = KeyEvent::from(KeyCode::Char(' '));
        let result = control_view.handle_input(space_key, &store);
        assert!(result.is_ok(), "Command should handle concurrent updates");
        
        // Verify state consistency
        store.with_state(|state| {
            assert!(state.topology.is_some());
            assert!(state.system.is_some());
            assert!(state.selected_speaker_uuid.is_some());
        });
    }

    #[test]
    fn test_error_recovery_after_failed_commands() {
        // Create store with intentionally problematic setup
        let store = Store::new();
        let topology = create_comprehensive_test_topology();
        
        // Set topology but no system (commands will fail)
        store.dispatch(AppAction::SetTopology(topology));
        
        let mut control_view = ControlView::new(&store);
        
        // Navigate and attempt commands that will fail
        let down_key = KeyEvent::from(KeyCode::Down);
        let result = control_view.handle_input(down_key, &store);
        assert!(result.is_ok());
        
        // Try multiple failing commands
        for _ in 0..5 {
            let space_key = KeyEvent::from(KeyCode::Char(' '));
            let result = control_view.handle_input(space_key, &store);
            assert!(result.is_ok(), "Failed commands should be handled gracefully");
            
            let enter_key = KeyEvent::from(KeyCode::Enter);
            let result = control_view.handle_input(enter_key, &store);
            assert!(result.is_ok(), "Failed commands should be handled gracefully");
        }
        
        // Now add system and verify recovery
        let system = create_mock_system();
        store.dispatch(AppAction::SetSystem(system));
        
        // Commands should now be attempted (though may still fail due to mock system)
        let space_key = KeyEvent::from(KeyCode::Char(' '));
        let result = control_view.handle_input(space_key, &store);
        assert!(result.is_ok(), "Commands should work after system is available");
        
        // Verify system recovered
        store.with_state(|state| {
            assert!(state.system.is_some());
            assert!(state.topology.is_some());
        });
    }

    #[test]
    fn test_ui_responsiveness_during_command_execution() {
        // Create store and set up state
        let store = Store::new();
        let topology = create_comprehensive_test_topology();
        let system = create_mock_system();
        
        store.dispatch(AppAction::SetTopology(topology));
        store.dispatch(AppAction::SetSystem(system));
        
        let mut control_view = ControlView::new(&store);
        
        // Interleave navigation and command execution to test responsiveness
        let commands = vec![
            KeyCode::Down,
            KeyCode::Char(' '), // pause
            KeyCode::Down,
            KeyCode::Enter,     // play
            KeyCode::Up,
            KeyCode::Left,      // volume down
            KeyCode::Down,
            KeyCode::Right,     // volume up
            KeyCode::Char(' '), // pause
            KeyCode::Up,
            KeyCode::Enter,     // play
        ];
        
        for (i, key_code) in commands.iter().enumerate() {
            let key_event = KeyEvent::from(*key_code);
            let result = control_view.handle_input(key_event, &store);
            assert!(result.is_ok(), "Command {} should be handled responsively", i);
            
            // Verify state remains consistent after each command
            store.with_state(|state| {
                assert!(state.topology.is_some());
                assert!(state.system.is_some());
            });
        }
    }

    #[test]
    fn test_selection_state_consistency_across_operations() {
        // Create store and set up state
        let store = Store::new();
        let topology = create_comprehensive_test_topology();
        let system = create_mock_system();
        
        store.dispatch(AppAction::SetTopology(topology));
        store.dispatch(AppAction::SetSystem(system));
        
        let mut control_view = ControlView::new(&store);
        
        // Track selection changes through navigation
        let mut previous_selection: Option<String> = None;
        
        for i in 0..10 {
            // Navigate
            let down_key = KeyEvent::from(KeyCode::Down);
            let result = control_view.handle_input(down_key, &store);
            assert!(result.is_ok(), "Navigation step {} should succeed", i);
            
            // Check selection consistency
            store.with_state(|state| {
                let current_selection = state.selected_speaker_uuid.clone();
                
                // Selection should change or remain consistent
                if let (Some(prev), Some(curr)) = (&previous_selection, &current_selection) {
                    // If selection changed, it should be to a valid speaker
                    if prev != curr {
                        assert!(state.topology.as_ref().unwrap().get_speaker_by_uuid(curr).is_some(),
                               "New selection should be valid: {}", curr);
                    }
                }
                
                previous_selection = current_selection;
            });
            
            // Execute command to test selection usage
            if i % 3 == 0 {
                let space_key = KeyEvent::from(KeyCode::Char(' '));
                let result = control_view.handle_input(space_key, &store);
                assert!(result.is_ok(), "Command at step {} should be handled", i);
            }
        }
    }
}