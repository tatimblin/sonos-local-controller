//! Integration tests for error scenarios and edge cases
//!
//! These tests verify that the system handles various error conditions gracefully,
//! including network failures, missing speakers, invalid state, and recovery scenarios.

use std::sync::Arc;
use cli::state::store::{Store, AppState};
use cli::state::reducers::AppAction;
use cli::types::{Topology, Group, SpeakerInfo, System, SpeakerManagerError};
use cli::views::control::ControlView;
use crossterm::event::{KeyCode, KeyEvent};

/// Helper to create an empty topology for error testing
fn create_empty_topology() -> Topology {
    Topology { groups: vec![] }
}

/// Helper to create a topology that will become invalid
fn create_unstable_topology() -> Topology {
    Topology {
        groups: vec![
            Group {
                name: "Unstable Speaker".to_string(),
                speakers: vec![
                    SpeakerInfo {
                        name: "Unstable Speaker".to_string(),
                        uuid: "RINCON_UNSTABLE_001".to_string(),
                        ip: "192.168.1.100".to_string(),
                        is_coordinator: true,
                    },
                ],
            },
        ],
    }
}

/// Helper to create a topology with inconsistent data
fn create_inconsistent_topology() -> Topology {
    Topology {
        groups: vec![
            Group {
                name: "Group With No Coordinator".to_string(),
                speakers: vec![
                    SpeakerInfo {
                        name: "Speaker 1".to_string(),
                        uuid: "RINCON_SPEAKER_1".to_string(),
                        ip: "192.168.1.100".to_string(),
                        is_coordinator: false, // No coordinator in group!
                    },
                    SpeakerInfo {
                        name: "Speaker 2".to_string(),
                        uuid: "RINCON_SPEAKER_2".to_string(),
                        ip: "192.168.1.101".to_string(),
                        is_coordinator: false, // No coordinator in group!
                    },
                ],
            },
        ],
    }
}

#[cfg(test)]
mod error_scenario_tests {
    use super::*;

    #[test]
    fn test_empty_topology_handling() {
        let store = Store::new();
        let empty_topology = create_empty_topology();
        let system = Arc::new(System::new().expect("Failed to create system"));
        
        // Set up state with empty topology
        store.dispatch(AppAction::SetTopology(empty_topology));
        store.dispatch(AppAction::SetSystem(system));
        
        // Create control view with empty topology
        let mut control_view = ControlView::new(&store);
        
        // Test navigation with empty topology
        let down_key = KeyEvent::from(KeyCode::Down);
        let result = control_view.handle_input(down_key, &store);
        assert!(result.is_ok(), "Navigation should handle empty topology gracefully");
        
        // Test command execution with empty topology
        let space_key = KeyEvent::from(KeyCode::Char(' '));
        let result = control_view.handle_input(space_key, &store);
        assert!(result.is_ok(), "Commands should handle empty topology gracefully");
        
        // Verify error message was set
        store.with_state(|state| {
            assert!(state.status_message.contains("failed") || state.status_message.contains("No selection"));
        });
    }

    #[test]
    fn test_missing_system_reference_handling() {
        let store = Store::new();
        let topology = create_unstable_topology();
        
        // Set topology but deliberately omit system
        store.dispatch(AppAction::SetTopology(topology));
        
        // Set selection
        store.dispatch(AppAction::SetSelectedSpeakerUuid("RINCON_UNSTABLE_001".to_string()));
        
        // Create control view
        let mut control_view = ControlView::new(&store);
        
        // Test command execution without system
        let space_key = KeyEvent::from(KeyCode::Char(' '));
        let result = control_view.handle_input(space_key, &store);
        assert!(result.is_ok(), "Command should handle missing system gracefully");
        
        // Verify error was handled
        store.with_state(|state| {
            assert!(state.status_message.contains("failed") || state.status_message.contains("No selection"));
        });
    }

    #[test]
    fn test_invalid_selection_state_handling() {
        let store = Store::new();
        let topology = create_unstable_topology();
        let system = Arc::new(System::new().expect("Failed to create system"));
        
        store.dispatch(AppAction::SetTopology(topology));
        store.dispatch(AppAction::SetSystem(system));
        
        // Set invalid selections
        store.dispatch(AppAction::SetSelectedSpeakerUuid("RINCON_NONEXISTENT".to_string()));
        store.dispatch(AppAction::SetSelectedGroupUuid("RINCON_ALSO_NONEXISTENT".to_string()));
        
        let mut control_view = ControlView::new(&store);
        
        // Test command execution with invalid selections
        let space_key = KeyEvent::from(KeyCode::Char(' '));
        let result = control_view.handle_input(space_key, &store);
        assert!(result.is_ok(), "Command should handle invalid selection gracefully");
        
        let enter_key = KeyEvent::from(KeyCode::Enter);
        let result = control_view.handle_input(enter_key, &store);
        assert!(result.is_ok(), "Command should handle invalid selection gracefully");
        
        // Verify error messages were set
        store.with_state(|state| {
            assert!(state.status_message.contains("failed") || state.status_message.contains("No selection"));
        });
    }

    #[test]
    fn test_topology_update_during_command_execution() {
        let store = Store::new();
        let initial_topology = create_unstable_topology();
        let system = Arc::new(System::new().expect("Failed to create system"));
        
        store.dispatch(AppAction::SetTopology(initial_topology));
        store.dispatch(AppAction::SetSystem(system));
        store.dispatch(AppAction::SetSelectedSpeakerUuid("RINCON_UNSTABLE_001".to_string()));
        
        let mut control_view = ControlView::new(&store);
        
        // Start command execution
        let space_key = KeyEvent::from(KeyCode::Char(' '));
        let result = control_view.handle_input(space_key, &store);
        assert!(result.is_ok());
        
        // Update topology to remove selected speaker
        let empty_topology = create_empty_topology();
        store.dispatch(AppAction::SetTopology(empty_topology));
        
        // Verify selection was cleared
        store.with_state(|state| {
            assert!(state.selected_speaker_uuid.is_none());
            assert!(state.selected_group_uuid.is_none());
        });
        
        // Try command execution after topology change
        let enter_key = KeyEvent::from(KeyCode::Enter);
        let result = control_view.handle_input(enter_key, &store);
        assert!(result.is_ok(), "Command should handle topology changes gracefully");
    }

    #[test]
    fn test_inconsistent_topology_data_handling() {
        let store = Store::new();
        let inconsistent_topology = create_inconsistent_topology();
        let system = Arc::new(System::new().expect("Failed to create system"));
        
        store.dispatch(AppAction::SetTopology(inconsistent_topology));
        store.dispatch(AppAction::SetSystem(system));
        
        // Try to select a speaker from the inconsistent group
        store.dispatch(AppAction::SetSelectedSpeakerUuid("RINCON_SPEAKER_1".to_string()));
        
        let mut control_view = ControlView::new(&store);
        
        // Test command execution with inconsistent data
        let space_key = KeyEvent::from(KeyCode::Char(' '));
        let result = control_view.handle_input(space_key, &store);
        assert!(result.is_ok(), "Should handle inconsistent topology data");
        
        // Try to find group by coordinator (should fail gracefully)
        store.with_state(|state| {
            if let Some(topology) = &state.topology {
                let group = topology.get_group_by_coordinator_uuid("RINCON_SPEAKER_1");
                assert!(group.is_none(), "Should not find group with non-coordinator");
                
                let speaker = topology.get_speaker_by_uuid("RINCON_SPEAKER_1");
                assert!(speaker.is_some(), "Should find speaker even in inconsistent group");
                assert!(!speaker.unwrap().is_coordinator);
            }
        });
    }

    #[test]
    fn test_rapid_state_changes_error_handling() {
        let store = Store::new();
        let topology = create_unstable_topology();
        let system = Arc::new(System::new().expect("Failed to create system"));
        
        store.dispatch(AppAction::SetTopology(topology));
        store.dispatch(AppAction::SetSystem(system));
        
        let mut control_view = ControlView::new(&store);
        
        // Perform rapid state changes that could cause race conditions
        for i in 0..20 {
            // Rapid selection changes
            let uuid = format!("RINCON_RAPID_{}", i);
            store.dispatch(AppAction::SetSelectedSpeakerUuid(uuid));
            
            // Rapid command attempts
            let space_key = KeyEvent::from(KeyCode::Char(' '));
            let result = control_view.handle_input(space_key, &store);
            assert!(result.is_ok(), "Rapid command {} should be handled", i);
            
            // Rapid status updates
            store.dispatch(AppAction::SetStatusMessage(format!("Rapid update {}", i)));
            
            // Occasional topology updates
            if i % 5 == 0 {
                let new_topology = if i % 10 == 0 {
                    create_empty_topology()
                } else {
                    create_unstable_topology()
                };
                store.dispatch(AppAction::SetTopology(new_topology));
            }
        }
        
        // Verify system remains stable
        store.with_state(|state| {
            assert!(state.topology.is_some());
            assert!(state.system.is_some());
            assert!(!state.status_message.is_empty());
        });
    }

    #[test]
    fn test_error_recovery_after_system_failure() {
        let store = Store::new();
        let topology = create_unstable_topology();
        let system = Arc::new(System::new().expect("Failed to create system"));
        
        store.dispatch(AppAction::SetTopology(topology));
        store.dispatch(AppAction::SetSystem(system));
        store.dispatch(AppAction::SetSelectedSpeakerUuid("RINCON_UNSTABLE_001".to_string()));
        
        let mut control_view = ControlView::new(&store);
        
        // Simulate system failure by removing system reference
        store.dispatch(AppAction::SetSystem(Arc::new(System::new().expect("New system"))));
        
        // Try commands that will fail
        for _ in 0..5 {
            let space_key = KeyEvent::from(KeyCode::Char(' '));
            let result = control_view.handle_input(space_key, &store);
            assert!(result.is_ok(), "Failed commands should be handled gracefully");
        }
        
        // Restore system and verify recovery
        let new_system = Arc::new(System::new().expect("Recovery system"));
        store.dispatch(AppAction::SetSystem(new_system));
        
        // Commands should now be attempted again
        let enter_key = KeyEvent::from(KeyCode::Enter);
        let result = control_view.handle_input(enter_key, &store);
        assert!(result.is_ok(), "Commands should work after system recovery");
        
        // Verify system is functional
        store.with_state(|state| {
            assert!(state.system.is_some());
            assert!(state.topology.is_some());
        });
    }

    #[test]
    fn test_memory_safety_during_error_conditions() {
        let store = Store::new();
        
        // Create and destroy multiple topologies to test memory handling
        for i in 0..10 {
            let topology = Topology {
                groups: vec![
                    Group {
                        name: format!("Test Group {}", i),
                        speakers: vec![
                            SpeakerInfo {
                                name: format!("Test Speaker {}", i),
                                uuid: format!("RINCON_TEST_{:03}", i),
                                ip: format!("192.168.1.{}", 100 + i),
                                is_coordinator: true,
                            },
                        ],
                    },
                ],
            };
            
            store.dispatch(AppAction::SetTopology(topology));
            
            // Create and destroy systems
            let system = Arc::new(System::new().expect("Failed to create system"));
            store.dispatch(AppAction::SetSystem(system));
            
            // Set selections that will become invalid
            store.dispatch(AppAction::SetSelectedSpeakerUuid(format!("RINCON_TEST_{:03}", i)));
            
            // Create control view (will be dropped at end of iteration)
            let mut control_view = ControlView::new(&store);
            
            // Perform operations that might cause memory issues
            let space_key = KeyEvent::from(KeyCode::Char(' '));
            let _result = control_view.handle_input(space_key, &store);
        }
        
        // Verify final state is clean
        store.with_state(|state| {
            assert!(state.topology.is_some());
            assert!(state.system.is_some());
        });
    }

    #[test]
    fn test_error_message_consistency_and_clarity() {
        let store = Store::new();
        let topology = create_unstable_topology();
        let system = Arc::new(System::new().expect("Failed to create system"));
        
        store.dispatch(AppAction::SetTopology(topology));
        store.dispatch(AppAction::SetSystem(system));
        
        let mut control_view = ControlView::new(&store);
        
        // Test different error scenarios and verify message consistency
        
        // No selection error
        store.dispatch(AppAction::ClearSelection);
        let space_key = KeyEvent::from(KeyCode::Char(' '));
        let _result = control_view.handle_input(space_key, &store);
        
        store.with_state(|state| {
            let message = &state.status_message;
            assert!(message.contains("failed") || message.contains("No selection"));
            assert!(!message.is_empty());
        });
        
        // Invalid selection error
        store.dispatch(AppAction::SetSelectedSpeakerUuid("RINCON_INVALID".to_string()));
        let enter_key = KeyEvent::from(KeyCode::Enter);
        let _result = control_view.handle_input(enter_key, &store);
        
        store.with_state(|state| {
            let message = &state.status_message;
            assert!(message.contains("failed"));
            assert!(!message.is_empty());
        });
        
        // System unavailable error
        store.dispatch(AppAction::SetSystem(Arc::new(System::new().expect("Empty system"))));
        store.dispatch(AppAction::SetSelectedSpeakerUuid("RINCON_UNSTABLE_001".to_string()));
        let space_key = KeyEvent::from(KeyCode::Char(' '));
        let _result = control_view.handle_input(space_key, &store);
        
        store.with_state(|state| {
            let message = &state.status_message;
            assert!(!message.is_empty());
            // Message should be informative about the failure
            assert!(message.len() > 5);
        });
    }

    #[test]
    fn test_edge_case_uuid_handling() {
        let store = Store::new();
        
        // Create topology with edge case UUIDs
        let edge_case_topology = Topology {
            groups: vec![
                Group {
                    name: "Edge Case Group".to_string(),
                    speakers: vec![
                        SpeakerInfo {
                            name: "Empty UUID Speaker".to_string(),
                            uuid: "".to_string(), // Empty UUID
                            ip: "192.168.1.100".to_string(),
                            is_coordinator: true,
                        },
                        SpeakerInfo {
                            name: "Special Chars Speaker".to_string(),
                            uuid: "RINCON_!@#$%^&*()".to_string(), // Special characters
                            ip: "192.168.1.101".to_string(),
                            is_coordinator: false,
                        },
                    ],
                },
            ],
        };
        
        let system = Arc::new(System::new().expect("Failed to create system"));
        
        store.dispatch(AppAction::SetTopology(edge_case_topology));
        store.dispatch(AppAction::SetSystem(system));
        
        // Test lookups with edge case UUIDs
        store.with_state(|state| {
            if let Some(topology) = &state.topology {
                // Empty UUID lookup
                let empty_speaker = topology.get_speaker_by_uuid("");
                assert!(empty_speaker.is_some());
                assert_eq!(empty_speaker.unwrap().name, "Empty UUID Speaker");
                
                // Special characters UUID lookup
                let special_speaker = topology.get_speaker_by_uuid("RINCON_!@#$%^&*()");
                assert!(special_speaker.is_some());
                assert_eq!(special_speaker.unwrap().name, "Special Chars Speaker");
                
                // Group lookup with empty UUID coordinator
                let group_by_empty = topology.get_group_by_coordinator_uuid("");
                assert!(group_by_empty.is_some());
                assert_eq!(group_by_empty.unwrap().name, "Edge Case Group");
            }
        });
        
        // Test command execution with edge case selections
        store.dispatch(AppAction::SetSelectedSpeakerUuid("".to_string()));
        
        let mut control_view = ControlView::new(&store);
        let space_key = KeyEvent::from(KeyCode::Char(' '));
        let result = control_view.handle_input(space_key, &store);
        assert!(result.is_ok(), "Should handle empty UUID selection");
        
        store.dispatch(AppAction::SetSelectedSpeakerUuid("RINCON_!@#$%^&*()".to_string()));
        let enter_key = KeyEvent::from(KeyCode::Enter);
        let result = control_view.handle_input(enter_key, &store);
        assert!(result.is_ok(), "Should handle special character UUID selection");
    }

    #[test]
    fn test_large_topology_error_handling() {
        let store = Store::new();
        
        // Create a large topology that might cause performance issues
        let mut groups = Vec::new();
        for i in 0..1000 {
            groups.push(Group {
                name: format!("Group {}", i),
                speakers: vec![
                    SpeakerInfo {
                        name: format!("Speaker {}", i),
                        uuid: format!("RINCON_LARGE_{:04}", i),
                        ip: format!("192.168.{}.{}", (i / 255) + 1, (i % 255) + 1),
                        is_coordinator: true,
                    },
                ],
            });
        }
        
        let large_topology = Topology { groups };
        let system = Arc::new(System::new().expect("Failed to create system"));
        
        store.dispatch(AppAction::SetTopology(large_topology));
        store.dispatch(AppAction::SetSystem(system));
        
        let mut control_view = ControlView::new(&store);
        
        // Test that large topology doesn't cause performance issues during errors
        let start_time = std::time::Instant::now();
        
        // Try multiple invalid operations
        for i in 0..10 {
            store.dispatch(AppAction::SetSelectedSpeakerUuid(format!("RINCON_INVALID_{}", i)));
            
            let space_key = KeyEvent::from(KeyCode::Char(' '));
            let result = control_view.handle_input(space_key, &store);
            assert!(result.is_ok(), "Large topology error handling should be fast");
        }
        
        let elapsed = start_time.elapsed();
        assert!(elapsed.as_millis() < 200, "Large topology error handling took too long: {:?}", elapsed);
        
        // Verify system remains responsive
        store.with_state(|state| {
            assert!(state.topology.is_some());
            assert!(state.system.is_some());
            assert_eq!(state.topology.as_ref().unwrap().groups.len(), 1000);
        });
    }

    #[test]
    fn test_error_handling_with_corrupted_state() {
        let store = Store::new();
        let topology = create_unstable_topology();
        let system = Arc::new(System::new().expect("Failed to create system"));
        
        store.dispatch(AppAction::SetTopology(topology));
        store.dispatch(AppAction::SetSystem(system));
        
        // Create a scenario where state becomes inconsistent
        store.dispatch(AppAction::SetSelectedSpeakerUuid("RINCON_UNSTABLE_001".to_string()));
        store.dispatch(AppAction::SetSelectedGroupUuid("RINCON_DIFFERENT_GROUP".to_string()));
        
        let mut control_view = ControlView::new(&store);
        
        // Test command execution with inconsistent selection state
        let space_key = KeyEvent::from(KeyCode::Char(' '));
        let result = control_view.handle_input(space_key, &store);
        assert!(result.is_ok(), "Should handle inconsistent selection state");
        
        let enter_key = KeyEvent::from(KeyCode::Enter);
        let result = control_view.handle_input(enter_key, &store);
        assert!(result.is_ok(), "Should handle inconsistent selection state");
        
        // Verify error messages are appropriate
        store.with_state(|state| {
            assert!(!state.status_message.is_empty());
            assert!(state.status_message.contains("failed") || state.status_message.contains("No selection"));
        });
    }
}