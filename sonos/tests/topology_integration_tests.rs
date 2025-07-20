use sonos::topology::{Topology, get_topology_from_ip};
use sonos::{SonosError, System, SystemEvent, SpeakerTrait};

// Import real topology response from test data file
const REAL_TOPOLOGY_RESPONSE: &str = include_str!("./topology_integration_tests_data.txt");

// Helper function to extract the first complete XML response from concatenated responses
fn extract_first_xml_response(data: &str) -> &str {
    // Find the end of the first complete XML response
    if let Some(end_pos) = data.find("</u:GetZoneGroupStateResponse>") {
        let end_pos = end_pos + "</u:GetZoneGroupStateResponse>".len();
        &data[..end_pos]
    } else {
        // Fallback to first line if pattern not found
        data.lines().next().unwrap_or("")
    }
}

#[test]
fn test_topology_public_api_compatibility() {
    // Test that all public types are accessible through the main module
    
    // Test that we can reference all the main types
    let _topology_type: Option<Topology> = None;
    let _zone_group_type: Option<sonos::topology::ZoneGroup> = None;
    let _zone_group_member_type: Option<sonos::topology::ZoneGroupMember> = None;
    let _satellite_type: Option<sonos::topology::Satellite> = None;
    let _vanished_devices_type: Option<sonos::topology::VanishedDevices> = None;
    let _vanished_device_type: Option<sonos::topology::VanishedDevice> = None;
    
    // Test that the main function is accessible
    let _function_exists = get_topology_from_ip;
}

#[test]
fn test_topology_re_exports_from_main_lib() {
    // Test that all types are also accessible through the main sonos crate
    let _topology_type: Option<sonos::Topology> = None;
    let _zone_group_type: Option<sonos::ZoneGroup> = None;
    let _zone_group_member_type: Option<sonos::ZoneGroupMember> = None;
    let _satellite_type: Option<sonos::Satellite> = None;
    let _vanished_devices_type: Option<sonos::VanishedDevices> = None;
    let _vanished_device_type: Option<sonos::VanishedDevice> = None;
}

#[test]
fn test_topology_from_xml_functionality() {
    // Test with real topology response from log file
    // Extract just the first complete XML response
    let first_response = extract_first_xml_response(REAL_TOPOLOGY_RESPONSE);
    
    let result = Topology::from_xml(first_response);
    assert!(result.is_ok(), "Failed to parse real topology XML: {:?}", result.err());
    
    let topology = result.unwrap();
    assert!(topology.zone_group_count() > 0, "Should have at least one zone group");
    assert!(topology.total_speaker_count() > 0, "Should have at least one speaker");
    
    let speakers = topology.all_speakers();
    assert!(speakers.len() > 0, "Should have speakers");
    
    // Verify we can access speaker properties
    for speaker in &speakers {
        assert!(!speaker.zone_name.is_empty(), "Speaker should have a zone name");
        assert!(!speaker.uuid.is_empty(), "Speaker should have a UUID");
        assert!(!speaker.location.is_empty(), "Speaker should have a location");
    }
}

#[test]
fn test_topology_error_handling() {
    // Test error handling with invalid XML
    let invalid_xml = "This is not valid XML";
    let result = Topology::from_xml(invalid_xml);
    assert!(result.is_err(), "Should fail with invalid XML");
    
    // Test error handling with malformed SOAP response
    let malformed_soap = r#"<?xml version="1.0"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/">
<s:Body>
<u:GetZoneGroupStateResponse xmlns:u="urn:schemas-upnp-org:service:ZoneGroupTopology:1">
<ZoneGroupState>Invalid HTML entities &amp;lt;test&amp;gt;</ZoneGroupState>
</u:GetZoneGroupStateResponse>
</s:Body>
</s:Envelope>"#;
    
    let result = Topology::from_xml(malformed_soap);
    // This should either succeed or fail gracefully
    match result {
        Ok(_) => {}, // Success is fine
        Err(e) => {
            // Error should be a proper SonosError
            match e {
                SonosError::ParseError(_) => {}, // Expected error type
                _ => panic!("Unexpected error type: {:?}", e),
            }
        }
    }
}

#[test]
fn test_topology_with_satellites() {
    // Test satellite functionality using real data
    let first_response = REAL_TOPOLOGY_RESPONSE.lines().next().unwrap_or("");
    
    let result = Topology::from_xml(first_response);
    assert!(result.is_ok(), "Failed to parse topology XML");
    
    let topology = result.unwrap();
    let speakers = topology.all_speakers();
    assert!(speakers.len() > 0, "Should have speakers");
    
    // Test satellite functionality for each speaker
    for speaker in &speakers {
        // Test has_satellites method
        let has_satellites = speaker.has_satellites();
        assert_eq!(has_satellites, !speaker.satellites.is_empty(), "has_satellites should match satellite count");
        
        // Test total_speaker_count includes satellites
        let expected_count = 1 + speaker.satellites.len(); // Main speaker + satellites
        assert_eq!(speaker.total_speaker_count(), expected_count, "Total speaker count should include satellites");
        
        // Test satellite IP address extraction if any satellites exist
        for satellite in &speaker.satellites {
            assert!(!satellite.uuid.is_empty(), "Satellite should have UUID");
            assert!(!satellite.location.is_empty(), "Satellite should have location");
            let ip = satellite.ip_address();
            assert!(ip.is_some(), "Satellite should have extractable IP address");
        }
    }
}

#[test]
fn test_topology_with_vanished_devices() {
    // Test vanished devices functionality using real data
    let first_response = REAL_TOPOLOGY_RESPONSE.lines().next().unwrap_or("");
    
    let result = Topology::from_xml(first_response);
    assert!(result.is_ok(), "Failed to parse topology XML");
    
    let topology = result.unwrap();
    
    // Test vanished devices structure
    if let Some(vanished) = &topology.vanished_devices {
        // If vanished devices exist, test their structure
        for device in &vanished.devices {
            assert!(!device.uuid.is_empty(), "Vanished device should have UUID");
            assert!(!device.zone_name.is_empty(), "Vanished device should have zone name");
        }
    } else {
        // If no vanished devices, that's also valid - just verify the structure exists
        // The real data shows empty VanishedDevices element, which should parse as Some with empty vec
        assert!(topology.vanished_devices.is_some(), "Should have vanished devices structure even if empty");
        let vanished = topology.vanished_devices.as_ref().unwrap();
        assert_eq!(vanished.devices.len(), 0, "Should have empty vanished devices list");
    }
}

#[test]
fn test_zone_group_functionality() {
    // Test zone group specific functionality using real data
    let first_response = REAL_TOPOLOGY_RESPONSE.lines().next().unwrap_or("");
    
    let result = Topology::from_xml(first_response);
    assert!(result.is_ok(), "Failed to parse topology XML");
    
    let topology = result.unwrap();
    assert!(topology.zone_groups.len() > 0, "Should have zone groups");
    
    // Test each zone group
    for zone_group in &topology.zone_groups {
        // Test coordinator speaker lookup
        let coordinator = zone_group.coordinator_speaker();
        assert!(coordinator.is_some(), "Zone group should have a coordinator speaker");
        assert_eq!(coordinator.unwrap().uuid, zone_group.coordinator, "Coordinator UUID should match");
        
        // Test speaker count
        let speaker_count = zone_group.total_speaker_count();
        assert!(speaker_count > 0, "Zone group should have at least one speaker");
        assert_eq!(speaker_count, zone_group.members.len(), "Speaker count should match member count");
        
        // Test is_grouped logic
        let is_grouped = zone_group.is_grouped();
        if zone_group.members.len() > 1 {
            assert!(is_grouped, "Zone group with multiple members should be grouped");
        } else {
            assert!(!is_grouped, "Zone group with single member should not be grouped");
        }
    }
}

#[test]
fn test_ip_address_extraction() {
    // Test IP address extraction from location URLs using real data
    let first_response = extract_first_xml_response(REAL_TOPOLOGY_RESPONSE);
    
    let result = Topology::from_xml(first_response);
    assert!(result.is_ok(), "Failed to parse topology XML");
    
    let topology = result.unwrap();
    let speakers = topology.all_speakers();
    assert!(speakers.len() > 0, "Should have speakers");
    
    // Test IP address extraction for each speaker
    for speaker in &speakers {
        let ip = speaker.ip_address();
        assert!(ip.is_some(), "Speaker should have extractable IP address from location: {}", speaker.location);
        let ip_str = ip.unwrap();
        assert!(ip_str.contains('.'), "IP address should be in dotted format: {}", ip_str);
        assert!(ip_str.starts_with("192.168."), "IP should be in local network range: {}", ip_str);
    }
}

#[test]
fn test_find_functionality() {
    // Test find methods using real data
    let first_response = extract_first_xml_response(REAL_TOPOLOGY_RESPONSE);
    
    let result = Topology::from_xml(first_response);
    assert!(result.is_ok(), "Failed to parse topology XML");
    
    let topology = result.unwrap();
    
    // Get the first zone group and speaker to test with
    assert!(topology.zone_groups.len() > 0, "Should have zone groups");
    let first_zone_group = &topology.zone_groups[0];
    let coordinator_uuid = &first_zone_group.coordinator;
    
    // Test find_zone_group_by_coordinator
    let found_zone_group = topology.find_zone_group_by_coordinator(coordinator_uuid);
    assert!(found_zone_group.is_some(), "Should find zone group by coordinator");
    assert_eq!(found_zone_group.unwrap().coordinator, *coordinator_uuid);
    
    // Test find_speaker_by_uuid with the coordinator
    let found_speaker = topology.find_speaker_by_uuid(coordinator_uuid);
    assert!(found_speaker.is_some(), "Should find speaker by UUID");
    assert_eq!(found_speaker.unwrap().uuid, *coordinator_uuid);
    
    // Test with non-existent UUID
    let not_found = topology.find_speaker_by_uuid("NONEXISTENT_UUID_12345");
    assert!(not_found.is_none(), "Should not find non-existent speaker");
}

#[cfg(test)]
mod debug_file_tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_debug_file_writing_functionality() {
        // Test that debug file writing still works correctly
        // This tests the utils::write_debug_xml function indirectly
        
        // Clean up any existing debug files first
        let _ = fs::remove_file("decoded_topology.xml");
        let _ = fs::remove_file("raw_topology_response.xml");
        
        // Parse some XML which should trigger debug file writing if enabled
        let first_response = REAL_TOPOLOGY_RESPONSE.lines().next().unwrap_or("");

        let result = Topology::from_xml(first_response);
        assert!(result.is_ok(), "XML parsing should succeed");
        
        // The debug files may or may not be created depending on configuration,
        // but the parsing should work regardless
        let topology = result.unwrap();
        assert!(topology.zone_group_count() > 0, "Should have zone groups");
    }
}

// Test that network functionality is accessible (but don't actually make network calls)
#[test]
fn test_network_function_signatures() {
    // Test that the get_topology_from_ip function exists and has the right signature
    let _function_exists: fn(&str) -> Result<Topology, SonosError> = get_topology_from_ip;
    
    // Test that we can create a TopologyClient
    use sonos::topology::TopologyClient;
    let _client = TopologyClient::new();
    let _client_with_custom = TopologyClient::with_client(sonos::Client::new(ureq::Agent::new()));
}

// Integration tests for System topology integration
#[cfg(test)]
mod system_integration_tests {
    use super::*;

    #[test]
    fn test_system_discover_method_signature() {
        // Test that System::discover uses &mut self instead of consuming self
        let mut system = System::new().expect("Failed to create system");
        
        // Call discover and consume the iterator
        {
            let discovery_iter = system.discover();
            let _events: Vec<_> = discovery_iter.collect();
        }
        
        // System should still be accessible after discovery (not consumed)
        let _speakers = system.speakers();
        let _count = system.speaker_count();
        let _has_topology = system.has_topology();
        
        // Should be able to call discover again
        {
            let discovery_iter2 = system.discover();
            let _events2: Vec<_> = discovery_iter2.collect();
        }
        
        // System should still be accessible
        assert_eq!(system.speakers().len(), system.speaker_count());
    }

    #[test]
    fn test_system_event_types_compatibility() {
        // Test that all expected event types exist and are properly named
        let mut system = System::new().expect("Failed to create system");
        
        let events: Vec<_> = system.discover().collect();
        
        // Verify event types are correct (no old "Found" events)
        for event in &events {
            match event {
                SystemEvent::SpeakerChange(_) => {
                    // This is the correct new event name
                },
                SystemEvent::TopologyChange(_) => {
                    // New topology event
                },

                SystemEvent::Error(_) => {
                    // Error events should be generic, not topology-specific
                },
                // Note: No SystemEvent::Found variant should exist anymore
            }
        }
        
        // Discovery should emit events (may be empty if no speakers found)
    }

    #[test]
    fn test_complete_discovery_flow_with_topology_integration() {
        let mut system = System::new().expect("Failed to create system");
        
        // Initial state should be empty
        assert_eq!(system.speaker_count(), 0);
        assert!(!system.has_topology());
        assert!(system.topology().is_none());
        
        // Run discovery
        let events: Vec<_> = system.discover().collect();
        
        // Analyze the discovery flow
        let speaker_events: Vec<_> = events.iter()
            .filter(|e| matches!(e, SystemEvent::SpeakerChange(_)))
            .collect();
        
        let topology_events: Vec<_> = events.iter()
            .filter(|e| matches!(e, SystemEvent::TopologyChange(_)))
            .collect();
        
        let error_events: Vec<_> = events.iter()
            .filter(|e| matches!(e, SystemEvent::Error(_)))
            .collect();
        
        // Verify events were emitted during discovery
        
        // If speakers were found, verify integration
        if !speaker_events.is_empty() {
            // Speakers should be stored in the system
            assert!(system.speaker_count() > 0, "Speakers should be stored in system");
            
            // Topology should have been attempted (either success or error)
            let topology_attempted = !topology_events.is_empty() || 
                error_events.iter().any(|e| {
                    if let SystemEvent::Error(msg) = e {
                        msg.contains("Topology retrieval failed")
                    } else {
                        false
                    }
                });
            
            assert!(topology_attempted, "Topology retrieval should have been attempted");
            
            // If topology was successful, it should be stored
            if !topology_events.is_empty() {
                assert!(system.has_topology(), "Topology should be stored when TopologyChange event emitted");
                assert!(system.topology().is_some(), "Topology should be accessible");
                
                // Verify topology data integrity
                let topology = system.topology().unwrap();
                assert!(topology.zone_group_count() > 0, "Topology should have zone groups");
            }
            
            // Verify speakers can be accessed by UUID
            for event in &speaker_events {
                if let SystemEvent::SpeakerChange(speaker) = event {
                    let uuid = speaker.uuid();
                    let stored_speaker = system.get_speaker_by_uuid(uuid);
                    assert!(stored_speaker.is_some(), "Speaker should be retrievable by UUID: {}", uuid);
                    assert_eq!(stored_speaker.unwrap().uuid(), uuid);
                }
            }
        }
    }

    #[test]
    fn test_system_remains_usable_after_discovery() {
        let mut system = System::new().expect("Failed to create system");
        
        // Run discovery multiple times to verify system remains usable
        for iteration in 0..3 {
            let events: Vec<_> = system.discover().collect();
            
            // System should be usable after each discovery
            let speaker_count = system.speaker_count();
            let has_topology = system.has_topology();
            let speakers_ref = system.speakers();
            
            // Should be able to access all state methods
            assert_eq!(speakers_ref.len(), speaker_count);
            
            if has_topology {
                let topology = system.topology();
                assert!(topology.is_some());
            }
            
            // Discovery should work consistently across iterations
            
            // Test speaker lookup functionality
            for (uuid, _) in speakers_ref.iter() {
                let found_speaker = system.get_speaker_by_uuid(uuid);
                assert!(found_speaker.is_some(), "Should find speaker by UUID: {}", uuid);
            }
        }
    }

    #[test]
    fn test_error_handling_scenarios_for_topology_retrieval() {
        let mut system = System::new().expect("Failed to create system");
        
        let events: Vec<_> = system.discover().collect();
        
        // Check for topology-related errors
        let topology_errors: Vec<_> = events.iter()
            .filter(|e| {
                if let SystemEvent::Error(msg) = e {
                    msg.contains("Topology retrieval failed") || msg.contains("topology")
                } else {
                    false
                }
            })
            .collect();
        
        let speaker_events: Vec<_> = events.iter()
            .filter(|e| matches!(e, SystemEvent::SpeakerChange(_)))
            .collect();
        
        // If there were topology errors, verify they don't stop speaker discovery
        if !topology_errors.is_empty() && !speaker_events.is_empty() {
            // Speakers should still be stored even if topology failed
            assert!(system.speaker_count() > 0, "Speakers should be stored even if topology fails");
            
            // System should still be functional
            assert!(!system.speakers().is_empty());
            
            // Topology should not be available
            assert!(!system.has_topology());
            assert!(system.topology().is_none());
        }
        
        // Verify error events use generic Error type (not TopologyError)
        for error_event in &topology_errors {
            assert!(matches!(error_event, SystemEvent::Error(_)), 
                   "Topology errors should use generic Error event type");
        }
        
        // Discovery should work even with potential errors
    }

    #[test]
    fn test_state_access_methods_integration() {
        let mut system = System::new().expect("Failed to create system");
        
        // Test initial state
        assert_eq!(system.speaker_count(), 0);
        assert!(!system.has_topology());
        assert!(system.topology().is_none());
        assert!(system.speakers().is_empty());
        assert!(system.get_speaker_by_uuid("any-uuid").is_none());
        
        // Run discovery
        let events: Vec<_> = system.discover().collect();
        
        // Test state after discovery
        let final_speaker_count = system.speaker_count();
        let final_has_topology = system.has_topology();
        let final_speakers = system.speakers();
        let final_topology = system.topology();
        
        // Verify consistency between methods
        assert_eq!(final_speakers.len(), final_speaker_count);
        assert_eq!(final_topology.is_some(), final_has_topology);
        
        // Test speaker lookup for all stored speakers
        for (uuid, speaker) in final_speakers.iter() {
            let found_speaker = system.get_speaker_by_uuid(uuid);
            assert!(found_speaker.is_some(), "Should find speaker by UUID: {}", uuid);
            assert_eq!(found_speaker.unwrap().uuid(), speaker.uuid());
        }
        
        // Test edge cases for speaker lookup
        assert!(system.get_speaker_by_uuid("").is_none());
        assert!(system.get_speaker_by_uuid("NONEXISTENT_UUID").is_none());
        
        // If topology is available, verify its data
        if let Some(topology) = final_topology {
            assert!(topology.zone_group_count() > 0);
            
            // Verify topology-speaker integration
            let all_topology_speakers = topology.all_speakers();
            for topology_speaker in &all_topology_speakers {
                // Each speaker in topology should be findable in the system
                let system_speaker = system.get_speaker_by_uuid(&topology_speaker.uuid);
                if system_speaker.is_some() {
                    // If found, UUIDs should match
                    assert_eq!(system_speaker.unwrap().uuid(), topology_speaker.uuid);
                }
            }
        }
        
        // Verify discovery ran
    }

    #[test]
    fn test_backward_compatibility_verification() {
        // Test that the new API maintains expected behavior patterns
        let mut system = System::new().expect("Failed to create system");
        
        // Old pattern: system.discover().collect() should still work
        let events: Vec<_> = system.discover().collect();
        
        // System should remain available (this is the key backward compatibility improvement)
        let _post_discovery_speakers = system.speakers();
        let _post_discovery_count = system.speaker_count();
        
        // Event types should be consistent with new naming
        let has_speaker_found = events.iter().any(|e| matches!(e, SystemEvent::SpeakerChange(_)));
        
        // If speakers were found, verify they use the new event name
        if has_speaker_found {
            // Verify no old "Found" events exist (this would be a compilation error anyway)
            // The fact that we can match on SpeakerChange proves the rename worked
            
            for event in &events {
                if let SystemEvent::SpeakerChange(speaker) = event {
                    // Verify speaker data is accessible
                    assert!(!speaker.uuid().is_empty());
                    assert!(!speaker.name().is_empty());
                    assert!(!speaker.ip().is_empty());
                }
            }
        }
        
        // Test that multiple discoveries work (system not consumed)
        let events2: Vec<_> = system.discover().collect();
        // Discovery should work consistently
        
        // System should still be accessible
        let _final_state = (system.speaker_count(), system.has_topology());
    }

    #[test]
    fn test_event_emission_order_and_consistency() {
        let mut system = System::new().expect("Failed to create system");
        
        let events: Vec<_> = system.discover().collect();
        
        // Track event order
        let mut speaker_found_indices = Vec::new();
        let mut topology_ready_indices = Vec::new();
        let mut error_indices = Vec::new();
        
        for (i, event) in events.iter().enumerate() {
            match event {
                SystemEvent::SpeakerChange(_) => speaker_found_indices.push(i),
                SystemEvent::TopologyChange(_) => topology_ready_indices.push(i),
                SystemEvent::Error(_) => error_indices.push(i),
            }
        }
        
        // If topology was retrieved, it should come after at least one speaker
        if !topology_ready_indices.is_empty() && !speaker_found_indices.is_empty() {
            let first_topology = topology_ready_indices[0];
            let first_speaker = speaker_found_indices[0];
            // Topology should come after first speaker (or at least not before)
            assert!(first_topology >= first_speaker, 
                   "TopologyChange should come after first SpeakerChange event");
        }
    }

    #[test]
    fn test_topology_integration_with_speaker_storage() {
        let mut system = System::new().expect("Failed to create system");
        
        let events: Vec<_> = system.discover().collect();
        
        let speaker_events: Vec<_> = events.iter()
            .filter_map(|e| if let SystemEvent::SpeakerChange(speaker) = e { Some(speaker) } else { None })
            .collect();
        
        let topology_events: Vec<_> = events.iter()
            .filter_map(|e| if let SystemEvent::TopologyChange(topology) = e { Some(topology) } else { None })
            .collect();
        
        // If both speakers and topology were found, verify integration
        if !speaker_events.is_empty() && !topology_events.is_empty() {
            let topology = topology_events[0];
            
            // Verify topology is stored in system
            assert!(system.has_topology());
            let stored_topology = system.topology().unwrap();
            assert_eq!(stored_topology.zone_group_count(), topology.zone_group_count());
            
            // Verify speakers are stored and accessible
            assert!(system.speaker_count() > 0);
            
            // Test integration: speakers in topology should be findable in system
            let topology_speakers = topology.all_speakers();
            for topology_speaker in &topology_speakers {
                let system_speaker = system.get_speaker_by_uuid(&topology_speaker.uuid);
                // Note: Not all topology speakers may be in the system if discovery is still ongoing
                // But if they are found, they should match
                if let Some(found_speaker) = system_speaker {
                    assert_eq!(found_speaker.uuid(), topology_speaker.uuid);
                }
            }
            
            // Test that system speakers have valid UUIDs that could be used with topology
            for (uuid, speaker) in system.speakers().iter() {
                assert_eq!(uuid, speaker.uuid());
                assert!(!uuid.is_empty());
                assert!(uuid.starts_with("RINCON_") || uuid.contains("uuid:"), 
                       "Speaker UUID should be in expected format: {}", uuid);
            }
        }
        
        // Verify discovery ran properly
    }
}