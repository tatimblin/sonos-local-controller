//! Integration tests for network scenarios using mock infrastructure
//!
//! These tests simulate various network conditions including failures,
//! timeouts, and recovery scenarios to ensure the system handles
//! network issues gracefully.

use std::sync::Arc;
use cli::state::store::{Store, AppState};
use cli::state::reducers::AppAction;
use cli::types::{Topology, Group, SpeakerInfo, System, SpeakerManagerError};
use cli::views::control::ControlView;
use crossterm::event::{KeyCode, KeyEvent};

#[cfg(feature = "mock")]
use sonos::speaker::mock::{MockSpeaker, MockSpeakerBuilder, MockSpeakerConfig, MockSystem, MockNetworkConfig, MockZoneGroup};

/// Helper to create a topology that matches mock speakers
fn create_mock_topology() -> Topology {
    Topology {
        groups: vec![
            Group {
                name: "Mock Living Room".to_string(),
                speakers: vec![
                    SpeakerInfo {
                        name: "Mock Living Room".to_string(),
                        uuid: "RINCON_MOCK_LIVING_001".to_string(),
                        ip: "192.168.1.100".to_string(),
                        is_coordinator: true,
                    },
                    SpeakerInfo {
                        name: "Mock Kitchen".to_string(),
                        uuid: "RINCON_MOCK_KITCHEN_001".to_string(),
                        ip: "192.168.1.101".to_string(),
                        is_coordinator: false,
                    },
                ],
            },
            Group {
                name: "Mock Bedroom".to_string(),
                speakers: vec![
                    SpeakerInfo {
                        name: "Mock Bedroom".to_string(),
                        uuid: "RINCON_MOCK_BEDROOM_001".to_string(),
                        ip: "192.168.1.102".to_string(),
                        is_coordinator: true,
                    },
                ],
            },
        ],
    }
}

#[cfg(feature = "mock")]
fn create_mock_system_with_speakers() -> MockSystem {
    let mut system = MockSystem::new();
    
    // Add mock speakers
    let living_room = Box::new(MockSpeaker::new(
        "Mock Living Room",
        "RINCON_MOCK_LIVING_001",
        "192.168.1.100"
    ));
    let kitchen = Box::new(MockSpeaker::new(
        "Mock Kitchen",
        "RINCON_MOCK_KITCHEN_001",
        "192.168.1.101"
    ));
    let bedroom = Box::new(MockSpeaker::new(
        "Mock Bedroom",
        "RINCON_MOCK_BEDROOM_001",
        "192.168.1.102"
    ));
    
    system.add_speaker(living_room);
    system.add_speaker(kitchen);
    system.add_speaker(bedroom);
    
    // Add zone groups
    let mut living_room_group = MockZoneGroup::new("RINCON_MOCK_LIVING_001", "Mock Living Room");
    living_room_group.add_member("RINCON_MOCK_KITCHEN_001");
    system.add_zone_group(living_room_group);
    
    let bedroom_group = MockZoneGroup::new("RINCON_MOCK_BEDROOM_001", "Mock Bedroom");
    system.add_zone_group(bedroom_group);
    
    system
}

#[cfg(test)]
#[cfg(feature = "mock")]
mod network_scenario_tests {
    use super::*;

    #[test]
    fn test_speaker_unreachable_scenario() {
        let store = Store::new();
        let topology = create_mock_topology();
        
        // Create system with unreachable speaker
        let mut mock_system = create_mock_system_with_speakers();
        
        // Make one speaker unreachable
        let unreachable_speaker = Box::new(
            MockSpeakerBuilder::new()
                .name("Unreachable Speaker")
                .uuid("RINCON_MOCK_BEDROOM_001")
                .ip("192.168.1.102")
                .unreachable()
                .build()
        );
        mock_system.add_speaker(unreachable_speaker);
        
        // Set up state
        store.dispatch(AppAction::SetTopology(topology));
        // Note: We can't directly use MockSystem with the real System type
        // This test demonstrates the pattern for network failure testing
        
        let system = Arc::new(System::new().expect("Failed to create system"));
        store.dispatch(AppAction::SetSystem(system));
        store.dispatch(AppAction::SetSelectedSpeakerUuid("RINCON_MOCK_BEDROOM_001".to_string()));
        
        let mut control_view = ControlView::new(&store);
        
        // Test command execution with unreachable speaker
        let space_key = KeyEvent::from(KeyCode::Char(' '));
        let result = control_view.handle_input(space_key, &store);
        assert!(result.is_ok(), "Should handle unreachable speaker gracefully");
        
        // Verify error message was set
        store.with_state(|state| {
            assert!(state.status_message.contains("failed"));
        });
    }

    #[test]
    fn test_network_timeout_scenario() {
        let store = Store::new();
        let topology = create_mock_topology();
        
        // Create system with timeout-prone speaker
        let timeout_speaker = MockSpeakerBuilder::new()
            .name("Timeout Speaker")
            .uuid("RINCON_TIMEOUT_001")
            .ip("192.168.1.200")
            .with_timeout()
            .build();
        
        // Test timeout behavior
        let result = timeout_speaker.play();
        assert!(result.is_err());
        match result.unwrap_err() {
            sonos::SonosError::NetworkTimeout => {
                // Expected timeout error
            }
            _ => panic!("Expected NetworkTimeout error"),
        }
        
        // Test that commands are recorded even when they fail
        let history = timeout_speaker.get_command_history();
        assert!(history.is_empty()); // Command should not be recorded on failure
    }

    #[test]
    fn test_intermittent_network_failures() {
        let store = Store::new();
        let topology = create_mock_topology();
        let system = Arc::new(System::new().expect("Failed to create system"));
        
        store.dispatch(AppAction::SetTopology(topology));
        store.dispatch(AppAction::SetSystem(system));
        
        let mut control_view = ControlView::new(&store);
        
        // Simulate intermittent failures by trying multiple commands
        let commands = vec![
            KeyCode::Char(' '), // pause
            KeyCode::Enter,     // play
            KeyCode::Left,      // volume down
            KeyCode::Right,     // volume up
            KeyCode::Char(' '), // pause again
        ];
        
        for (i, key_code) in commands.iter().enumerate() {
            store.dispatch(AppAction::SetSelectedSpeakerUuid(
                format!("RINCON_MOCK_LIVING_{:03}", i % 2 + 1)
            ));
            
            let key_event = KeyEvent::from(*key_code);
            let result = control_view.handle_input(key_event, &store);
            assert!(result.is_ok(), "Command {} should be handled gracefully", i);
            
            // Each command should update status (either success or failure)
            store.with_state(|state| {
                assert!(!state.status_message.is_empty());
            });
        }
    }

    #[test]
    fn test_network_recovery_scenario() {
        // Create a speaker that starts as unreachable then becomes reachable
        let recovering_speaker = MockSpeakerBuilder::new()
            .name("Recovering Speaker")
            .uuid("RINCON_RECOVERING_001")
            .ip("192.168.1.150")
            .unreachable()
            .build();
        
        // Test initial failure
        let result = recovering_speaker.play();
        assert!(result.is_err());
        
        // Simulate network recovery
        recovering_speaker.set_config(MockSpeakerConfig::default());
        
        // Test recovery
        let result = recovering_speaker.play();
        assert!(result.is_ok());
        
        // Verify command was recorded after recovery
        let history = recovering_speaker.get_command_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0], "play");
    }

    #[test]
    fn test_slow_network_response_handling() {
        // Create speaker with network delay
        let slow_speaker = MockSpeakerBuilder::new()
            .name("Slow Speaker")
            .uuid("RINCON_SLOW_001")
            .ip("192.168.1.160")
            .with_delay(100) // 100ms delay
            .build();
        
        // Test that commands still work with delay
        let start_time = std::time::Instant::now();
        let result = slow_speaker.play();
        let elapsed = start_time.elapsed();
        
        assert!(result.is_ok());
        assert!(elapsed.as_millis() >= 100);
        
        // Verify command was recorded
        let history = slow_speaker.get_command_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0], "play");
    }

    #[test]
    fn test_parse_error_handling() {
        // Create speaker that returns parse errors
        let parse_error_speaker = MockSpeakerBuilder::new()
            .name("Parse Error Speaker")
            .uuid("RINCON_PARSE_ERROR_001")
            .ip("192.168.1.170")
            .with_parse_error()
            .build();
        
        // Test parse error handling
        let result = parse_error_speaker.get_volume();
        assert!(result.is_err());
        
        match result.unwrap_err() {
            sonos::SonosError::ParseError(msg) => {
                assert!(msg.contains("Mock parse error"));
            }
            _ => panic!("Expected ParseError"),
        }
    }

    #[test]
    fn test_custom_error_scenarios() {
        // Create speaker with custom error message
        let custom_error_speaker = MockSpeakerBuilder::new()
            .name("Custom Error Speaker")
            .uuid("RINCON_CUSTOM_001")
            .ip("192.168.1.180")
            .with_custom_error("Custom network failure: Connection refused")
            .build();
        
        // Test custom error handling
        let result = custom_error_speaker.pause();
        assert!(result.is_err());
        
        match result.unwrap_err() {
            sonos::SonosError::ParseError(msg) => {
                assert_eq!(msg, "Custom network failure: Connection refused");
            }
            _ => panic!("Expected custom ParseError"),
        }
    }

    #[test]
    fn test_volume_operations_with_network_issues() {
        // Create speaker for volume testing
        let volume_speaker = MockSpeaker::new(
            "Volume Test Speaker",
            "RINCON_VOLUME_001",
            "192.168.1.190"
        );
        
        // Test normal volume operations
        assert_eq!(volume_speaker.get_volume().unwrap(), 50);
        assert_eq!(volume_speaker.set_volume(75).unwrap(), 75);
        assert_eq!(volume_speaker.adjust_volume(10).unwrap(), 85);
        assert_eq!(volume_speaker.adjust_volume(-20).unwrap(), 65);
        
        // Test volume operations with network failure
        volume_speaker.set_config(MockSpeakerConfig {
            simulate_unreachable: true,
            ..Default::default()
        });
        
        assert!(volume_speaker.get_volume().is_err());
        assert!(volume_speaker.set_volume(50).is_err());
        assert!(volume_speaker.adjust_volume(5).is_err());
    }

    #[test]
    fn test_mock_system_discovery_failures() {
        let mut mock_system = MockSystem::new();
        
        // Configure discovery to fail
        mock_system.set_network_config(MockNetworkConfig {
            discovery_fails: true,
            ..Default::default()
        });
        
        // Test discovery failure
        let result = mock_system.simulate_discovery();
        assert!(result.is_err());
        
        match result.unwrap_err() {
            sonos::SonosError::NetworkTimeout => {
                // Expected discovery failure
            }
            _ => panic!("Expected NetworkTimeout for discovery failure"),
        }
    }

    #[test]
    fn test_mock_system_topology_retrieval_failures() {
        let mut mock_system = MockSystem::new();
        
        // Configure topology retrieval to fail
        mock_system.set_network_config(MockNetworkConfig {
            topology_retrieval_fails: true,
            ..Default::default()
        });
        
        // Test topology retrieval failure
        let result = mock_system.simulate_topology_retrieval();
        assert!(result.is_err());
        
        match result.unwrap_err() {
            sonos::SonosError::ParseError(msg) => {
                assert!(msg.contains("topology retrieval failed"));
            }
            _ => panic!("Expected ParseError for topology retrieval failure"),
        }
    }

    #[test]
    fn test_mock_system_speaker_lookup_failures() {
        let mut mock_system = create_mock_system_with_speakers();
        
        // Verify speakers exist normally
        assert!(mock_system.get_speaker_by_uuid("RINCON_MOCK_LIVING_001").is_some());
        
        // Configure speaker lookup to fail
        mock_system.set_network_config(MockNetworkConfig {
            speaker_lookup_fails: true,
            ..Default::default()
        });
        
        // Test speaker lookup failure
        assert!(mock_system.get_speaker_by_uuid("RINCON_MOCK_LIVING_001").is_none());
        assert!(mock_system.get_speaker_by_uuid("RINCON_MOCK_BEDROOM_001").is_none());
    }

    #[test]
    fn test_network_delay_simulation() {
        let mut mock_system = MockSystem::new();
        
        // Configure discovery delay
        mock_system.set_network_config(MockNetworkConfig {
            discovery_delay_ms: 50,
            ..Default::default()
        });
        
        // Test discovery with delay
        let start_time = std::time::Instant::now();
        let result = mock_system.simulate_discovery();
        let elapsed = start_time.elapsed();
        
        assert!(result.is_ok());
        assert!(elapsed.as_millis() >= 50);
    }

    #[test]
    fn test_comprehensive_network_failure_recovery() {
        let store = Store::new();
        let topology = create_mock_topology();
        let system = Arc::new(System::new().expect("Failed to create system"));
        
        store.dispatch(AppAction::SetTopology(topology));
        store.dispatch(AppAction::SetSystem(system));
        
        let mut control_view = ControlView::new(&store);
        
        // Test multiple failure and recovery cycles
        for cycle in 0..3 {
            // Set selection
            store.dispatch(AppAction::SetSelectedSpeakerUuid("RINCON_MOCK_LIVING_001".to_string()));
            
            // Try commands during "network failure" (will fail due to mock system)
            let space_key = KeyEvent::from(KeyCode::Char(' '));
            let result = control_view.handle_input(space_key, &store);
            assert!(result.is_ok(), "Cycle {} pause should be handled", cycle);
            
            let enter_key = KeyEvent::from(KeyCode::Enter);
            let result = control_view.handle_input(enter_key, &store);
            assert!(result.is_ok(), "Cycle {} play should be handled", cycle);
            
            // Verify error messages were set
            store.with_state(|state| {
                assert!(state.status_message.contains("failed"));
            });
            
            // Simulate network recovery by updating status
            store.dispatch(AppAction::SetStatusMessage(format!("Recovery cycle {}", cycle)));
        }
        
        // Verify system remains stable after multiple failure cycles
        store.with_state(|state| {
            assert!(state.topology.is_some());
            assert!(state.system.is_some());
            assert!(state.status_message.contains("Recovery cycle"));
        });
    }

    #[test]
    fn test_mock_speaker_command_history_tracking() {
        let speaker = MockSpeaker::new("History Test", "RINCON_HISTORY_001", "192.168.1.200");
        
        // Perform various operations
        let _ = speaker.play();
        let _ = speaker.pause();
        let _ = speaker.set_volume(60);
        let _ = speaker.adjust_volume(10);
        let _ = speaker.get_volume();
        
        // Check command history
        let history = speaker.get_command_history();
        assert_eq!(history.len(), 5);
        assert_eq!(history[0], "play");
        assert_eq!(history[1], "pause");
        assert_eq!(history[2], "set_volume(60)");
        assert_eq!(history[3], "adjust_volume(10)");
        assert_eq!(history[4], "get_volume");
        
        // Test history clearing
        speaker.clear_command_history();
        assert!(speaker.get_command_history().is_empty());
        
        // Test that failed commands don't get recorded
        speaker.set_config(MockSpeakerConfig {
            simulate_unreachable: true,
            ..Default::default()
        });
        
        let _ = speaker.play(); // This should fail
        assert!(speaker.get_command_history().is_empty());
    }

    #[test]
    fn test_edge_case_network_scenarios() {
        // Test speaker with zero delay
        let zero_delay_speaker = MockSpeakerBuilder::new()
            .name("Zero Delay")
            .uuid("RINCON_ZERO_DELAY")
            .with_delay(0)
            .build();
        
        let start = std::time::Instant::now();
        let result = zero_delay_speaker.play();
        let elapsed = start.elapsed();
        
        assert!(result.is_ok());
        assert!(elapsed.as_millis() < 10); // Should be very fast
        
        // Test speaker with very long delay (but still reasonable for testing)
        let long_delay_speaker = MockSpeakerBuilder::new()
            .name("Long Delay")
            .uuid("RINCON_LONG_DELAY")
            .with_delay(200)
            .build();
        
        let start = std::time::Instant::now();
        let result = long_delay_speaker.pause();
        let elapsed = start.elapsed();
        
        assert!(result.is_ok());
        assert!(elapsed.as_millis() >= 200);
        
        // Test speaker that fails commands but has delay
        let failing_delayed_speaker = MockSpeakerBuilder::new()
            .name("Failing Delayed")
            .uuid("RINCON_FAILING_DELAYED")
            .with_delay(50)
            .failing_commands()
            .build();
        
        let start = std::time::Instant::now();
        let result = failing_delayed_speaker.play();
        let elapsed = start.elapsed();
        
        assert!(result.is_err());
        assert!(elapsed.as_millis() >= 50); // Should still have delay even on failure
    }
}