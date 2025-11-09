use sonos::{
    discover_speakers_with_timeout, get_zone_groups_from_speaker, streaming::EventStreamBuilder,
    PlaybackState, SonosError, SpeakerState, StateCache,
};
use std::io::{self, Write};
use std::sync::Arc;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎵 Debug Sonos State Monitor");
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
    for speaker in &speakers {
        println!("  - {} ({}:{})", speaker.name, speaker.ip_address, speaker.port);
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

    // Setup event streaming with debugging
    match EventStreamBuilder::new(speakers) {
        Ok(builder) => {
            println!("EventStreamBuilder created successfully");

            match builder
                .with_state_cache(state_cache.clone())
                .with_event_handler(move |event| {
                    // Avoid blocking operations in event handler
                    println!("🎯 Received event: {:?}", event);
                })
                .start()
            {
                Ok(_stream) => {
                    println!("✅ Event streaming started successfully");

                    // Display initial topology
                    display_topology(&state_cache);

                    println!("⏳ Waiting for events... (will exit after 30 seconds)");

                    // Wait for 30 seconds then exit
                    for i in 1..=30 {
                        std::thread::sleep(Duration::from_secs(1));
                        if i % 5 == 0 {
                            println!("⏰ {} seconds elapsed...", i);
                        }
                    }

                    println!("🏁 Exiting after 30 seconds");
                }
                Err(e) => {
                    println!("⚠️  Streaming failed: {:?}", e);
                    println!("Displaying static topology...\n");
                    display_topology(&state_cache);
                }
            }
        }
        Err(e) => {
            println!("⚠️  Failed to create event stream: {:?}", e);
            println!("Displaying static topology...\n");
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