use sonos::{
    discover_speakers_with_timeout, get_zone_groups_from_speaker, streaming::EventStreamBuilder,
    PlaybackState, SonosError, StateCache,
};
use std::sync::Arc;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎵 Simple Sonos State Monitor");
    println!("Discovering speakers...");

    // Discover speakers
    let speakers = match discover_speakers_with_timeout(Duration::from_secs(2)) {
        Ok(speakers) if !speakers.is_empty() => speakers,
        Ok(_) | Err(SonosError::DiscoveryFailed(_)) => {
            println!("No Sonos speakers found on the network.");
            return Ok(());
        }
        Err(e) => return Err(Box::new(e)),
    };

    println!("Found {} speakers", speakers.len());
    for (i, speaker) in speakers.iter().enumerate() {
        println!("  {}. {} ({}:{})", i + 1, speaker.name, speaker.ip_address, speaker.port);
    }

    // Get zone groups
    let groups = get_zone_groups_from_speaker(&speakers[0]).unwrap_or_else(|e| {
        println!("Warning: Failed to fetch groups: {:?}", e);
        vec![]
    });

    println!("Found {} groups", groups.len());

    // Initialize state cache
    let state_cache = Arc::new(StateCache::new());
    state_cache.initialize(speakers.clone(), groups);

    println!("State cache initialized");

    // Use only the first speaker to avoid connectivity issues
    let test_speakers = vec![speakers[0].clone()];
    println!("Using 1 speaker for testing: {}", test_speakers[0].name);

    // Setup event streaming
    match EventStreamBuilder::new(test_speakers) {
        Ok(builder) => {
            println!("EventStreamBuilder created successfully");

            let event_count = Arc::new(std::sync::atomic::AtomicU32::new(0));

            println!("🚀 Starting event stream...");
            match builder
                .with_state_cache(state_cache.clone())
                .with_event_handler(move |event| {
                    let count = event_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                    println!("🎯 Event #{}: {:?}", count, event);
                })
                .start()
            {
                Ok(_stream) => {
                    println!("✅ Event streaming started successfully!");

                    // Display initial topology
                    display_topology(&state_cache);

                    println!("⏳ Monitoring for 30 seconds...");

                    // Run for 30 seconds
                    for i in 1..=30 {
                        std::thread::sleep(Duration::from_secs(1));
                        if i % 5 == 0 {
                            println!("⏰ {} seconds elapsed", i);
                            display_topology(&state_cache);
                        }
                    }

                    println!("🏁 Monitoring complete!");
                }
                Err(e) => {
                    println!("⚠️  Streaming failed: {:?}", e);
                    display_topology(&state_cache);
                }
            }
        }
        Err(e) => {
            println!("⚠️  Failed to create event stream: {:?}", e);
            display_topology(&state_cache);
        }
    }

    Ok(())
}

fn display_topology(state_cache: &Arc<StateCache>) {
    let groups = state_cache.get_all_groups();
    let all_speakers = state_cache.get_all_speakers();

    println!("📊 Current Topology:");
    
    if groups.is_empty() {
        println!("  No groups found");
        for speaker in &all_speakers {
            println!("  🔊 {} - {} - {}", 
                speaker.speaker.room_name,
                format_playback_state(speaker.playback_state),
                format_volume(speaker.volume, speaker.muted)
            );
        }
        return;
    }

    for (i, group) in groups.iter().enumerate() {
        let group_speakers = state_cache.get_speakers_in_group(group.id);

        if group_speakers.len() > 1 {
            println!("  🏠 Group {} ({} speakers)", i + 1, group_speakers.len());
            for speaker in &group_speakers {
                let role = if speaker.is_coordinator { " 👑" } else { "" };
                println!("    🔊 {}{} - {} - {}",
                    speaker.speaker.room_name,
                    role,
                    format_playback_state(speaker.playback_state),
                    format_volume(speaker.volume, speaker.muted)
                );
            }
        } else if let Some(speaker) = group_speakers.first() {
            println!("  🔊 {} (Solo) - {} - {}",
                speaker.speaker.room_name,
                format_playback_state(speaker.playback_state),
                format_volume(speaker.volume, speaker.muted)
            );
        }
    }

    let playing_count = all_speakers
        .iter()
        .filter(|s| s.playback_state == PlaybackState::Playing)
        .count();

    println!("  📈 Summary: {} speakers, {} playing", all_speakers.len(), playing_count);
}

fn format_playback_state(state: PlaybackState) -> String {
    match state {
        PlaybackState::Playing => "▶️ Playing".to_string(),
        PlaybackState::Paused => "⏸️ Paused".to_string(),
        PlaybackState::Stopped => "⏹️ Stopped".to_string(),
        PlaybackState::Transitioning => "🔄 Transitioning".to_string(),
    }
}

fn format_volume(volume: u8, muted: bool) -> String {
    if muted {
        format!("🔇 {}%", volume)
    } else {
        let icon = match volume {
            0 => "🔈",
            1..=33 => "🔉",
            _ => "🔊",
        };
        format!("{} {}%", icon, volume)
    }
}