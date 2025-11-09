use sonos::transport::discovery;
use std::time::Duration;

/// Integration tests that run against real Sonos speakers on the network.
/// These tests will be skipped if no speakers are found.
/// 
/// To run these tests:
/// ```
/// cargo test --test integration_tests
/// ```

#[test]
fn test_discover_real_speakers() {
    let speakers = discovery::discover_speakers_with_timeout(Duration::from_secs(5))
        .expect("Discovery should not fail");
    
    if speakers.is_empty() {
        println!("No Sonos speakers found on network - skipping real speaker tests");
        return;
    }

    println!("Found {} Sonos speakers:", speakers.len());
    for speaker in &speakers {
        println!("  - {} ({}) at {}", speaker.name, speaker.model_name, speaker.ip_address);
        
        // Validate speaker data
        assert!(!speaker.name.is_empty(), "Speaker name should not be empty");
        assert!(!speaker.ip_address.is_empty(), "IP address should not be empty");
        assert!(!speaker.udn.is_empty(), "UDN should not be empty");
        assert_eq!(speaker.port, 1400, "Sonos speakers should use port 1400");
        
        // Validate IP address format (basic check)
        let ip_parts: Vec<&str> = speaker.ip_address.split('.').collect();
        assert_eq!(ip_parts.len(), 4, "IP address should have 4 octets");
        for part in ip_parts {
            let _octet: u8 = part.parse().expect("IP octet should be valid number");
            // u8 is automatically <= 255, so just parsing validates the range
        }
        
        // Validate UDN format (should start with uuid:RINCON_)
        assert!(speaker.udn.starts_with("uuid:RINCON_"), 
                "Sonos UDN should start with 'uuid:RINCON_', got: {}", speaker.udn);
    }
}

#[test]
fn test_discover_speakers_timeout_behavior() {
    // Test with very short timeout
    let start = std::time::Instant::now();
    let result = discovery::discover_speakers_with_timeout(Duration::from_millis(100));
    let elapsed = start.elapsed();
    
    // Should complete within reasonable time even with short timeout
    assert!(elapsed < Duration::from_secs(2), "Discovery should respect timeout bounds");
    
    // Result should be Ok (empty vec is fine with short timeout)
    assert!(result.is_ok(), "Discovery should not fail even with short timeout");
}

#[test]
fn test_discover_speakers_consistency() {
    // Run discovery twice and compare results
    let speakers1 = discovery::discover_speakers_with_timeout(Duration::from_secs(3))
        .expect("First discovery should not fail");
    
    // Wait a moment to avoid network flooding
    std::thread::sleep(Duration::from_millis(500));
    
    let speakers2 = discovery::discover_speakers_with_timeout(Duration::from_secs(3))
        .expect("Second discovery should not fail");
    
    if speakers1.is_empty() && speakers2.is_empty() {
        println!("No speakers found in either discovery - test passed");
        return;
    }
    
    // If we found speakers, the results should be reasonably consistent
    // (allowing for more variation due to network timing and non-Sonos device filtering)
    let diff = (speakers1.len() as i32 - speakers2.len() as i32).abs();
    assert!(diff <= 3, "Speaker count should be reasonably consistent between discoveries (found {} vs {}, diff: {})", 
            speakers1.len(), speakers2.len(), diff);
    
    // Check that we find the same speakers (by UDN)
    let udns1: std::collections::HashSet<_> = speakers1.iter().map(|s| &s.udn).collect();
    let udns2: std::collections::HashSet<_> = speakers2.iter().map(|s| &s.udn).collect();
    
    let common_udns = udns1.intersection(&udns2).count();
    
    println!("Discovery 1 found {} speakers, Discovery 2 found {} speakers, {} in common", 
             speakers1.len(), speakers2.len(), common_udns);
    
    // If we found speakers in both runs, most should be consistent
    // But allow for network timing variations
    if !speakers1.is_empty() && !speakers2.is_empty() {
        let min_speakers = speakers1.len().min(speakers2.len());
        let consistency_ratio = common_udns as f64 / min_speakers as f64;
        assert!(consistency_ratio >= 0.7, 
                "At least 70% of speakers should be found consistently ({}% consistency, {} common out of {} min)", 
                (consistency_ratio * 100.0) as u32, common_udns, min_speakers);
    }
}

#[test]
fn test_speaker_data_quality() {
    let speakers = discovery::discover_speakers_with_timeout(Duration::from_secs(4))
        .expect("Discovery should not fail");
    
    if speakers.is_empty() {
        println!("No speakers found - skipping data quality test");
        return;
    }
    
    for speaker in &speakers {
        // Test that speaker IDs are unique
        let other_speakers: Vec<_> = speakers.iter()
            .filter(|s| s.id == speaker.id && s.udn != speaker.udn)
            .collect();
        assert!(other_speakers.is_empty(), 
                "Speaker ID should be unique (found duplicate ID for different UDNs)");
        
        // Test that UDNs are unique
        let duplicate_udns: Vec<_> = speakers.iter()
            .filter(|s| s.udn == speaker.udn)
            .collect();
        assert_eq!(duplicate_udns.len(), 1, 
                   "Each UDN should appear exactly once (found {} instances of {})", 
                   duplicate_udns.len(), speaker.udn);
        
        // Test reasonable field lengths
        assert!(speaker.name.len() <= 100, "Speaker name should be reasonable length");
        assert!(speaker.room_name.len() <= 100, "Room name should be reasonable length");
        assert!(speaker.model_name.len() <= 50, "Model name should be reasonable length");
    }
}