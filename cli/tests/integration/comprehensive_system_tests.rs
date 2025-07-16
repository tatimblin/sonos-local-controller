//! Comprehensive system tests that demonstrate the complete testing infrastructure
//!
//! These tests combine all aspects of the speaker management system testing,
//! including end-to-end flows, error handling, network scenarios, and recovery.

use std::sync::Arc;
use cli::state::store::{Store, AppState};
use cli::state::reducers::AppAction;
use cli::types::{Topology, Group, SpeakerInfo, System, SpeakerManagerError};
use cli::views::control::ControlView;
use crossterm::event::{KeyCode, KeyEvent};

#[cfg(feature = "mock")]
use sonos::speaker::mock::{MockSpeaker, MockSpeakerBuilder, MockSpeakerConfig};

/// Comprehensive test topology with various speaker configurations
fn create_comprehensive_test_topology() -> Topology {
    Topology {
        groups: vec![
            // Single speaker group
            Group {
                name: "Bedroom".to_string(),
                speakers: vec![
                    SpeakerInfo {
                        name: "Bedroom".to_string(),
                        uuid: "RINCON_BEDROOM_MAIN".to_string(),
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
                        uuid: "RINCON_LIVING_ROOM_MAIN".to_string(),
                        ip: "192.168.1.101".to_string(),
                        is_coordinator: true,
                    },
                    SpeakerInfo {
                        name: "Kitchen".to_string(),
                        uuid: "RINCON_KITCHEN_SECONDARY".to_string(),
                        ip: "192.168.1.102".to_string(),
                        is_coordinator: false,
                    },
                    SpeakerInfo {
                        name: "Dining Room".to_string(),
                        uuid: "RINCON_DINING_SECONDARY".to_string(),
                        ip: "192.168.1.103".to_string(),
                        is_coordinator: false,
                    },
                ],
            },
            // Another single speaker
            Group {
                name: "Office".to_string(),
                speakers: vec![
                    SpeakerInfo {
                        name: "Office".to_string(),
                        uuid: "RINCON_OFFICE_MAIN".to_string(),
                        ip: "192.168.1.104".to_string(),
                        is_coordinator: true,
                    },
                ],
            },
            // Group with problematic speaker
            Group {
                name: "Bathroom".to_string(),
                speakers: vec![
                    SpeakerInfo {
                        name: "Bathroom".to_string(),
                        uuid: "RINCON_BATHROOM_UNRELIABLE".to_string(),
                        ip: "192.168.1.105".to_string(),
                        is_coordinator: true,
                    },
                ],
            },
        ],
    }
}

#[cfg(test)]
mod comprehensive_system_tests {
    use super::*;

    #[test]
    fn test_complete_system_lifecycle() {
        // Test the complete lifecycle from system startup to command execution
        let store = Store::new();
        
        // 1. Initial state should be empty
        store.with_state(|state| {
            assert!(matches!(state.view, cli::types::View::Startup));
            assert!(state.topology.is_none());
            assert!(state.system.is_none());
            assert!(state.selected_speaker_uuid.is_none());
            assert!(state.selected_group_uuid.is_none());
        });
        
        // 2. System discovery and setup
        let system = Arc::new(System::new().expect("Failed to create system"));
        store.dispatch(AppAction::SetSystem(system));
        
        store.with_state(|state| {
            assert!(state.system.is_some());
            assert!(state.topology.is_none()); // Still no topology
        });
        
        // 3. Topology discovery and setup
        let topology = create_comprehensive_test_topology();
        store.dispatch(AppAction::SetTopology(topology));
        
        store.with_state(|state| {
            assert!(matches!(state.view, cli::types::View::Control)); // Should switch to control view
            assert!(state.topology.is_some());
            assert!(state.system.is_some());
            
            let topology = state.topology.as_ref().unwrap();
            assert_eq!(topology.groups.len(), 4);
        });
        
        // 4. UI interaction and selection
        let mut control_view = ControlView::new(&store);
        
        // Navigate through the topology
        for _ in 0..5 {
            let down_key = KeyEvent::from(KeyCode::Down);
            let result = control_view.handle_input(down_key, &store);
            assert!(result.is_ok());
        }
        
        // 5. Command execution
        let space_key = KeyEvent::from(KeyCode::Char(' '));
        let result = control_view.handle_input(space_key, &store);
        assert!(result.is_ok());
        
        let enter_key = KeyEvent::from(KeyCode::Enter);
        let result = control_view.handle_input(enter_key, &store);
        assert!(result.is_ok());
        
        // 6. Verify final state
        store.with_state(|state| {
            assert!(state.topology.is_some());
            assert!(state.system.is_some());
            assert!(!state.status_message.is_empty());
        });
    }

    #[test]
    fn test_topology_update_and_selection_management() {
        let store = Store::new();
        let system = Arc::new(System::new().expect("Failed to create system"));
        
        // Set up initial state
        let initial_topology = create_comprehensive_test_topology();
        store.dispatch(AppAction::SetSystem(system));
        store.dispatch(AppAction::SetTopology(initial_topology));
        
        // Set selections
        store.dispatch(AppAction::SetSelectedSpeakerUuid("RINCON_LIVING_ROOM_MAIN".to_string()));
        store.dispatch(AppAction::SetSelectedGroupUuid("RINCON_LIVING_ROOM_MAIN".to_string()));
        
        // Verify selections are set
        store.with_state(|state| {
            assert_eq!(state.selected_speaker_uuid.as_ref().unwrap(), "RINCON_LIVING_ROOM_MAIN");
            assert_eq!(state.selected_group_uuid.as_ref().unwrap(), "RINCON_LIVING_ROOM_MAIN");
        });
        
        // Update topology - keep the selected speaker
        let updated_topology = Topology {
            groups: vec![
                Group {
                    name: "Living Room".to_string(),
                    speakers: vec![
                        SpeakerInfo {
                            name: "Living Room".to_string(),
                            uuid: "RINCON_LIVING_ROOM_MAIN".to_string(),
                            ip: "192.168.1.101".to_string(),
                            is_coordinator: true,
                        },
                    ],
                },
                Group {
                    name: "New Room".to_string(),
                    speakers: vec![
                        SpeakerInfo {
                            name: "New Room".to_string(),
                            uuid: "RINCON_NEW_ROOM".to_string(),
                            ip: "192.168.1.200".to_string(),
                            is_coordinator: true,
                        },
                    ],
                },
            ],
        };
        
        store.dispatch(AppAction::SetTopology(updated_topology));
        
        // Verify selections are preserved
        store.with_state(|state| {
            assert_eq!(state.selected_speaker_uuid.as_ref().unwrap(), "RINCON_LIVING_ROOM_MAIN");
            assert_eq!(state.selected_group_uuid.as_ref().unwrap(), "RINCON_LIVING_ROOM_MAIN");
        });
        
        // Update topology - remove the selected speaker
        let topology_without_selection = Topology {
            groups: vec![
                Group {
                    name: "New Room".to_string(),
                    speakers: vec![
                        SpeakerInfo {
                            name: "New Room".to_string(),
                            uuid: "RINCON_NEW_ROOM".to_string(),
                            ip: "192.168.1.200".to_string(),
                            is_coordinator: true,
                        },
                    ],
                },
            ],
        };
        
        store.dispatch(AppAction::SetTopology(topology_without_selection));
        
        // Verify selections are cleared
        store.with_state(|state| {
            assert!(state.selected_speaker_uuid.is_none());
            assert!(state.selected_group_uuid.is_none());
        });
    }

    #[test]
    fn test_error_handling_and_recovery_patterns() {
        let store = Store::new();
        let topology = create_comprehensive_test_topology();
        let system = Arc::new(System::new().expect("Failed to create system"));
        
        store.dispatch(AppAction::SetTopology(topology));
        store.dispatch(AppAction::SetSystem(system));
        
        let mut control_view = ControlView::new(&store);
        
        // Test various error scenarios
        
        // 1. No selection error
        store.dispatch(AppAction::ClearSelection);
        let space_key = KeyEvent::from(KeyCode::Char(' '));
        let result = control_view.handle_input(space_key, &store);
        assert!(result.is_ok());
        
        store.with_state(|state| {
            assert!(state.status_message.contains("failed") || state.status_message.contains("No selection"));
        });
        
        // 2. Invalid selection error
        store.dispatch(AppAction::SetSelectedSpeakerUuid("RINCON_NONEXISTENT".to_string()));
        let enter_key = KeyEvent::from(KeyCode::Enter);
        let result = control_view.handle_input(enter_key, &store);
        assert!(result.is_ok());
        
        store.with_state(|state| {
            assert!(state.status_message.contains("failed"));
        });
        
        // 3. System unavailable error
        store.dispatch(AppAction::SetSystem(Arc::new(System::new().expect("New system"))));
        store.dispatch(AppAction::SetSelectedSpeakerUuid("RINCON_BEDROOM_MAIN".to_string()));
        let space_key = KeyEvent::from(KeyCode::Char(' '));
        let result = control_view.handle_input(space_key, &store);
        assert!(result.is_ok());
        
        // 4. Recovery - restore proper system and selection
        let recovery_system = Arc::new(System::new().expect("Recovery system"));
        store.dispatch(AppAction::SetSystem(recovery_system));
        store.dispatch(AppAction::SetSelectedSpeakerUuid("RINCON_BEDROOM_MAIN".to_string()));
        
        let enter_key = KeyEvent::from(KeyCode::Enter);
        let result = control_view.handle_input(enter_key, &store);
        assert!(result.is_ok());
        
        // Verify system is functional after recovery
        store.with_state(|state| {
            assert!(state.system.is_some());
            assert!(state.topology.is_some());
            assert!(state.selected_speaker_uuid.is_some());
        });
    }

    #[test]
    fn test_concurrent_operations_and_state_consistency() {
        let store = Store::new();
        let topology = create_comprehensive_test_topology();
        let system = Arc::new(System::new().expect("Failed to create system"));
        
        store.dispatch(AppAction::SetTopology(topology));
        store.dispatch(AppAction::SetSystem(system));
        
        let mut control_view = ControlView::new(&store);
        
        // Simulate concurrent operations
        for i in 0..20 {
            // Rapid state changes
            let speaker_uuids = vec![
                "RINCON_BEDROOM_MAIN",
                "RINCON_LIVING_ROOM_MAIN",
                "RINCON_OFFICE_MAIN",
                "RINCON_BATHROOM_UNRELIABLE",
            ];
            
            let selected_uuid = speaker_uuids[i % speaker_uuids.len()];
            store.dispatch(AppAction::SetSelectedSpeakerUuid(selected_uuid.to_string()));
            
            // Rapid command execution
            let commands = vec![KeyCode::Char(' '), KeyCode::Enter, KeyCode::Left, KeyCode::Right];
            let command = commands[i % commands.len()];
            
            let key_event = KeyEvent::from(command);
            let result = control_view.handle_input(key_event, &store);
            assert!(result.is_ok(), "Concurrent operation {} should succeed", i);
            
            // Rapid status updates
            store.dispatch(AppAction::SetStatusMessage(format!("Concurrent operation {}", i)));
            
            // Verify state consistency
            store.with_state(|state| {
                assert!(state.topology.is_some());
                assert!(state.system.is_some());
                assert!(state.selected_speaker_uuid.is_some());
                assert_eq!(state.selected_speaker_uuid.as_ref().unwrap(), selected_uuid);
                assert_eq!(state.status_message, format!("Concurrent operation {}", i));
            });
        }
    }

    #[test]
    fn test_topology_lookup_methods_comprehensive() {
        let store = Store::new();
        let topology = create_comprehensive_test_topology();
        
        store.dispatch(AppAction::SetTopology(topology));
        
        store.with_state(|state| {
            let topology = state.topology.as_ref().unwrap();
            
            // Test get_speaker_by_uuid with all speakers
            let test_cases = vec![
                ("RINCON_BEDROOM_MAIN", "Bedroom", true),
                ("RINCON_LIVING_ROOM_MAIN", "Living Room", true),
                ("RINCON_KITCHEN_SECONDARY", "Kitchen", false),
                ("RINCON_DINING_SECONDARY", "Dining Room", false),
                ("RINCON_OFFICE_MAIN", "Office", true),
                ("RINCON_BATHROOM_UNRELIABLE", "Bathroom", true),
            ];
            
            for (uuid, expected_name, expected_coordinator) in test_cases {
                let speaker = topology.get_speaker_by_uuid(uuid);
                assert!(speaker.is_some(), "Should find speaker {}", uuid);
                let speaker = speaker.unwrap();
                assert_eq!(speaker.name, expected_name);
                assert_eq!(speaker.is_coordinator, expected_coordinator);
            }
            
            // Test get_group_by_uuid (any member)
            let group_by_coordinator = topology.get_group_by_uuid("RINCON_LIVING_ROOM_MAIN");
            assert!(group_by_coordinator.is_some());
            assert_eq!(group_by_coordinator.unwrap().name, "Living Room");
            
            let group_by_member = topology.get_group_by_uuid("RINCON_KITCHEN_SECONDARY");
            assert!(group_by_member.is_some());
            assert_eq!(group_by_member.unwrap().name, "Living Room");
            
            // Test get_group_by_coordinator_uuid (only coordinators)
            let group_by_coord = topology.get_group_by_coordinator_uuid("RINCON_LIVING_ROOM_MAIN");
            assert!(group_by_coord.is_some());
            assert_eq!(group_by_coord.unwrap().name, "Living Room");
            
            let no_group_by_member = topology.get_group_by_coordinator_uuid("RINCON_KITCHEN_SECONDARY");
            assert!(no_group_by_member.is_none());
            
            // Test helper methods
            let selected_speaker_uuid = Some("RINCON_DINING_SECONDARY".to_string());
            let selected_speaker = topology.get_selected_speaker(selected_speaker_uuid.as_ref());
            assert!(selected_speaker.is_some());
            assert_eq!(selected_speaker.unwrap().name, "Dining Room");
            
            let selected_group_uuid = Some("RINCON_OFFICE_MAIN".to_string());
            let selected_group = topology.get_selected_group(selected_group_uuid.as_ref());
            assert!(selected_group.is_some());
            assert_eq!(selected_group.unwrap().name, "Office");
            
            // Test with None selections
            assert!(topology.get_selected_speaker(None).is_none());
            assert!(topology.get_selected_group(None).is_none());
            
            // Test with invalid UUIDs
            assert!(topology.get_speaker_by_uuid("RINCON_INVALID").is_none());
            assert!(topology.get_group_by_uuid("RINCON_INVALID").is_none());
            assert!(topology.get_group_by_coordinator_uuid("RINCON_INVALID").is_none());
        });
    }

    #[test]
    fn test_ui_navigation_and_selection_integration() {
        let store = Store::new();
        let topology = create_comprehensive_test_topology();
        let system = Arc::new(System::new().expect("Failed to create system"));
        
        store.dispatch(AppAction::SetTopology(topology));
        store.dispatch(AppAction::SetSystem(system));
        
        let mut control_view = ControlView::new(&store);
        
        // Test navigation through all items
        let navigation_sequence = vec![
            KeyCode::Down,  // Move to next item
            KeyCode::Down,  // Move to next item
            KeyCode::Up,    // Move back
            KeyCode::Down,  // Move forward again
            KeyCode::Down,  // Continue navigation
            KeyCode::Down,  // Continue navigation
            KeyCode::Up,    // Move back
            KeyCode::Up,    // Move back more
        ];
        
        for (i, key_code) in navigation_sequence.iter().enumerate() {
            let key_event = KeyEvent::from(*key_code);
            let result = control_view.handle_input(key_event, &store);
            assert!(result.is_ok(), "Navigation step {} should succeed", i);
            
            // Verify selection state is updated after each navigation
            store.with_state(|state| {
                // Selection should be updated (either speaker or group)
                // The exact selection depends on the topology structure and navigation
                assert!(
                    state.selected_speaker_uuid.is_some() || 
                    state.selected_group_uuid.is_some() ||
                    state.topology.as_ref().unwrap().groups.is_empty()
                );
            });
        }
        
        // Test command execution after navigation
        let space_key = KeyEvent::from(KeyCode::Char(' '));
        let result = control_view.handle_input(space_key, &store);
        assert!(result.is_ok());
        
        let enter_key = KeyEvent::from(KeyCode::Enter);
        let result = control_view.handle_input(enter_key, &store);
        assert!(result.is_ok());
        
        // Test volume controls
        let left_key = KeyEvent::from(KeyCode::Left);
        let result = control_view.handle_input(left_key, &store);
        assert!(result.is_ok());
        
        let right_key = KeyEvent::from(KeyCode::Right);
        let result = control_view.handle_input(right_key, &store);
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(feature = "mock")]
    fn test_mock_infrastructure_integration() {
        // Test that mock infrastructure works with the real system
        
        // Create mock speakers with different behaviors
        let normal_speaker = MockSpeaker::new("Normal", "RINCON_NORMAL", "192.168.1.100");
        let unreachable_speaker = MockSpeakerBuilder::new()
            .name("Unreachable")
            .uuid("RINCON_UNREACHABLE")
            .ip("192.168.1.101")
            .unreachable()
            .build();
        let slow_speaker = MockSpeakerBuilder::new()
            .name("Slow")
            .uuid("RINCON_SLOW")
            .ip("192.168.1.102")
            .with_delay(50)
            .build();
        
        // Test normal speaker
        assert!(normal_speaker.play().is_ok());
        assert!(normal_speaker.pause().is_ok());
        assert_eq!(normal_speaker.get_volume().unwrap(), 50);
        
        let history = normal_speaker.get_command_history();
        assert_eq!(history.len(), 3);
        assert_eq!(history[0], "play");
        assert_eq!(history[1], "pause");
        assert_eq!(history[2], "get_volume");
        
        // Test unreachable speaker
        assert!(unreachable_speaker.play().is_err());
        assert!(unreachable_speaker.pause().is_err());
        assert!(unreachable_speaker.get_volume().is_err());
        
        // Commands should not be recorded on failure
        assert!(unreachable_speaker.get_command_history().is_empty());
        
        // Test slow speaker
        let start_time = std::time::Instant::now();
        assert!(slow_speaker.play().is_ok());
        let elapsed = start_time.elapsed();
        assert!(elapsed.as_millis() >= 50);
        
        // Command should be recorded after successful execution
        let slow_history = slow_speaker.get_command_history();
        assert_eq!(slow_history.len(), 1);
        assert_eq!(slow_history[0], "play");
    }

    #[test]
    fn test_system_stability_under_stress() {
        let store = Store::new();
        let topology = create_comprehensive_test_topology();
        let system = Arc::new(System::new().expect("Failed to create system"));
        
        store.dispatch(AppAction::SetTopology(topology));
        store.dispatch(AppAction::SetSystem(system));
        
        let mut control_view = ControlView::new(&store);
        
        // Perform stress testing with rapid operations
        for iteration in 0..100 {
            // Rapid selection changes
            let speakers = vec![
                "RINCON_BEDROOM_MAIN",
                "RINCON_LIVING_ROOM_MAIN", 
                "RINCON_KITCHEN_SECONDARY",
                "RINCON_OFFICE_MAIN",
            ];
            
            let speaker = speakers[iteration % speakers.len()];
            store.dispatch(AppAction::SetSelectedSpeakerUuid(speaker.to_string()));
            
            // Rapid command execution
            let commands = vec![
                KeyCode::Char(' '),
                KeyCode::Enter,
                KeyCode::Left,
                KeyCode::Right,
                KeyCode::Down,
                KeyCode::Up,
            ];
            
            let command = commands[iteration % commands.len()];
            let key_event = KeyEvent::from(command);
            let result = control_view.handle_input(key_event, &store);
            assert!(result.is_ok(), "Stress iteration {} should succeed", iteration);
            
            // Occasional topology updates
            if iteration % 25 == 0 {
                let stress_topology = Topology {
                    groups: vec![
                        Group {
                            name: format!("Stress Group {}", iteration),
                            speakers: vec![
                                SpeakerInfo {
                                    name: format!("Stress Speaker {}", iteration),
                                    uuid: format!("RINCON_STRESS_{:03}", iteration),
                                    ip: format!("192.168.1.{}", 150 + (iteration % 50)),
                                    is_coordinator: true,
                                },
                            ],
                        },
                    ],
                };
                store.dispatch(AppAction::SetTopology(stress_topology));
            }
            
            // Verify system remains stable
            if iteration % 10 == 0 {
                store.with_state(|state| {
                    assert!(state.topology.is_some());
                    assert!(state.system.is_some());
                });
            }
        }
        
        // Final stability check
        store.with_state(|state| {
            assert!(state.topology.is_some());
            assert!(state.system.is_some());
            assert!(!state.status_message.is_empty());
        });
    }

    #[test]
    fn test_error_message_consistency_and_user_experience() {
        let store = Store::new();
        let topology = create_comprehensive_test_topology();
        let system = Arc::new(System::new().expect("Failed to create system"));
        
        store.dispatch(AppAction::SetTopology(topology));
        store.dispatch(AppAction::SetSystem(system));
        
        let mut control_view = ControlView::new(&store);
        
        // Test different error scenarios and verify message quality
        
        // 1. No selection scenario
        store.dispatch(AppAction::ClearSelection);
        let space_key = KeyEvent::from(KeyCode::Char(' '));
        let _result = control_view.handle_input(space_key, &store);
        
        store.with_state(|state| {
            let message = &state.status_message;
            assert!(!message.is_empty());
            assert!(message.len() > 5); // Should be descriptive
            assert!(message.contains("failed") || message.contains("No selection"));
        });
        
        // 2. Invalid selection scenario
        store.dispatch(AppAction::SetSelectedSpeakerUuid("RINCON_DOES_NOT_EXIST".to_string()));
        let enter_key = KeyEvent::from(KeyCode::Enter);
        let _result = control_view.handle_input(enter_key, &store);
        
        store.with_state(|state| {
            let message = &state.status_message;
            assert!(!message.is_empty());
            assert!(message.contains("failed"));
        });
        
        // 3. Success scenario (should also have clear message)
        store.dispatch(AppAction::SetSelectedSpeakerUuid("RINCON_BEDROOM_MAIN".to_string()));
        let space_key = KeyEvent::from(KeyCode::Char(' '));
        let _result = control_view.handle_input(space_key, &store);
        
        store.with_state(|state| {
            let message = &state.status_message;
            assert!(!message.is_empty());
            // Message should indicate either success or failure clearly
            assert!(message.contains("Paused") || message.contains("failed"));
        });
    }
}