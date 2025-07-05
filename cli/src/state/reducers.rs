use sonos::Speaker;

use super::store::AppState;
use crate::types::{Topology, View};

#[derive(Debug)]
pub enum AppAction {
  AddSpeaker(Speaker),
  AdjustVolume(i8),
  SetSelectedSpeaker(usize),
  SetStatusMessage(String),
  SetTopology(Topology),
}

pub fn app_reducer(state: &mut AppState, action: AppAction) {
  match action {
    AppAction::AddSpeaker(_speaker) => {

    },
    AppAction::AdjustVolume(_adjustment) => {
      
    },
    AppAction::SetSelectedSpeaker(_index) => {
      
    },
    AppAction::SetStatusMessage(message) => {
      state.status_message = message;
    },
    AppAction::SetTopology(topology) => {
      log::debug!("SetTopology action received, switching to Control view");
      state.topology = Some(topology);
      state.view = View::Control;
    }
  }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{View};

    fn create_test_topology() -> Topology {
        Topology {
            groups: vec![
                Group {
                    name: "Living Room".to_string(),
                    speakers: vec!["Living Room".to_string()],
                },
                Group {
                    name: "Kitchen".to_string(),
                    speakers: vec!["Kitchen".to_string(), "Dining Room".to_string()],
                },
            ],
        }
    }

    fn create_updated_topology() -> Topology {
        Topology {
            groups: vec![
                Group {
                    name: "Bedroom".to_string(),
                    speakers: vec!["Bedroom".to_string()],
                },
                Group {
                    name: "Office".to_string(),
                    speakers: vec!["Office".to_string()],
                },
                Group {
                    name: "Bathroom".to_string(),
                    speakers: vec!["Bathroom".to_string()],
                },
            ],
        }
    }

    #[test]
    fn test_set_topology_action_updates_state_correctly() {
        let mut state = AppState::default();
        let topology = create_test_topology();
        
        // Verify initial state has no topology
        assert!(state.topology.is_none());
        
        // Dispatch SetTopology action
        app_reducer(&mut state, AppAction::SetTopology(topology.clone()));
        
        // Verify topology is now set in state
        assert!(state.topology.is_some());
        let stored_topology = state.topology.as_ref().unwrap();
        
        // Verify topology data is correct
        assert_eq!(stored_topology.groups.len(), 2);
        assert_eq!(stored_topology.groups[0].name, "Living Room");
        assert_eq!(stored_topology.groups[0].speakers.len(), 1);
        assert_eq!(stored_topology.groups[0].speakers[0], "Living Room");
        
        assert_eq!(stored_topology.groups[1].name, "Kitchen");
        assert_eq!(stored_topology.groups[1].speakers.len(), 2);
        assert_eq!(stored_topology.groups[1].speakers[0], "Kitchen");
        assert_eq!(stored_topology.groups[1].speakers[1], "Dining Room");
    }

    #[test]
    fn test_multiple_topology_updates_replace_previous_data() {
        let mut state = AppState::default();
        let first_topology = create_test_topology();
        let second_topology = create_updated_topology();
        
        // Set initial topology
        app_reducer(&mut state, AppAction::SetTopology(first_topology));
        
        // Verify first topology is set
        assert!(state.topology.is_some());
        let stored_topology = state.topology.as_ref().unwrap();
        assert_eq!(stored_topology.groups.len(), 2);
        assert_eq!(stored_topology.groups[0].name, "Living Room");
        assert_eq!(stored_topology.groups[1].name, "Kitchen");
        
        // Update with second topology
        app_reducer(&mut state, AppAction::SetTopology(second_topology));
        
        // Verify second topology replaced the first
        assert!(state.topology.is_some());
        let updated_topology = state.topology.as_ref().unwrap();
        assert_eq!(updated_topology.groups.len(), 3);
        assert_eq!(updated_topology.groups[0].name, "Bedroom");
        assert_eq!(updated_topology.groups[1].name, "Office");
        assert_eq!(updated_topology.groups[2].name, "Bathroom");
        
        // Verify old topology data is completely replaced
        assert!(!updated_topology.groups.iter().any(|g| g.name == "Living Room"));
        assert!(!updated_topology.groups.iter().any(|g| g.name == "Kitchen"));
    }

    #[test]
    fn test_initial_state_handling_with_no_topology() {
        let state = AppState::default();
        
        // Verify initial state has no topology
        assert!(state.topology.is_none());
        
        // Verify other state fields are properly initialized
        assert!(matches!(state.view, View::Startup));
        assert_eq!(state.status_message, "loading...");
    }

    #[test]
    fn test_set_topology_preserves_other_state_fields() {
        let mut state = AppState {
            view: View::Control,
            status_message: "Custom message".to_string(),
            topology: None,
        };
        
        let topology = create_test_topology();
        
        // Dispatch SetTopology action
        app_reducer(&mut state, AppAction::SetTopology(topology));
        
        // Verify topology is set
        assert!(state.topology.is_some());
        
        // Verify other state fields are preserved
        assert!(matches!(state.view, View::Control));
        assert_eq!(state.status_message, "Custom message");
    }

    #[test]
    fn test_set_topology_with_empty_groups() {
        let mut state = AppState::default();
        let empty_topology = Topology {
            groups: vec![],
        };
        
        // Dispatch SetTopology action with empty topology
        app_reducer(&mut state, AppAction::SetTopology(empty_topology));
        
        // Verify topology is set but empty
        assert!(state.topology.is_some());
        let stored_topology = state.topology.as_ref().unwrap();
        assert_eq!(stored_topology.groups.len(), 0);
    }

    #[test]
    fn test_set_topology_with_single_group() {
        let mut state = AppState::default();
        let single_group_topology = Topology {
            groups: vec![
                Group {
                    name: "Solo Speaker".to_string(),
                    speakers: vec!["Solo Speaker".to_string()],
                },
            ],
        };
        
        // Dispatch SetTopology action
        app_reducer(&mut state, AppAction::SetTopology(single_group_topology));
        
        // Verify topology is set correctly
        assert!(state.topology.is_some());
        let stored_topology = state.topology.as_ref().unwrap();
        assert_eq!(stored_topology.groups.len(), 1);
        assert_eq!(stored_topology.groups[0].name, "Solo Speaker");
        assert_eq!(stored_topology.groups[0].speakers.len(), 1);
        assert_eq!(stored_topology.groups[0].speakers[0], "Solo Speaker");
    }

    #[test]
    fn test_set_topology_action_is_debug_printable() {
        let topology = create_test_topology();
        let action = AppAction::SetTopology(topology);
        
        // This should not panic - verifies Debug trait is implemented
        let debug_string = format!("{:?}", action);
        assert!(debug_string.contains("SetTopology"));
    }
}
