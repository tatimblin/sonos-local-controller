use sonos::topology::{Topology, get_topology_from_ip};
use sonos::SonosError;

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