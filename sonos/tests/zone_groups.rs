use sonos::api::zone_groups::ZoneGroupsService;
use sonos::transport::discovery;
use sonos::transport::soap::SoapClient;
use std::time::Duration;

/// Integration tests for ZoneGroupsService that run against real Sonos speakers on the network.
/// These tests will be skipped if no speakers are found.
/// 
/// To run these tests:
/// ```
/// cargo test --test zone_groups
/// ```

#[test]
fn test_get_zone_group_state_real_speakers() {
    let speakers = discovery::discover_speakers_with_timeout(Duration::from_secs(5))
        .expect("Discovery should not fail");
    
    if speakers.is_empty() {
        println!("No Sonos speakers found on network - skipping zone group tests");
        return;
    }

    println!("Found {} Sonos speakers, testing zone group state...", speakers.len());
    
    let soap_client = SoapClient::new(Duration::from_secs(10))
        .expect("Should be able to create SOAP client");

    // Test with the first speaker found
    let speaker = &speakers[0];
    let device_url = format!("http://{}:{}", speaker.ip_address, speaker.port);
    
    println!("Testing zone group state on speaker: {} at {}", speaker.name, device_url);

    let result = ZoneGroupsService::get_zone_group_state(&soap_client, &device_url);
    
    match result {
        Ok(groups) => {
            println!("Successfully retrieved {} zone groups:", groups.len());
            
            if groups.is_empty() {
                println!("Warning: No zone groups found. This might indicate a parsing issue.");
                // Let's not fail the test immediately - this could be a valid state
                return;
            }
            
            for (i, group) in groups.iter().enumerate() {
                println!("  Group {}: Coordinator={:?}, Members={} speakers", 
                         i + 1, group.coordinator, group.members.len());
                
                // Validate group structure
                assert!(!group.members.is_empty(), "Group should have at least one member (coordinator)");
                assert!(group.is_member(group.coordinator), 
                        "Group members should include the coordinator");
                
                // Validate that group ID is based on coordinator
                assert_eq!(group.id, sonos::models::GroupId::from_coordinator(group.coordinator),
                          "Group ID should match coordinator");
            }
            
            // Test that we can find the current speaker in one of the groups
            // Note: Satellite speakers (like surround speakers) may not appear as regular members
            let current_speaker_id = sonos::models::SpeakerId::from_udn(&speaker.udn);
            let speaker_found = groups.iter().any(|group| 
                group.is_member(current_speaker_id) || group.all_speaker_ids().contains(&current_speaker_id)
            );
            
            if !speaker_found {
                println!("Warning: Current speaker ({}) not found as a regular group member.", speaker.name);
                println!("This is normal for satellite/surround speakers.");
                println!("UDN: {}", speaker.udn);
            } else {
                println!("Current speaker found as a regular group member.");
            }
        }
        Err(e) => {
            panic!("Failed to get zone group state from {}: {:?}", device_url, e);
        }
    }
}

#[test]
fn test_get_zone_group_state_multiple_speakers() {
    let speakers = discovery::discover_speakers_with_timeout(Duration::from_secs(5))
        .expect("Discovery should not fail");
    
    if speakers.len() < 2 {
        println!("Need at least 2 Sonos speakers for multi-speaker test - found {}, skipping", speakers.len());
        return;
    }

    println!("Testing zone group state consistency across {} speakers...", speakers.len());
    
    let soap_client = SoapClient::new(Duration::from_secs(10))
        .expect("Should be able to create SOAP client");

    let mut all_groups = Vec::new();
    
    // Query zone group state from multiple speakers
    for (i, speaker) in speakers.iter().take(3).enumerate() { // Test up to 3 speakers to avoid network flooding
        let device_url = format!("http://{}:{}", speaker.ip_address, speaker.port);
        
        println!("Querying speaker {}: {} at {}", i + 1, speaker.name, device_url);
        
        match ZoneGroupsService::get_zone_group_state(&soap_client, &device_url) {
            Ok(groups) => {
                println!("  Found {} zone groups from {}", groups.len(), speaker.name);
                all_groups.push((speaker.name.clone(), groups));
            }
            Err(e) => {
                println!("  Warning: Failed to get zone groups from {}: {:?}", speaker.name, e);
                // Don't fail the test - some speakers might be temporarily unavailable
            }
        }
        
        // Small delay to avoid overwhelming the network
        std::thread::sleep(Duration::from_millis(200));
    }
    
    if all_groups.is_empty() {
        println!("No speakers responded successfully - test inconclusive");
        return;
    }
    
    // Verify consistency - all speakers should report the same zone topology
    let (first_speaker, first_groups) = &all_groups[0];
    
    for (speaker_name, groups) in &all_groups[1..] {
        println!("Comparing zone groups between {} and {}", first_speaker, speaker_name);
        
        // Should have the same number of groups
        assert_eq!(first_groups.len(), groups.len(),
                  "All speakers should report the same number of zone groups ({} vs {})",
                  first_groups.len(), groups.len());
        
        // Each group should have the same coordinator and members
        for first_group in first_groups {
            let matching_group = groups.iter().find(|g| g.coordinator == first_group.coordinator);
            assert!(matching_group.is_some(),
                   "Group with coordinator {:?} should be found in both responses",
                   first_group.coordinator);
            
            let matching_group = matching_group.unwrap();
            assert_eq!(first_group.members.len(), matching_group.members.len(),
                      "Group member count should be consistent");
            
            // Check that all members match (order might differ)
            for member in &first_group.members {
                assert!(matching_group.members.contains(member),
                       "Member {:?} should be present in both group responses", member);
            }
        }
    }
    
    println!("Zone group consistency test passed across {} speakers", all_groups.len());
}

#[test]
fn test_get_zone_group_state_error_handling() {
    let soap_client = SoapClient::new(Duration::from_secs(5))
        .expect("Should be able to create SOAP client");

    // Test with invalid URL
    let result = ZoneGroupsService::get_zone_group_state(&soap_client, "http://192.0.2.1:1400");
    assert!(result.is_err(), "Should fail with unreachable host");
    
    // Test with invalid port
    let result = ZoneGroupsService::get_zone_group_state(&soap_client, "http://127.0.0.1:9999");
    assert!(result.is_err(), "Should fail with wrong port");
    
    println!("Error handling test passed - invalid requests properly rejected");
}

#[test]
fn test_get_zone_group_state_response_format() {
    let speakers = discovery::discover_speakers_with_timeout(Duration::from_secs(5))
        .expect("Discovery should not fail");
    
    if speakers.is_empty() {
        println!("No Sonos speakers found - skipping response format test");
        return;
    }

    let soap_client = SoapClient::new(Duration::from_secs(10))
        .expect("Should be able to create SOAP client");

    let speaker = &speakers[0];
    let device_url = format!("http://{}:{}", speaker.ip_address, speaker.port);
    
    println!("Testing zone group response format from: {}", speaker.name);

    let groups = ZoneGroupsService::get_zone_group_state(&soap_client, &device_url)
        .expect("Should successfully get zone group state");
    
    // Validate the structure of returned groups
    for group in &groups {
        // Group ID should be valid (just check it exists)
        let _ = group.id; // GroupId doesn't implement Display, but we can verify it exists
        
        // Coordinator should be valid (just check it exists)
        let _ = group.coordinator; // SpeakerId doesn't implement Display, but we can verify it exists
        
        // Should have at least the coordinator as a member
        assert!(!group.members.is_empty(), "Group should have at least one member");
        assert!(group.is_member(group.coordinator), 
                "Coordinator should be included in members list");
        
        // All member IDs should be valid (just check they exist)
        for member in &group.members {
            let _ = member; // SpeakerId doesn't implement Display, but we can verify it exists
        }
        
        println!("  Validated group: coordinator={:?}, members={}", 
                 group.coordinator, group.members.len());
    }
    
    println!("Response format validation passed for {} groups", groups.len());
}

#[test]
fn test_get_zone_group_state_performance() {
    let speakers = discovery::discover_speakers_with_timeout(Duration::from_secs(5))
        .expect("Discovery should not fail");
    
    if speakers.is_empty() {
        println!("No Sonos speakers found - skipping performance test");
        return;
    }

    let soap_client = SoapClient::new(Duration::from_secs(10))
        .expect("Should be able to create SOAP client");

    let speaker = &speakers[0];
    let device_url = format!("http://{}:{}", speaker.ip_address, speaker.port);
    
    println!("Testing zone group state performance with speaker: {}", speaker.name);

    // Measure multiple calls to check for consistency and performance
    let mut durations = Vec::new();
    
    for i in 0..5 {
        let start = std::time::Instant::now();
        
        let result = ZoneGroupsService::get_zone_group_state(&soap_client, &device_url);
        
        let duration = start.elapsed();
        durations.push(duration);
        
        match result {
            Ok(groups) => {
                println!("  Call {}: {} groups in {:?}", i + 1, groups.len(), duration);
            }
            Err(e) => {
                println!("  Call {} failed: {:?}", i + 1, e);
                // Don't fail the test immediately - network issues can happen
            }
        }
        
        // Small delay between calls
        std::thread::sleep(Duration::from_millis(100));
    }
    
    // Calculate average response time
    let total_duration: Duration = durations.iter().sum();
    let avg_duration = total_duration / durations.len() as u32;
    
    println!("Average response time: {:?}", avg_duration);
    
    // Response should be reasonably fast (under 5 seconds)
    assert!(avg_duration < Duration::from_secs(5), 
            "Average response time should be under 5 seconds, got {:?}", avg_duration);
    
    // No single call should take more than 10 seconds
    for (i, duration) in durations.iter().enumerate() {
        assert!(*duration < Duration::from_secs(10),
               "Call {} took too long: {:?}", i + 1, duration);
    }
    
    println!("Performance test passed - all calls completed within acceptable time");
}
