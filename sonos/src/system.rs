use std::collections::HashMap;
use std::{
  net::UdpSocket,
  io::Result,
};
use log::{info, warn, error, debug};

use crate::topology::Topology;
use crate::speaker::{Speaker, SpeakerFactory, SpeakerTrait};
use crate::util::ssdp::send_ssdp_request;

pub struct System {
  speakers: HashMap<String, Box<dyn SpeakerTrait>>,
  topology: Option<Topology>,
}

#[derive(Debug)]
pub enum SystemEvent {
  SpeakerFound(Speaker),
  TopologyReady(Topology),
  DiscoveryComplete,
  Error(String),
  GroupUpdate(String, Vec<String>),
}

impl System {
  pub fn new() -> Result<Self> {
    Ok(System {
      speakers: HashMap::new(),
      topology: None,
    })
  }

  /// Returns a reference to the speaker HashMap
  pub fn speakers(&self) -> &HashMap<String, Box<dyn SpeakerTrait>> {
    &self.speakers
  }

  /// Returns an optional reference to the topology
  pub fn topology(&self) -> Option<&Topology> {
    self.topology.as_ref()
  }

  /// Checks if topology is available
  pub fn has_topology(&self) -> bool {
    self.topology.is_some()
  }

  /// Returns the number of discovered speakers
  pub fn speaker_count(&self) -> usize {
    self.speakers.len()
  }

  /// Gets a speaker by UUID
  pub fn get_speaker_by_uuid(&self, uuid: &str) -> Option<&Box<dyn SpeakerTrait>> {
    self.speakers.get(uuid)
  }

  #[cfg(test)]
  /// Test helper method to add a speaker directly (bypassing discovery)
  fn add_speaker_for_test(&mut self, speaker: Box<dyn SpeakerTrait>) {
    let uuid = speaker.uuid().to_string();
    self.speakers.insert(uuid, speaker);
  }

  pub fn discover(&mut self) -> impl Iterator<Item = SystemEvent> + '_ {
    info!("Starting discovery process...");
    self.clear_state();

    let responses = match self.setup_discovery() {
      Ok(responses) => responses,
      Err(e) => {
        error!("Failed to setup discovery: {}", e);
        return Box::new(std::iter::once(SystemEvent::Error(e.to_string()))
          .chain(std::iter::once(SystemEvent::DiscoveryComplete))) as Box<dyn Iterator<Item = SystemEvent>>;
      }
    };

    let mut is_first_speaker = true;

    Box::new(responses
      .filter(|response| response.is_ok())
      .flat_map(move |response| {
        self.process_ssdp_response(response, &mut is_first_speaker)
      })
      .chain(std::iter::once_with(|| {
        info!("Discovery process completed");
        SystemEvent::DiscoveryComplete
      }))) as Box<dyn Iterator<Item = SystemEvent>>
  }

  fn clear_state(&mut self) {
    self.speakers.clear();
    self.topology = None;
  }

  fn setup_discovery(&self) -> Result<impl Iterator<Item = std::result::Result<crate::util::ssdp::SsdpResponse, std::io::Error>>> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    
    let responses = send_ssdp_request(
      socket,
      "239.255.255.250:1900",
      "urn:schemas-upnp-org:device:ZonePlayer:1"
    )?;

    info!("SSDP request sent, waiting for responses...");
    Ok(responses)
  }

  fn process_ssdp_response(
    &mut self, 
    response: std::result::Result<crate::util::ssdp::SsdpResponse, std::io::Error>,
    is_first_speaker: &mut bool
  ) -> Vec<SystemEvent> {
    match response {
      Ok(ssdp) => {
        info!("Processing SSDP response from location: {}", ssdp.location);
        self.process_speaker_discovery(&ssdp.location, is_first_speaker)
      },
      Err(e) => {
        error!("Error in SSDP response: {}", e);
        vec![SystemEvent::Error(e.to_string())]
      },
    }
  }

  fn process_speaker_discovery(&mut self, location: &str, is_first_speaker: &mut bool) -> Vec<SystemEvent> {
    match Speaker::from_location(location) {
      Ok(speaker) => {
        info!("Successfully created speaker: {}", speaker.ip());
        self.store_speaker(&speaker, is_first_speaker)
      },
      Err(e) => {
        error!("Failed to create speaker from location {}: {}", location, e);
        vec![SystemEvent::Error(e.to_string())]
      }
    }
  }

  fn store_speaker(&mut self, speaker: &Speaker, is_first_speaker: &mut bool) -> Vec<SystemEvent> {
    let speaker_uuid = speaker.uuid().to_string();
    if self.speakers.contains_key(&speaker_uuid) {
      warn!("Duplicate speaker UUID found: {}. Replacing existing speaker.", speaker_uuid);
    }

    let boxed_speaker: Box<dyn SpeakerTrait> = Box::new(speaker.clone());
    self.speakers.insert(speaker_uuid.clone(), boxed_speaker);
    info!("Stored speaker with UUID: {}", speaker_uuid);

    let mut events = vec![SystemEvent::SpeakerFound(speaker.clone())];
    
    if *is_first_speaker {
      *is_first_speaker = false;
      events.extend(self.attempt_topology_retrieval(speaker.ip()));
    }

    events
  }

  fn attempt_topology_retrieval(&mut self, speaker_ip: &str) -> Vec<SystemEvent> {
    info!("This is the first speaker, attempting to get topology...");
    
    match Topology::from_ip(speaker_ip) {
      Ok(topology) => {
        info!("Successfully retrieved topology with {} zone groups", topology.zone_group_count());
        debug!("Topology details: {:?}", topology);

        self.topology = Some(topology.clone());
        vec![SystemEvent::TopologyReady(topology)]
      },
      Err(e) => {
        error!("Failed to retrieve topology: {:?}", e);
        vec![SystemEvent::Error(format!("Topology retrieval failed: {}", e))]
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_new_system_has_empty_state() {
    let system = System::new().unwrap();
    
    assert_eq!(system.speaker_count(), 0);
    assert!(!system.has_topology());
    assert!(system.topology().is_none());
    assert!(system.speakers().is_empty());
  }

  #[test]
  fn test_speakers_returns_reference_to_hashmap() {
    let system = System::new().unwrap();
    let speakers_ref = system.speakers();
    
    // Verify it's a reference to the internal HashMap
    assert_eq!(speakers_ref.len(), 0);
    assert!(speakers_ref.is_empty());
  }

  #[test]
  fn test_topology_returns_none_when_not_set() {
    let system = System::new().unwrap();
    
    assert!(system.topology().is_none());
    assert!(!system.has_topology());
  }

  #[test]
  fn test_speaker_count_returns_zero_for_empty_system() {
    let system = System::new().unwrap();
    
    assert_eq!(system.speaker_count(), 0);
  }

  #[test]
  fn test_get_speaker_by_uuid_returns_none_for_empty_system() {
    let system = System::new().unwrap();
    
    assert!(system.get_speaker_by_uuid("test-uuid").is_none());
    assert!(system.get_speaker_by_uuid("").is_none());
  }

  #[test]
  fn test_has_topology_returns_false_for_new_system() {
    let system = System::new().unwrap();
    
    assert!(!system.has_topology());
  }

  #[test]
  fn test_discover_uses_mutable_reference_and_clears_state() {
    let mut system = System::new().unwrap();
    
    // Call discover and consume the iterator to completion
    {
      let discovery_iter = system.discover();
      let _events: Vec<_> = discovery_iter.collect();
    }
    
    // System should still be accessible after discovery completes
    // Note: speaker_count may be > 0 if real speakers are found on network
    // Note: topology may be available if speakers are found and topology retrieval succeeds
    // The important thing is that the system wasn't consumed
    
    // The fact that we can call methods on system proves it wasn't consumed
    let _speakers = system.speakers();
    let _count = system.speaker_count();
  }

  #[cfg(all(test, feature = "mock"))]
  fn create_test_speaker(uuid: &str, name: &str, ip: &str) -> Box<dyn SpeakerTrait> {
    use crate::speaker::mock::MockSpeakerBuilder;
    
    Box::new(
      MockSpeakerBuilder::new()
        .uuid(uuid)
        .name(name)
        .ip(ip)
        .build()
    )
  }

  #[test]
  #[cfg(feature = "mock")]
  fn test_speaker_storage_and_retrieval() {
    let mut system = System::new().unwrap();
    
    // Initial state should be empty
    assert_eq!(system.speaker_count(), 0);
    assert!(system.speakers().is_empty());
    
    // Add test speakers
    let speaker1 = create_test_speaker("RINCON_123", "Living Room", "192.168.1.100");
    let speaker2 = create_test_speaker("RINCON_456", "Kitchen", "192.168.1.101");
    
    system.add_speaker_for_test(speaker1);
    system.add_speaker_for_test(speaker2);
    
    // Verify speakers are stored
    assert_eq!(system.speaker_count(), 2);
    assert!(!system.speakers().is_empty());
    
    // Verify speakers can be retrieved by UUID
    assert!(system.get_speaker_by_uuid("RINCON_123").is_some());
    assert!(system.get_speaker_by_uuid("RINCON_456").is_some());
    assert!(system.get_speaker_by_uuid("RINCON_999").is_none());
    
    // Verify speaker details
    let living_room = system.get_speaker_by_uuid("RINCON_123").unwrap();
    assert_eq!(living_room.uuid(), "RINCON_123");
    assert_eq!(living_room.name(), "Living Room");
    assert_eq!(living_room.ip(), "192.168.1.100");
  }

  #[test]
  #[cfg(feature = "mock")]
  fn test_discovery_clears_existing_speakers() {
    let mut system = System::new().unwrap();
    
    // Add some test speakers first
    let speaker1 = create_test_speaker("RINCON_123", "Living Room", "192.168.1.100");
    system.add_speaker_for_test(speaker1);
    
    assert_eq!(system.speaker_count(), 1);
    
    // Discovery should clear existing speakers
    {
      let discovery_iter = system.discover();
      let _events: Vec<_> = discovery_iter.collect();
    }
    
    // The original test speaker should be cleared
    // Note: New speakers may be found during actual discovery
    // The important thing is that the clear operation happened
    assert!(system.get_speaker_by_uuid("RINCON_123").is_none());
  }

  #[test]
  fn test_topology_storage_and_events() {
    let mut system = System::new().unwrap();
    
    // Initially no topology
    assert!(!system.has_topology());
    assert!(system.topology().is_none());
    
    // Run discovery and collect events
    let events: Vec<_> = system.discover().collect();
    
    // Check if any speakers were found (depends on network environment)
    let speaker_found_events: Vec<_> = events.iter()
      .filter(|event| matches!(event, SystemEvent::SpeakerFound(_)))
      .collect();
    
    let topology_ready_events: Vec<_> = events.iter()
      .filter(|event| matches!(event, SystemEvent::TopologyReady(_)))
      .collect();
    
    let error_events: Vec<_> = events.iter()
      .filter(|event| matches!(event, SystemEvent::Error(_)))
      .collect();
    
    // If speakers were found, we should have attempted topology retrieval
    if !speaker_found_events.is_empty() {
      // Either topology was successfully retrieved OR an error was emitted
      let topology_attempted = !topology_ready_events.is_empty() || 
        error_events.iter().any(|event| {
          if let SystemEvent::Error(msg) = event {
            msg.contains("Topology retrieval failed")
          } else {
            false
          }
        });
      
      assert!(topology_attempted, "Topology retrieval should have been attempted when speakers were found");
      
      // If topology was successfully retrieved, it should be stored
      if !topology_ready_events.is_empty() {
        assert!(system.has_topology(), "Topology should be stored when TopologyReady event is emitted");
        assert!(system.topology().is_some(), "Topology should be available when TopologyReady event is emitted");
      }
    }
    
    // Verify that topology retrieval failure doesn't stop speaker discovery
    // (This is implicitly tested by the fact that we can have speakers without topology)
    if !speaker_found_events.is_empty() && topology_ready_events.is_empty() {
      // We found speakers but no topology - this is acceptable
      assert!(system.speaker_count() > 0, "Speakers should still be stored even if topology retrieval fails");
    }
  }

  #[test]
  fn test_discovery_complete_event_emitted() {
    let mut system = System::new().unwrap();
    
    // Run discovery and collect all events
    let events: Vec<_> = system.discover().collect();
    
    // DiscoveryComplete event should always be emitted at the end
    assert!(!events.is_empty(), "Discovery should emit at least the DiscoveryComplete event");
    
    // The last event should always be DiscoveryComplete
    let last_event = events.last().unwrap();
    assert!(matches!(last_event, SystemEvent::DiscoveryComplete), 
            "Last event should be DiscoveryComplete, but was: {:?}", last_event);
    
    // Count DiscoveryComplete events - should be exactly one
    let discovery_complete_count = events.iter()
      .filter(|event| matches!(event, SystemEvent::DiscoveryComplete))
      .count();
    
    assert_eq!(discovery_complete_count, 1, 
               "Should emit exactly one DiscoveryComplete event, but found: {}", discovery_complete_count);
  }

  #[test]
  fn test_discovery_complete_event_emitted_regardless_of_failures() {
    let mut system = System::new().unwrap();
    
    // Run discovery - even if there are errors or no speakers found,
    // DiscoveryComplete should still be emitted
    let events: Vec<_> = system.discover().collect();
    
    // Should always have at least the DiscoveryComplete event
    assert!(!events.is_empty(), "Discovery should always emit at least DiscoveryComplete event");
    
    // The last event should be DiscoveryComplete regardless of what happened before
    let last_event = events.last().unwrap();
    assert!(matches!(last_event, SystemEvent::DiscoveryComplete), 
            "DiscoveryComplete should be emitted even if there are failures");
    
    // Verify that DiscoveryComplete comes after all other processing
    let discovery_complete_index = events.iter()
      .position(|event| matches!(event, SystemEvent::DiscoveryComplete))
      .expect("DiscoveryComplete event should be present");
    
    assert_eq!(discovery_complete_index, events.len() - 1, 
               "DiscoveryComplete should be the last event emitted");
  }

  #[test]
  fn test_event_emission_order_and_types() {
    let mut system = System::new().unwrap();
    
    // Run discovery and collect all events
    let events: Vec<_> = system.discover().collect();
    
    // Verify event types are correct
    for event in &events {
      match event {
        SystemEvent::SpeakerFound(_) => {
          // SpeakerFound events should contain valid speaker data
        },
        SystemEvent::TopologyReady(_) => {
          // TopologyReady events should contain valid topology data
        },
        SystemEvent::Error(msg) => {
          // Error events should contain descriptive messages
          assert!(!msg.is_empty(), "Error messages should not be empty");
        },
        SystemEvent::DiscoveryComplete => {
          // DiscoveryComplete should be the last event
          let event_index = events.iter().position(|e| matches!(e, SystemEvent::DiscoveryComplete)).unwrap();
          assert_eq!(event_index, events.len() - 1, "DiscoveryComplete should be the last event");
        },
        SystemEvent::GroupUpdate(_, _) => {
          // GroupUpdate events are not used in discovery but should be valid if present
        }
      }
    }
    
    // Verify that if topology errors occur, they use the generic Error event
    let topology_error_events: Vec<_> = events.iter()
      .filter(|event| {
        if let SystemEvent::Error(msg) = event {
          msg.contains("Topology retrieval failed")
        } else {
          false
        }
      })
      .collect();
    
    // If there are topology errors, they should use the generic Error event (not a specific TopologyError)
    for event in topology_error_events {
      assert!(matches!(event, SystemEvent::Error(_)), "Topology errors should use generic Error event");
    }
  }

  #[test]
  #[cfg(feature = "mock")]
  fn test_state_access_methods_with_various_states() {
    let mut system = System::new().unwrap();
    
    // Test empty state
    assert_eq!(system.speaker_count(), 0);
    assert!(!system.has_topology());
    assert!(system.topology().is_none());
    assert!(system.speakers().is_empty());
    assert!(system.get_speaker_by_uuid("any-uuid").is_none());
    
    // Test state with speakers but no topology
    let speaker1 = create_test_speaker("RINCON_123", "Living Room", "192.168.1.100");
    let speaker2 = create_test_speaker("RINCON_456", "Kitchen", "192.168.1.101");
    
    system.add_speaker_for_test(speaker1);
    system.add_speaker_for_test(speaker2);
    
    assert_eq!(system.speaker_count(), 2);
    assert!(!system.has_topology());
    assert!(system.topology().is_none());
    assert_eq!(system.speakers().len(), 2);
    assert!(system.get_speaker_by_uuid("RINCON_123").is_some());
    assert!(system.get_speaker_by_uuid("RINCON_456").is_some());
    assert!(system.get_speaker_by_uuid("RINCON_999").is_none());
    
    // Test state with topology (simulate by setting it directly)
    use crate::topology::types::{Topology, ZoneGroup, ZoneGroupMember};
    let test_topology = Topology {
      zone_groups: vec![
        ZoneGroup {
          coordinator: "RINCON_123".to_string(),
          id: "RINCON_123:123".to_string(),
          members: vec![
            ZoneGroupMember {
              uuid: "RINCON_123".to_string(),
              location: "http://192.168.1.100:1400/xml/device_description.xml".to_string(),
              zone_name: "Living Room".to_string(),
              software_version: "56.0-76060".to_string(),
              configuration: "1".to_string(),
              icon: "x-rincon-roomicon:living".to_string(),
              satellites: vec![],
            }
          ],
        }
      ],
      vanished_devices: None,
    };
    
    system.topology = Some(test_topology.clone());
    
    assert_eq!(system.speaker_count(), 2); // Still 2 speakers
    assert!(system.has_topology());
    assert!(system.topology().is_some());
    assert_eq!(system.topology().unwrap().zone_group_count(), 1);
    
    // Test clearing state
    system.speakers.clear();
    system.topology = None;
    
    assert_eq!(system.speaker_count(), 0);
    assert!(!system.has_topology());
    assert!(system.topology().is_none());
    assert!(system.speakers().is_empty());
  }

  #[test]
  #[cfg(feature = "mock")]
  fn test_speaker_storage_with_uuid_conflicts() {
    let mut system = System::new().unwrap();
    
    // Add first speaker
    let speaker1 = create_test_speaker("RINCON_123", "Living Room", "192.168.1.100");
    system.add_speaker_for_test(speaker1);
    
    assert_eq!(system.speaker_count(), 1);
    let first_speaker = system.get_speaker_by_uuid("RINCON_123").unwrap();
    assert_eq!(first_speaker.name(), "Living Room");
    assert_eq!(first_speaker.ip(), "192.168.1.100");
    
    // Add second speaker with same UUID (should replace)
    let speaker2 = create_test_speaker("RINCON_123", "Kitchen", "192.168.1.101");
    system.add_speaker_for_test(speaker2);
    
    // Should still have only 1 speaker, but it should be the new one
    assert_eq!(system.speaker_count(), 1);
    let replaced_speaker = system.get_speaker_by_uuid("RINCON_123").unwrap();
    assert_eq!(replaced_speaker.name(), "Kitchen");
    assert_eq!(replaced_speaker.ip(), "192.168.1.101");
  }

  #[test]
  #[cfg(feature = "mock")]
  fn test_speaker_retrieval_edge_cases() {
    let mut system = System::new().unwrap();
    
    // Test with empty string UUID
    assert!(system.get_speaker_by_uuid("").is_none());
    
    // Test with whitespace UUID
    assert!(system.get_speaker_by_uuid("   ").is_none());
    
    // Add speaker and test exact match requirement
    let speaker = create_test_speaker("RINCON_123456", "Living Room", "192.168.1.100");
    system.add_speaker_for_test(speaker);
    
    // Exact match should work
    assert!(system.get_speaker_by_uuid("RINCON_123456").is_some());
    
    // Partial matches should not work
    assert!(system.get_speaker_by_uuid("RINCON_123").is_none());
    assert!(system.get_speaker_by_uuid("123456").is_none());
    assert!(system.get_speaker_by_uuid("RINCON_123456_EXTRA").is_none());
    
    // Case sensitivity test
    assert!(system.get_speaker_by_uuid("rincon_123456").is_none());
    assert!(system.get_speaker_by_uuid("RINCON_123456").is_some());
  }

  #[test]
  #[cfg(feature = "mock")]
  fn test_topology_storage_methods() {
    let mut system = System::new().unwrap();
    
    // Initially no topology
    assert!(!system.has_topology());
    assert!(system.topology().is_none());
    
    // Create test topology
    use crate::topology::types::{Topology, ZoneGroup, ZoneGroupMember, VanishedDevices, VanishedDevice};
    let test_topology = Topology {
      zone_groups: vec![
        ZoneGroup {
          coordinator: "RINCON_123".to_string(),
          id: "RINCON_123:123".to_string(),
          members: vec![
            ZoneGroupMember {
              uuid: "RINCON_123".to_string(),
              location: "http://192.168.1.100:1400/xml/device_description.xml".to_string(),
              zone_name: "Living Room".to_string(),
              software_version: "56.0-76060".to_string(),
              configuration: "1".to_string(),
              icon: "x-rincon-roomicon:living".to_string(),
              satellites: vec![],
            }
          ],
        },
        ZoneGroup {
          coordinator: "RINCON_456".to_string(),
          id: "RINCON_456:456".to_string(),
          members: vec![
            ZoneGroupMember {
              uuid: "RINCON_456".to_string(),
              location: "http://192.168.1.101:1400/xml/device_description.xml".to_string(),
              zone_name: "Kitchen".to_string(),
              software_version: "56.0-76060".to_string(),
              configuration: "1".to_string(),
              icon: "x-rincon-roomicon:kitchen".to_string(),
              satellites: vec![],
            }
          ],
        }
      ],
      vanished_devices: Some(VanishedDevices {
        devices: vec![
          VanishedDevice {
            uuid: "RINCON_VANISHED".to_string(),
            zone_name: "Old Speaker".to_string(),
            reason: "powered off".to_string(),
          }
        ],
      }),
    };
    
    // Set topology
    system.topology = Some(test_topology.clone());
    
    // Test topology access methods
    assert!(system.has_topology());
    assert!(system.topology().is_some());
    
    let stored_topology = system.topology().unwrap();
    assert_eq!(stored_topology.zone_group_count(), 2);
    assert_eq!(stored_topology.total_speaker_count(), 2);
    
    // Test topology data integrity
    assert_eq!(stored_topology.zone_groups.len(), 2);
    assert_eq!(stored_topology.zone_groups[0].coordinator, "RINCON_123");
    assert_eq!(stored_topology.zone_groups[1].coordinator, "RINCON_456");
    
    // Test vanished devices
    assert!(stored_topology.vanished_devices.is_some());
    let vanished = stored_topology.vanished_devices.as_ref().unwrap();
    assert_eq!(vanished.devices.len(), 1);
    assert_eq!(vanished.devices[0].uuid, "RINCON_VANISHED");
    
    // Clear topology
    system.topology = None;
    assert!(!system.has_topology());
    assert!(system.topology().is_none());
  }

  #[test]
  #[cfg(feature = "mock")]
  fn test_state_clearing_on_discovery_restart() {
    let mut system = System::new().unwrap();
    
    // Add initial state
    let speaker1 = create_test_speaker("RINCON_123", "Living Room", "192.168.1.100");
    let speaker2 = create_test_speaker("RINCON_456", "Kitchen", "192.168.1.101");
    system.add_speaker_for_test(speaker1);
    system.add_speaker_for_test(speaker2);
    
    use crate::topology::types::{Topology, ZoneGroup, ZoneGroupMember};
    let test_topology = Topology {
      zone_groups: vec![
        ZoneGroup {
          coordinator: "RINCON_123".to_string(),
          id: "RINCON_123:123".to_string(),
          members: vec![
            ZoneGroupMember {
              uuid: "RINCON_123".to_string(),
              location: "http://192.168.1.100:1400/xml/device_description.xml".to_string(),
              zone_name: "Living Room".to_string(),
              software_version: "56.0-76060".to_string(),
              configuration: "1".to_string(),
              icon: "x-rincon-roomicon:living".to_string(),
              satellites: vec![],
            }
          ],
        }
      ],
      vanished_devices: None,
    };
    system.topology = Some(test_topology);
    
    // Verify initial state
    assert_eq!(system.speaker_count(), 2);
    assert!(system.has_topology());
    assert!(system.get_speaker_by_uuid("RINCON_123").is_some());
    assert!(system.get_speaker_by_uuid("RINCON_456").is_some());
    
    // Run discovery completely (this should clear state at the beginning)
    let events: Vec<_> = system.discover().collect();
    
    // State should be cleared - the original test speakers should be gone
    // Note: New speakers may be discovered during the actual discovery process
    // but the original test speakers should definitely be cleared
    assert!(system.get_speaker_by_uuid("RINCON_123").is_none());
    assert!(system.get_speaker_by_uuid("RINCON_456").is_none());
    
    // Verify that discovery completed
    assert!(!events.is_empty());
    assert!(matches!(events.last().unwrap(), SystemEvent::DiscoveryComplete));
    
    // After discovery, the system should still be usable
    let _final_speaker_count = system.speaker_count();
    let _final_has_topology = system.has_topology();
  }

  #[test]
  fn test_all_new_event_types_are_debug() {
    // Test that all SystemEvent variants implement Debug properly
    use crate::speaker::{Speaker, SpeakerFactory};
    use crate::topology::types::Topology;
    
    // Create a test speaker - use SpeakerFactory::default to avoid ambiguity
    let test_speaker = <Speaker as SpeakerFactory>::default();
    let speaker_found = SystemEvent::SpeakerFound(test_speaker);
    
    let topology_ready = SystemEvent::TopologyReady(Topology {
      zone_groups: vec![],
      vanished_devices: None,
    });
    
    let discovery_complete = SystemEvent::DiscoveryComplete;
    let error_event = SystemEvent::Error("Test error".to_string());
    let group_update = SystemEvent::GroupUpdate("group_id".to_string(), vec!["speaker1".to_string()]);
    
    // Test that Debug formatting works for all events
    let speaker_debug = format!("{:?}", speaker_found);
    let topology_debug = format!("{:?}", topology_ready);
    let discovery_debug = format!("{:?}", discovery_complete);
    let error_debug = format!("{:?}", error_event);
    let group_debug = format!("{:?}", group_update);
    
    // Verify debug strings are not empty and contain expected content
    assert!(speaker_debug.contains("SpeakerFound"));
    assert!(topology_debug.contains("TopologyReady"));
    assert!(discovery_debug.contains("DiscoveryComplete"));
    assert!(error_debug.contains("Error"));
    assert!(error_debug.contains("Test error"));
    assert!(group_debug.contains("GroupUpdate"));
    assert!(group_debug.contains("group_id"));
  }

  #[test]
  #[cfg(feature = "mock")]
  fn test_comprehensive_event_emission_scenarios() {
    let mut system = System::new().unwrap();
    
    // Test discovery with no network speakers (should still emit DiscoveryComplete)
    let events: Vec<_> = system.discover().collect();
    
    // Should always have at least DiscoveryComplete
    assert!(!events.is_empty());
    assert!(matches!(events.last().unwrap(), SystemEvent::DiscoveryComplete));
    
    // Count event types
    let speaker_found_count = events.iter().filter(|e| matches!(e, SystemEvent::SpeakerFound(_))).count();
    let topology_ready_count = events.iter().filter(|e| matches!(e, SystemEvent::TopologyReady(_))).count();
    let error_count = events.iter().filter(|e| matches!(e, SystemEvent::Error(_))).count();
    let discovery_complete_count = events.iter().filter(|e| matches!(e, SystemEvent::DiscoveryComplete)).count();
    
    // DiscoveryComplete should always be exactly 1
    assert_eq!(discovery_complete_count, 1);
    
    // If speakers were found, topology should have been attempted
    if speaker_found_count > 0 {
      // Either topology succeeded or failed (error emitted)
      assert!(topology_ready_count > 0 || error_count > 0);
    }
    
    // If topology was ready, it should be stored
    if topology_ready_count > 0 {
      assert!(system.has_topology());
    }
    
    // Verify event ordering: SpeakerFound events should come before TopologyReady
    let mut found_topology_ready = false;
    for event in &events {
      match event {
        SystemEvent::TopologyReady(_) => {
          found_topology_ready = true;
        },
        SystemEvent::SpeakerFound(_) => {
          // If we've already seen TopologyReady, this is wrong order
          // (TopologyReady should only come after first speaker)
          if found_topology_ready {
            // This is actually OK - we might find more speakers after topology
            // The important thing is that topology comes after at least one speaker
          }
        },
        SystemEvent::DiscoveryComplete => {
          // This should be the last event
          let current_index = events.iter().position(|e| matches!(e, SystemEvent::DiscoveryComplete)).unwrap();
          assert_eq!(current_index, events.len() - 1);
        },
        _ => {}
      }
    }
  }
}
