//! Integration tests for ControlView with refactored ListItem navigation
//!
//! These tests verify that the ControlView works correctly with the refactored
//! SpeakerList interface using ListItem objects instead of string formatting.

use std::sync::Arc;
use cli::state::store::{Store, AppState};
use cli::state::reducers::AppAction;
use cli::views::control::ControlView;
use cli::topology::{topology_item::TopologyItem, topology_list::TopologyList};
use crossterm::event::{KeyCode, KeyEvent};

/// Helper function to create a test topology with various item types
fn create_test_topology() -> TopologyList {
    TopologyList {
        items: vec![
            TopologyItem::Group {
                ip: "192.168.1.100".to_string(),
                name: "Living Room".to_string(),
                uuid: "RINCON_LIVING_ROOM_001".to_string(),
                is_last: false,
            },
            TopologyItem::Speaker {
                ip: "192.168.1.101".to_string(),
                name: "Kitchen".to_string(),
                uuid: "RINCON_KITCHEN_001".to_string(),
                is_last: false,
            },
            TopologyItem::Speaker {
                ip: "192.168.1.102".to_string(),
                name: "Bedroom".to_string(),
                uuid: "RINCON_BEDROOM_001".to_string(),
                is_last: false,
            },
            TopologyItem::Satellite {
                uuid: "RINCON_SAT_001".to_string(),
                is_last: true,
            },
        ],
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_control_view_creation_with_topology() {
        // Create store and set up topology
        let store = Arc::new(Store::new());
        let topology = create_test_topology();
        
        store.dispatch(AppAction::UpdateTopology(topology.clone()));
        
        // Create control view - should use refactored SpeakerList with ListItem
        let control_view = ControlView::new(store.clone());
        
        // Verify the control view was created successfully
        // This indirectly tests that SpeakerList::new works with ListItem objects
        store.with_state(|state| {
            assert!(state.topology.is_some());
            assert_eq!(state.topology.as_ref().unwrap().items.len(), 4);
        });
    }

    #[test]
    fn test_control_view_creation_with_empty_topology() {
        // Create store with empty topology
        let store = Arc::new(Store::new());
        let empty_topology = TopologyList { items: vec![] };
        
        store.dispatch(AppAction::UpdateTopology(empty_topology));
        
        // Create control view - should handle empty topology gracefully
        let control_view = ControlView::new(store.clone());
        
        // Verify the control view handles empty topology
        store.with_state(|state| {
            assert!(state.topology.is_some());
            assert_eq!(state.topology.as_ref().unwrap().items.len(), 0);
        });
    }

    #[test]
    fn test_control_view_navigation_with_list_items() {
        // Create store and set up topology
        let store = Arc::new(Store::new());
        let topology = create_test_topology();
        
        store.dispatch(AppAction::UpdateTopology(topology.clone()));
        
        // Create control view
        let mut control_view = ControlView::new(store.clone());
        
        // Test navigation down - should work with ListItem objects
        let down_key = KeyEvent::from(KeyCode::Down);
        let result = control_view.handle_input(down_key, &store);
        assert!(result.is_ok(), "Down navigation should work with ListItem");
        
        // Verify highlight was updated
        store.with_state(|state| {
            assert!(state.highlight.is_some());
            // Should be the Kitchen speaker (second item)
            match state.highlight.as_ref().unwrap() {
                TopologyItem::Speaker { name, .. } => {
                    assert_eq!(name, "Kitchen");
                }
                _ => panic!("Expected speaker item"),
            }
        });
        
        // Test navigation up
        let up_key = KeyEvent::from(KeyCode::Up);
        let result = control_view.handle_input(up_key, &store);
        assert!(result.is_ok(), "Up navigation should work with ListItem");
        
        // Verify highlight was updated back to first item
        store.with_state(|state| {
            assert!(state.highlight.is_some());
            // Should be the Living Room group (first item)
            match state.highlight.as_ref().unwrap() {
                TopologyItem::Group { name, .. } => {
                    assert_eq!(name, "Living Room");
                }
                _ => panic!("Expected group item"),
            }
        });
    }

    #[test]
    fn test_control_view_selection_functionality() {
        // Create store and set up topology
        let store = Arc::new(Store::new());
        let topology = create_test_topology();
        
        store.dispatch(AppAction::UpdateTopology(topology.clone()));
        
        // Create control view
        let mut control_view = ControlView::new(store.clone());
        
        // Navigate to a speaker
        let down_key = KeyEvent::from(KeyCode::Down);
        let result = control_view.handle_input(down_key, &store);
        assert!(result.is_ok());
        
        // Select the speaker with space key
        let space_key = KeyEvent::from(KeyCode::Char(' '));
        let result = control_view.handle_input(space_key, &store);
        assert!(result.is_ok(), "Speaker selection should work");
        
        // Verify speaker was selected
        store.with_state(|state| {
            assert!(state.selected_speaker_ip.is_some());
            assert_eq!(state.selected_speaker_ip.as_ref().unwrap(), "192.168.1.101");
        });
        
        // Select again to toggle off
        let space_key = KeyEvent::from(KeyCode::Char(' '));
        let result = control_view.handle_input(space_key, &store);
        assert!(result.is_ok(), "Speaker deselection should work");
        
        // Verify speaker was deselected
        store.with_state(|state| {
            assert!(state.selected_speaker_ip.is_none());
        });
    }

    #[test]
    fn test_control_view_volume_controls() {
        // Create store and set up topology
        let store = Arc::new(Store::new());
        let topology = create_test_topology();
        
        store.dispatch(AppAction::UpdateTopology(topology.clone()));
        
        // Create control view
        let mut control_view = ControlView::new(store.clone());
        
        // Navigate to a speaker
        let down_key = KeyEvent::from(KeyCode::Down);
        let result = control_view.handle_input(down_key, &store);
        assert!(result.is_ok());
        
        // Test volume down
        let left_key = KeyEvent::from(KeyCode::Left);
        let result = control_view.handle_input(left_key, &store);
        assert!(result.is_ok(), "Volume down should be handled");
        
        // Test volume up
        let right_key = KeyEvent::from(KeyCode::Right);
        let result = control_view.handle_input(right_key, &store);
        assert!(result.is_ok(), "Volume up should be handled");
        
        // Verify state remains consistent
        store.with_state(|state| {
            assert!(state.topology.is_some());
            assert!(state.highlight.is_some());
        });
    }

    #[test]
    fn test_control_view_play_pause_controls() {
        // Create store and set up topology
        let store = Arc::new(Store::new());
        let topology = create_test_topology();
        
        store.dispatch(AppAction::UpdateTopology(topology.clone()));
        
        // Create control view
        let mut control_view = ControlView::new(store.clone());
        
        // Test play/pause command on group (first item)
        let p_key = KeyEvent::from(KeyCode::Char('p'));
        let result = control_view.handle_input(p_key, &store);
        assert!(result.is_ok(), "Play/pause should be handled for groups");
        
        // Navigate to speaker and test
        let down_key = KeyEvent::from(KeyCode::Down);
        let result = control_view.handle_input(down_key, &store);
        assert!(result.is_ok());
        
        let p_key = KeyEvent::from(KeyCode::Char('p'));
        let result = control_view.handle_input(p_key, &store);
        assert!(result.is_ok(), "Play/pause should be handled for speakers");
        
        // Verify state remains consistent
        store.with_state(|state| {
            assert!(state.topology.is_some());
            assert!(state.highlight.is_some());
        });
    }

    #[test]
    fn test_control_view_navigation_wraparound() {
        // Create store and set up topology
        let store = Arc::new(Store::new());
        let topology = create_test_topology();
        
        store.dispatch(AppAction::UpdateTopology(topology.clone()));
        
        // Create control view
        let mut control_view = ControlView::new(store.clone());
        
        // Navigate to the last item
        for _ in 0..4 {
            let down_key = KeyEvent::from(KeyCode::Down);
            let result = control_view.handle_input(down_key, &store);
            assert!(result.is_ok());
        }
        
        // Should wrap around to first item
        store.with_state(|state| {
            assert!(state.highlight.is_some());
            match state.highlight.as_ref().unwrap() {
                TopologyItem::Group { name, .. } => {
                    assert_eq!(name, "Living Room");
                }
                _ => panic!("Expected to wrap around to first group item"),
            }
        });
        
        // Test upward wraparound
        let up_key = KeyEvent::from(KeyCode::Up);
        let result = control_view.handle_input(up_key, &store);
        assert!(result.is_ok());
        
        // Should wrap to last item (satellite)
        store.with_state(|state| {
            assert!(state.highlight.is_some());
            match state.highlight.as_ref().unwrap() {
                TopologyItem::Satellite { .. } => {
                    // Expected satellite item
                }
                _ => panic!("Expected to wrap around to last satellite item"),
            }
        });
    }

    #[test]
    fn test_control_view_handles_all_topology_item_types() {
        // Create store and set up topology
        let store = Arc::new(Store::new());
        let topology = create_test_topology();
        
        store.dispatch(AppAction::UpdateTopology(topology.clone()));
        
        // Create control view
        let mut control_view = ControlView::new(store.clone());
        
        // Navigate through all items and verify each type is handled
        let expected_types = vec!["Group", "Speaker", "Speaker", "Satellite"];
        
        for (i, expected_type) in expected_types.iter().enumerate() {
            // Verify current item type
            store.with_state(|state| {
                assert!(state.highlight.is_some());
                let actual_type = match state.highlight.as_ref().unwrap() {
                    TopologyItem::Group { .. } => "Group",
                    TopologyItem::Speaker { .. } => "Speaker",
                    TopologyItem::Satellite { .. } => "Satellite",
                };
                assert_eq!(actual_type, *expected_type, "Item {} should be {}", i, expected_type);
            });
            
            // Test that selection works for each type
            let space_key = KeyEvent::from(KeyCode::Char(' '));
            let result = control_view.handle_input(space_key, &store);
            assert!(result.is_ok(), "Selection should work for {} items", expected_type);
            
            // Navigate to next item (except for last)
            if i < expected_types.len() - 1 {
                let down_key = KeyEvent::from(KeyCode::Down);
                let result = control_view.handle_input(down_key, &store);
                assert!(result.is_ok());
            }
        }
    }

    #[test]
    fn test_control_view_multiple_operation_cycles() {
        // Create store and set up topology
        let store = Arc::new(Store::new());
        let topology = create_test_topology();
        
        store.dispatch(AppAction::UpdateTopology(topology.clone()));
        
        // Create control view
        let mut control_view = ControlView::new(store.clone());
        
        // Perform multiple cycles of operations
        for cycle in 0..3 {
            // Navigate through all items
            for _ in 0..4 {
                let down_key = KeyEvent::from(KeyCode::Down);
                let result = control_view.handle_input(down_key, &store);
                assert!(result.is_ok(), "Navigation cycle {} should work", cycle);
            }
            
            // Test various commands
            let commands = vec![
                KeyCode::Char(' '), // selection
                KeyCode::Left,      // volume down
                KeyCode::Right,     // volume up
                KeyCode::Char('p'), // play/pause
            ];
            
            for command in commands {
                let key_event = KeyEvent::from(command);
                let result = control_view.handle_input(key_event, &store);
                assert!(result.is_ok(), "Command {:?} in cycle {} should work", command, cycle);
            }
            
            // Verify state consistency
            store.with_state(|state| {
                assert!(state.topology.is_some());
                assert_eq!(state.topology.as_ref().unwrap().items.len(), 4);
            });
        }
    }

    #[test]
    fn test_control_view_state_consistency_after_operations() {
        // Create store and set up topology
        let store = Arc::new(Store::new());
        let topology = create_test_topology();
        
        store.dispatch(AppAction::UpdateTopology(topology.clone()));
        
        // Create control view
        let mut control_view = ControlView::new(store.clone());
        
        // Perform a series of operations
        let operations = vec![
            KeyCode::Down,      // navigate
            KeyCode::Char(' '), // select
            KeyCode::Down,      // navigate
            KeyCode::Left,      // volume down
            KeyCode::Up,        // navigate
            KeyCode::Right,     // volume up
            KeyCode::Down,      // navigate
            KeyCode::Char('p'), // play/pause
            KeyCode::Char(' '), // select
        ];
        
        for (i, operation) in operations.iter().enumerate() {
            let key_event = KeyEvent::from(*operation);
            let result = control_view.handle_input(key_event, &store);
            assert!(result.is_ok(), "Operation {} should succeed", i);
            
            // Verify state consistency after each operation
            store.with_state(|state| {
                assert!(state.topology.is_some(), "Topology should remain available after operation {}", i);
                assert_eq!(state.topology.as_ref().unwrap().items.len(), 4, "Topology should have 4 items after operation {}", i);
                
                // Highlight should always be set when topology is available
                assert!(state.highlight.is_some(), "Highlight should be set after operation {}", i);
            });
        }
    }

    #[test]
    fn test_control_view_rendering_integration() {
        use ratatui::{backend::TestBackend, Terminal};
        
        // Create store and set up topology
        let store = Arc::new(Store::new());
        let topology = create_test_topology();
        
        store.dispatch(AppAction::UpdateTopology(topology.clone()));
        
        // Create control view
        let mut control_view = ControlView::new(store.clone());
        
        // Test that rendering works with ListItem objects
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        
        let render_result = terminal.draw(|frame| {
            control_view.render(frame);
        });
        
        assert!(render_result.is_ok(), "Rendering should work with ListItem objects");
        
        // Verify state is still consistent after rendering
        store.with_state(|state| {
            assert!(state.topology.is_some());
            assert_eq!(state.topology.as_ref().unwrap().items.len(), 4);
        });
    }

    #[test]
    fn test_control_view_empty_topology_navigation() {
        // Create store with empty topology
        let store = Arc::new(Store::new());
        let empty_topology = TopologyList { items: vec![] };
        
        store.dispatch(AppAction::UpdateTopology(empty_topology));
        
        // Create control view
        let mut control_view = ControlView::new(store.clone());
        
        // Test navigation with empty topology - should not crash
        let down_key = KeyEvent::from(KeyCode::Down);
        let result = control_view.handle_input(down_key, &store);
        assert!(result.is_ok(), "Navigation should handle empty topology gracefully");
        
        let up_key = KeyEvent::from(KeyCode::Up);
        let result = control_view.handle_input(up_key, &store);
        assert!(result.is_ok(), "Navigation should handle empty topology gracefully");
        
        // Test commands with empty topology
        let space_key = KeyEvent::from(KeyCode::Char(' '));
        let result = control_view.handle_input(space_key, &store);
        assert!(result.is_ok(), "Commands should handle empty topology gracefully");
        
        // Verify state consistency
        store.with_state(|state| {
            assert!(state.topology.is_some());
            assert_eq!(state.topology.as_ref().unwrap().items.len(), 0);
            assert!(state.highlight.is_none());
            assert!(state.selected_speaker_ip.is_none());
        });
    }

    #[test]
    fn test_control_view_single_item_navigation() {
        // Create store with single item topology
        let store = Arc::new(Store::new());
        let single_item_topology = TopologyList {
            items: vec![
                TopologyItem::Speaker {
                    ip: "192.168.1.100".to_string(),
                    name: "Only Speaker".to_string(),
                    uuid: "RINCON_ONLY_001".to_string(),
                    is_last: true,
                },
            ],
        };
        
        store.dispatch(AppAction::UpdateTopology(single_item_topology));
        
        // Create control view
        let mut control_view = ControlView::new(store.clone());
        
        // Test navigation with single item - should stay on same item
        let down_key = KeyEvent::from(KeyCode::Down);
        let result = control_view.handle_input(down_key, &store);
        assert!(result.is_ok());
        
        // Verify still on the same item
        store.with_state(|state| {
            assert!(state.highlight.is_some());
            match state.highlight.as_ref().unwrap() {
                TopologyItem::Speaker { name, .. } => {
                    assert_eq!(name, "Only Speaker");
                }
                _ => panic!("Expected the only speaker item"),
            }
        });
        
        // Test up navigation
        let up_key = KeyEvent::from(KeyCode::Up);
        let result = control_view.handle_input(up_key, &store);
        assert!(result.is_ok());
        
        // Should still be on the same item
        store.with_state(|state| {
            assert!(state.highlight.is_some());
            match state.highlight.as_ref().unwrap() {
                TopologyItem::Speaker { name, .. } => {
                    assert_eq!(name, "Only Speaker");
                }
                _ => panic!("Expected the only speaker item"),
            }
        });
    }
}