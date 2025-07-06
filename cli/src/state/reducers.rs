use std::sync::Arc;
use sonos::{Speaker, System};

use super::store::AppState;
use crate::types::{Topology, View};

pub enum AppAction {
  AddSpeaker(Speaker),
  AdjustVolume(i8),
  SetSelectedSpeaker(usize),
  SetStatusMessage(String),
  SetTopology(Topology),
  SetSystem(Arc<System>),
  SetSelectedSpeakerUuid(String),
  SetSelectedGroupUuid(String),
  ClearSelection,
}

impl std::fmt::Debug for AppAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppAction::AddSpeaker(speaker) => f.debug_tuple("AddSpeaker").field(speaker).finish(),
            AppAction::AdjustVolume(adjustment) => f.debug_tuple("AdjustVolume").field(adjustment).finish(),
            AppAction::SetSelectedSpeaker(index) => f.debug_tuple("SetSelectedSpeaker").field(index).finish(),
            AppAction::SetStatusMessage(message) => f.debug_tuple("SetStatusMessage").field(message).finish(),
            AppAction::SetTopology(topology) => f.debug_tuple("SetTopology").field(topology).finish(),
            AppAction::SetSystem(_) => f.debug_tuple("SetSystem").field(&"Arc<System>").finish(),
            AppAction::SetSelectedSpeakerUuid(uuid) => f.debug_tuple("SetSelectedSpeakerUuid").field(uuid).finish(),
            AppAction::SetSelectedGroupUuid(uuid) => f.debug_tuple("SetSelectedGroupUuid").field(uuid).finish(),
            AppAction::ClearSelection => f.debug_tuple("ClearSelection").finish(),
        }
    }
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
    },
    AppAction::SetSystem(system) => {
      log::debug!("SetSystem action received");
      state.system = Some(system);
    },
    AppAction::SetSelectedSpeakerUuid(uuid) => {
      log::debug!("SetSelectedSpeakerUuid action received: {}", uuid);
      state.selected_speaker_uuid = Some(uuid);
    },
    AppAction::SetSelectedGroupUuid(uuid) => {
      log::debug!("SetSelectedGroupUuid action received: {}", uuid);
      state.selected_group_uuid = Some(uuid);
    },
    AppAction::ClearSelection => {
      log::debug!("ClearSelection action received");
      state.selected_speaker_uuid = None;
      state.selected_group_uuid = None;
    }
  }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{View, Group, SpeakerInfo};

    fn create_test_topology() -> Topology {
        Topology {
            groups: vec![
                Group {
                    name: "Living Room".to_string(),
                    speakers: vec![SpeakerInfo::from_name("Living Room", true)],
                },
                Group {
                    name: "Kitchen".to_string(),
                    speakers: vec![
                        SpeakerInfo::from_name("Kitchen", true),
                        SpeakerInfo::from_name("Dining Room", false),
                    ],
                },
            ],
        }
    }

    fn create_updated_topology() -> Topology {
        Topology {
            groups: vec![
                Group {
                    name: "Bedroom".to_string(),
                    speakers: vec![SpeakerInfo::from_name("Bedroom", true)],
                },
                Group {
                    name: "Office".to_string(),
                    speakers: vec![SpeakerInfo::from_name("Office", true)],
                },
                Group {
                    name: "Bathroom".to_string(),
                    speakers: vec![SpeakerInfo::from_name("Bathroom", true)],
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
            system: None,
            selected_speaker_uuid: None,
            selected_group_uuid: None,
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
                    speakers: vec![SpeakerInfo::from_name("Solo Speaker", true)],
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

    #[test]
    fn test_set_system_action_updates_state_correctly() {
        let mut state = AppState::default();
        let system = Arc::new(System::new().unwrap());
        
        // Verify initial state has no system
        assert!(state.system.is_none());
        
        // Dispatch SetSystem action
        app_reducer(&mut state, AppAction::SetSystem(system.clone()));
        
        // Verify system is now set in state
        assert!(state.system.is_some());
        
        // Verify it's the same Arc reference
        let stored_system = state.system.as_ref().unwrap();
        assert!(Arc::ptr_eq(&system, stored_system));
    }

    #[test]
    fn test_multiple_system_updates_replace_previous_reference() {
        let mut state = AppState::default();
        let first_system = Arc::new(System::new().unwrap());
        let second_system = Arc::new(System::new().unwrap());
        
        // Set initial system
        app_reducer(&mut state, AppAction::SetSystem(first_system.clone()));
        
        // Verify first system is set
        assert!(state.system.is_some());
        let stored_system = state.system.as_ref().unwrap();
        assert!(Arc::ptr_eq(&first_system, stored_system));
        
        // Update with second system
        app_reducer(&mut state, AppAction::SetSystem(second_system.clone()));
        
        // Verify second system replaced the first
        assert!(state.system.is_some());
        let updated_system = state.system.as_ref().unwrap();
        assert!(Arc::ptr_eq(&second_system, updated_system));
        assert!(!Arc::ptr_eq(&first_system, updated_system));
    }

    #[test]
    fn test_set_system_preserves_other_state_fields() {
        let mut state = AppState {
            view: View::Control,
            status_message: "Custom message".to_string(),
            topology: Some(create_test_topology()),
            system: None,
            selected_speaker_uuid: None,
            selected_group_uuid: None,
        };
        
        let system = Arc::new(System::new().unwrap());
        
        // Dispatch SetSystem action
        app_reducer(&mut state, AppAction::SetSystem(system.clone()));
        
        // Verify system is set
        assert!(state.system.is_some());
        let stored_system = state.system.as_ref().unwrap();
        assert!(Arc::ptr_eq(&system, stored_system));
        
        // Verify other state fields are preserved
        assert!(matches!(state.view, View::Control));
        assert_eq!(state.status_message, "Custom message");
        assert!(state.topology.is_some());
    }

    #[test]
    fn test_initial_state_has_no_system_reference() {
        let state = AppState::default();
        
        // Verify initial state has no system
        assert!(state.system.is_none());
        
        // Verify other state fields are properly initialized
        assert!(matches!(state.view, View::Startup));
        assert_eq!(state.status_message, "loading...");
        assert!(state.topology.is_none());
    }

    #[test]
    fn test_set_system_action_is_debug_printable() {
        let system = Arc::new(System::new().unwrap());
        let action = AppAction::SetSystem(system);
        
        // This should not panic - verifies Debug trait is implemented
        let debug_string = format!("{:?}", action);
        assert!(debug_string.contains("SetSystem"));
    }

    #[test]
    fn test_system_reference_sharing_with_arc() {
        let mut state = AppState::default();
        let system = Arc::new(System::new().unwrap());
        
        // Create multiple references to the same system
        let system_ref1 = system.clone();
        let system_ref2 = system.clone();
        
        // Set system in state
        app_reducer(&mut state, AppAction::SetSystem(system_ref1));
        
        // Verify all references point to the same system
        let stored_system = state.system.as_ref().unwrap();
        assert!(Arc::ptr_eq(&system, stored_system));
        assert!(Arc::ptr_eq(&system_ref2, stored_system));
        
        // Verify reference count is correct (original + state + ref2 = 3)
        assert_eq!(Arc::strong_count(stored_system), 3);
    }

    #[test]
    fn test_system_reference_cleanup_on_replacement() {
        let mut state = AppState::default();
        let first_system = Arc::new(System::new().unwrap());
        let second_system = Arc::new(System::new().unwrap());
        
        // Keep a reference to first system to test cleanup
        let first_system_ref = first_system.clone();
        
        // Set first system
        app_reducer(&mut state, AppAction::SetSystem(first_system));
        
        // Verify reference count (original ref + state = 2)
        assert_eq!(Arc::strong_count(&first_system_ref), 2);
        
        // Replace with second system
        app_reducer(&mut state, AppAction::SetSystem(second_system.clone()));
        
        // Verify first system reference count decreased (only original ref remains)
        assert_eq!(Arc::strong_count(&first_system_ref), 1);
        
        // Verify second system reference count (original + state = 2)
        assert_eq!(Arc::strong_count(&second_system), 2);
    }

    #[test]
    fn test_selection_state_initialization() {
        let state = AppState::default();
        
        // Verify initial selection state is None
        assert!(state.selected_speaker_uuid.is_none());
        assert!(state.selected_group_uuid.is_none());
    }

    #[test]
    fn test_set_selected_speaker_uuid_action() {
        let mut state = AppState::default();
        let speaker_uuid = "RINCON_TEST_SPEAKER_UUID".to_string();
        
        // Verify initial state has no selected speaker
        assert!(state.selected_speaker_uuid.is_none());
        
        // Dispatch SetSelectedSpeakerUuid action
        app_reducer(&mut state, AppAction::SetSelectedSpeakerUuid(speaker_uuid.clone()));
        
        // Verify speaker UUID is now set in state
        assert!(state.selected_speaker_uuid.is_some());
        assert_eq!(state.selected_speaker_uuid.unwrap(), speaker_uuid);
    }

    #[test]
    fn test_set_selected_group_uuid_action() {
        let mut state = AppState::default();
        let group_uuid = "RINCON_TEST_GROUP_UUID".to_string();
        
        // Verify initial state has no selected group
        assert!(state.selected_group_uuid.is_none());
        
        // Dispatch SetSelectedGroupUuid action
        app_reducer(&mut state, AppAction::SetSelectedGroupUuid(group_uuid.clone()));
        
        // Verify group UUID is now set in state
        assert!(state.selected_group_uuid.is_some());
        assert_eq!(state.selected_group_uuid.unwrap(), group_uuid);
    }

    #[test]
    fn test_clear_selection_action() {
        let mut state = AppState::default();
        let speaker_uuid = "RINCON_TEST_SPEAKER_UUID".to_string();
        let group_uuid = "RINCON_TEST_GROUP_UUID".to_string();
        
        // Set both speaker and group selections
        app_reducer(&mut state, AppAction::SetSelectedSpeakerUuid(speaker_uuid));
        app_reducer(&mut state, AppAction::SetSelectedGroupUuid(group_uuid));
        
        // Verify both selections are set
        assert!(state.selected_speaker_uuid.is_some());
        assert!(state.selected_group_uuid.is_some());
        
        // Dispatch ClearSelection action
        app_reducer(&mut state, AppAction::ClearSelection);
        
        // Verify both selections are cleared
        assert!(state.selected_speaker_uuid.is_none());
        assert!(state.selected_group_uuid.is_none());
    }

    #[test]
    fn test_selection_state_transitions() {
        let mut state = AppState::default();
        let first_speaker_uuid = "RINCON_FIRST_SPEAKER".to_string();
        let second_speaker_uuid = "RINCON_SECOND_SPEAKER".to_string();
        let group_uuid = "RINCON_TEST_GROUP".to_string();
        
        // Set first speaker selection
        app_reducer(&mut state, AppAction::SetSelectedSpeakerUuid(first_speaker_uuid.clone()));
        assert_eq!(state.selected_speaker_uuid.as_ref().unwrap(), &first_speaker_uuid);
        assert!(state.selected_group_uuid.is_none());
        
        // Change to second speaker selection
        app_reducer(&mut state, AppAction::SetSelectedSpeakerUuid(second_speaker_uuid.clone()));
        assert_eq!(state.selected_speaker_uuid.as_ref().unwrap(), &second_speaker_uuid);
        assert!(state.selected_group_uuid.is_none());
        
        // Add group selection (both can be set simultaneously)
        app_reducer(&mut state, AppAction::SetSelectedGroupUuid(group_uuid.clone()));
        assert_eq!(state.selected_speaker_uuid.as_ref().unwrap(), &second_speaker_uuid);
        assert_eq!(state.selected_group_uuid.as_ref().unwrap(), &group_uuid);
        
        // Clear all selections
        app_reducer(&mut state, AppAction::ClearSelection);
        assert!(state.selected_speaker_uuid.is_none());
        assert!(state.selected_group_uuid.is_none());
    }

    #[test]
    fn test_selection_actions_preserve_other_state_fields() {
        let mut state = AppState {
            view: View::Control,
            status_message: "Custom message".to_string(),
            topology: Some(create_test_topology()),
            system: Some(Arc::new(System::new().unwrap())),
            selected_speaker_uuid: None,
            selected_group_uuid: None,
        };
        
        let speaker_uuid = "RINCON_TEST_SPEAKER".to_string();
        let group_uuid = "RINCON_TEST_GROUP".to_string();
        
        // Set speaker selection
        app_reducer(&mut state, AppAction::SetSelectedSpeakerUuid(speaker_uuid.clone()));
        
        // Verify selection is set and other fields are preserved
        assert_eq!(state.selected_speaker_uuid.as_ref().unwrap(), &speaker_uuid);
        assert!(matches!(state.view, View::Control));
        assert_eq!(state.status_message, "Custom message");
        assert!(state.topology.is_some());
        assert!(state.system.is_some());
        
        // Set group selection
        app_reducer(&mut state, AppAction::SetSelectedGroupUuid(group_uuid.clone()));
        
        // Verify both selections are set and other fields are preserved
        assert_eq!(state.selected_speaker_uuid.as_ref().unwrap(), &speaker_uuid);
        assert_eq!(state.selected_group_uuid.as_ref().unwrap(), &group_uuid);
        assert!(matches!(state.view, View::Control));
        assert_eq!(state.status_message, "Custom message");
        assert!(state.topology.is_some());
        assert!(state.system.is_some());
        
        // Clear selections
        app_reducer(&mut state, AppAction::ClearSelection);
        
        // Verify selections are cleared and other fields are preserved
        assert!(state.selected_speaker_uuid.is_none());
        assert!(state.selected_group_uuid.is_none());
        assert!(matches!(state.view, View::Control));
        assert_eq!(state.status_message, "Custom message");
        assert!(state.topology.is_some());
        assert!(state.system.is_some());
    }

    #[test]
    fn test_selection_actions_are_debug_printable() {
        let speaker_uuid = "RINCON_TEST_SPEAKER".to_string();
        let group_uuid = "RINCON_TEST_GROUP".to_string();
        
        // Test SetSelectedSpeakerUuid action debug
        let speaker_action = AppAction::SetSelectedSpeakerUuid(speaker_uuid);
        let debug_string = format!("{:?}", speaker_action);
        assert!(debug_string.contains("SetSelectedSpeakerUuid"));
        
        // Test SetSelectedGroupUuid action debug
        let group_action = AppAction::SetSelectedGroupUuid(group_uuid);
        let debug_string = format!("{:?}", group_action);
        assert!(debug_string.contains("SetSelectedGroupUuid"));
        
        // Test ClearSelection action debug
        let clear_action = AppAction::ClearSelection;
        let debug_string = format!("{:?}", clear_action);
        assert!(debug_string.contains("ClearSelection"));
    }
}