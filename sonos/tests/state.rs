use sonos::{discover_speakers_with_timeout, get_zone_groups_from_speaker, SonosError, StateCache};
use std::collections::HashSet;
use std::time::Duration;

/// Integration test that discovers real Sonos speakers and initializes state cache with real groups
#[test]
fn test_initialize_state_with_real_speakers_and_groups() {
    // Discover speakers with a reasonable timeout
    let speakers = match discover_speakers_with_timeout(Duration::from_secs(5)) {
        Ok(speakers) => speakers,
        Err(SonosError::DiscoveryFailed(_)) => {
            println!("No Sonos speakers found on network - skipping test");
            return;
        }
        Err(e) => panic!("Discovery failed with unexpected error: {:?}", e),
    };

    if speakers.is_empty() {
        println!("No Sonos speakers discovered - skipping test");
        return;
    }

    println!("Discovered {} Sonos speakers:", speakers.len());
    for speaker in &speakers {
        println!(
            "  - {} ({}) at {}",
            speaker.name, speaker.model_name, speaker.ip_address
        );
    }

    // Fetch real groups from the first speaker
    let groups = match get_zone_groups_from_speaker(&speakers[0]) {
        Ok(groups) => groups,
        Err(e) => {
            println!("Failed to fetch groups from {}: {:?}", speakers[0].name, e);
            println!("Continuing test with empty groups...");
            vec![]
        }
    };

    println!("Fetched {} groups from Sonos system:", groups.len());
    for group in &groups {
        println!(
            "  - Group {:?} with {} members (coordinator: {:?})",
            group.id,
            group.members.len(),
            group.coordinator
        );
    }

    // Create state cache and initialize with discovered speakers and real groups
    let cache = StateCache::new();
    cache.initialize(speakers.clone(), groups.clone());

    // Verify all speakers were added to the cache
    let cached_speakers = cache.get_all_speakers();
    assert_eq!(cached_speakers.len(), speakers.len());

    // Verify each speaker is properly initialized
    for original_speaker in &speakers {
        let cached_speaker = cache
            .get_speaker(original_speaker.id)
            .expect("Speaker should be found in cache");

        // Verify speaker data matches
        assert_eq!(cached_speaker.speaker.id, original_speaker.id);
        assert_eq!(cached_speaker.speaker.name, original_speaker.name);
        assert_eq!(cached_speaker.speaker.room_name, original_speaker.room_name);
        assert_eq!(
            cached_speaker.speaker.ip_address,
            original_speaker.ip_address
        );
        assert_eq!(
            cached_speaker.speaker.model_name,
            original_speaker.model_name
        );
        assert_eq!(cached_speaker.speaker.udn, original_speaker.udn);
        assert_eq!(cached_speaker.speaker.port, original_speaker.port);

        // Verify default state values
        assert_eq!(cached_speaker.playback_state, sonos::PlaybackState::Stopped);
        assert_eq!(cached_speaker.volume, 0);
        assert_eq!(cached_speaker.muted, false);
        assert_eq!(cached_speaker.position_ms, 0);
        assert_eq!(cached_speaker.duration_ms, 0);

        // Group membership will be set based on real groups
        println!(
            "  Speaker {} - Group: {:?}, Coordinator: {}",
            cached_speaker.speaker.name, cached_speaker.group_id, cached_speaker.is_coordinator
        );
    }

    // Verify groups were added
    let cached_groups = cache.get_all_groups();
    assert_eq!(cached_groups.len(), groups.len());

    // Verify group data
    for original_group in &groups {
        let cached_group = cache
            .get_group(original_group.id)
            .expect("Group should be found in cache");
        assert_eq!(cached_group.coordinator, original_group.coordinator);
        assert_eq!(cached_group.members.len(), original_group.members.len());

        // Verify group members have correct state
        for &member_id in &cached_group.members {
            if let Some(member_state) = cache.get_speaker(member_id) {
                assert_eq!(member_state.group_id, Some(original_group.id));
                assert_eq!(
                    member_state.is_coordinator,
                    member_id == original_group.coordinator
                );
            }
        }
    }

    // Validate that all discovered speakers are accounted for in groups
    let mut speakers_in_groups = HashSet::new();
    for group in &groups {
        for &member_id in &group.members {
            speakers_in_groups.insert(member_id);
        }
    }

    let discovered_speaker_ids: HashSet<_> = speakers.iter().map(|s| s.id).collect();

    // Find speakers that are discovered but not in any group
    let orphaned_speakers: Vec<_> = discovered_speaker_ids
        .difference(&speakers_in_groups)
        .collect();

    // Find group members that weren't discovered
    let missing_speakers: Vec<_> = speakers_in_groups
        .difference(&discovered_speaker_ids)
        .collect();

    // Report the analysis
    println!("Speaker-Group Analysis:");
    println!("  - Discovered speakers: {}", discovered_speaker_ids.len());
    println!("  - Speakers in groups: {}", speakers_in_groups.len());
    println!(
        "  - Orphaned speakers (discovered but not in groups): {}",
        orphaned_speakers.len()
    );
    println!(
        "  - Missing speakers (in groups but not discovered): {}",
        missing_speakers.len()
    );

    if !orphaned_speakers.is_empty() {
        println!("  Orphaned speakers (this is normal if speakers are offline or in different households):");
        for speaker_id in &orphaned_speakers {
            if let Some(speaker) = speakers.iter().find(|s| s.id == **speaker_id) {
                println!("    - {} at {}", speaker.name, speaker.ip_address);
            }
        }
    }

    let missing_count = missing_speakers.len();
    if !missing_speakers.is_empty() {
        println!("  Missing speakers (this could indicate discovery issues):");
        for speaker_id in &missing_speakers {
            println!("    - {:?}", speaker_id);
        }
    }

    // Verify that all group members were discovered (this should always be true)
    assert!(
        missing_speakers.is_empty(),
        "Found {} group members that were not discovered. This indicates a discovery or parsing issue.",
        missing_count
    );

    // Verify that we have at least some speakers in groups (basic sanity check)
    assert!(
        !speakers_in_groups.is_empty(),
        "No speakers found in any groups. This indicates a zone topology parsing issue."
    );

    println!(
        "✓ State cache successfully validated: {} discovered speakers, {} in groups, {} groups total",
        speakers.len(),
        speakers_in_groups.len(),
        groups.len()
    );
}

/// Integration test for speaker lookup methods with real data
#[test]
fn test_speaker_lookup_methods_with_real_data() {
    // Discover speakers
    let speakers = match discover_speakers_with_timeout(Duration::from_secs(5)) {
        Ok(speakers) => speakers,
        Err(SonosError::DiscoveryFailed(_)) => {
            println!("No Sonos speakers found on network - skipping test");
            return;
        }
        Err(e) => panic!("Discovery failed with unexpected error: {:?}", e),
    };

    if speakers.is_empty() {
        println!("No Sonos speakers discovered - skipping test");
        return;
    }

    // Initialize cache
    let cache = StateCache::new();
    cache.initialize(speakers.clone(), vec![]);

    // Test get_by_name
    for speaker in &speakers {
        let found = cache.get_by_name(&speaker.name);
        assert!(
            found.is_some(),
            "Should find speaker by name: {}",
            speaker.name
        );
        let found_speaker = found.unwrap();
        assert_eq!(found_speaker.speaker.id, speaker.id);
        assert_eq!(found_speaker.speaker.name, speaker.name);
    }

    // Test get_by_room
    for speaker in &speakers {
        let room_speakers = cache.get_by_room(&speaker.room_name);
        assert!(
            !room_speakers.is_empty(),
            "Should find speakers in room: {}",
            speaker.room_name
        );

        // Verify the speaker is in the results
        let found = room_speakers.iter().any(|s| s.speaker.id == speaker.id);
        assert!(
            found,
            "Speaker {} should be found in room {}",
            speaker.name, speaker.room_name
        );
    }

    // Test get_speaker by ID
    for speaker in &speakers {
        let found = cache.get_speaker(speaker.id);
        assert!(found.is_some(), "Should find speaker by ID");
        let found_speaker = found.unwrap();
        assert_eq!(found_speaker.speaker.id, speaker.id);
    }

    println!(
        "✓ All speaker lookup methods work correctly with {} real speakers",
        speakers.len()
    );
}

/// Integration test for state updates with real speakers
#[test]
fn test_state_updates_with_real_speakers() {
    // Discover speakers
    let speakers = match discover_speakers_with_timeout(Duration::from_secs(5)) {
        Ok(speakers) => speakers,
        Err(SonosError::DiscoveryFailed(_)) => {
            println!("No Sonos speakers found on network - skipping test");
            return;
        }
        Err(e) => panic!("Discovery failed with unexpected error: {:?}", e),
    };

    if speakers.is_empty() {
        println!("No Sonos speakers discovered - skipping test");
        return;
    }

    // Initialize cache
    let cache = StateCache::new();
    cache.initialize(speakers.clone(), vec![]);

    let test_speaker = &speakers[0];
    println!(
        "Testing state updates with speaker: {} ({})",
        test_speaker.name, test_speaker.model_name
    );

    // Test volume update
    cache.update_volume(test_speaker.id, 50);
    let updated_state = cache.get_speaker(test_speaker.id).unwrap();
    assert_eq!(updated_state.volume, 50);

    // Test mute update
    cache.update_mute(test_speaker.id, true);
    let updated_state = cache.get_speaker(test_speaker.id).unwrap();
    assert_eq!(updated_state.muted, true);

    // Test playback state update
    cache.update_playback_state(test_speaker.id, sonos::PlaybackState::Playing);
    let updated_state = cache.get_speaker(test_speaker.id).unwrap();
    assert_eq!(updated_state.playback_state, sonos::PlaybackState::Playing);

    // Test position update
    cache.update_position(test_speaker.id, 30000);
    let updated_state = cache.get_speaker(test_speaker.id).unwrap();
    assert_eq!(updated_state.position_ms, 30000);

    // Verify other speakers weren't affected
    for other_speaker in &speakers[1..] {
        let other_state = cache.get_speaker(other_speaker.id).unwrap();
        assert_eq!(other_state.volume, 0);
        assert_eq!(other_state.muted, false);
        assert_eq!(other_state.playback_state, sonos::PlaybackState::Stopped);
        assert_eq!(other_state.position_ms, 0);
    }

    println!("✓ State updates work correctly with real speaker data");
}

/// Integration test for cache cloning with real data
#[test]
fn test_cache_cloning_with_real_data() {
    // Discover speakers
    let speakers = match discover_speakers_with_timeout(Duration::from_secs(5)) {
        Ok(speakers) => speakers,
        Err(SonosError::DiscoveryFailed(_)) => {
            println!("No Sonos speakers found on network - skipping test");
            return;
        }
        Err(e) => panic!("Discovery failed with unexpected error: {:?}", e),
    };

    if speakers.is_empty() {
        println!("No Sonos speakers discovered - skipping test");
        return;
    }

    // Initialize cache
    let cache = StateCache::new();
    cache.initialize(speakers.clone(), vec![]);

    // Clone the cache
    let cloned_cache = cache.clone();

    // Verify both caches have the same data
    let original_speakers = cache.get_all_speakers();
    let cloned_speakers = cloned_cache.get_all_speakers();
    assert_eq!(original_speakers.len(), cloned_speakers.len());

    // Verify they share the same underlying data (Arc)
    let test_speaker = &speakers[0];
    cache.update_volume(test_speaker.id, 75);

    let original_state = cache.get_speaker(test_speaker.id).unwrap();
    let cloned_state = cloned_cache.get_speaker(test_speaker.id).unwrap();
    assert_eq!(original_state.volume, 75);
    assert_eq!(cloned_state.volume, 75);

    println!(
        "✓ Cache cloning works correctly with {} real speakers",
        speakers.len()
    );
}
