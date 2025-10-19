use sonos::{
    discover_speakers_with_timeout, get_zone_groups_from_speaker, streaming::EventStreamBuilder,
    PlaybackState, SonosError, SpeakerState, StateCache,
};
use std::io::{self, Write};
use std::sync::Arc;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎵 Sonos Topology Monitor - Event-Driven Visualization");
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

    // Get zone groups
    let groups = get_zone_groups_from_speaker(&speakers[0]).unwrap_or_else(|e| {
        println!("Warning: Failed to fetch groups: {:?}", e);
        vec![]
    });

    // Initialize state cache
    let state_cache = Arc::new(StateCache::new());
    state_cache.initialize(speakers.clone(), groups);

    // Setup event streaming with new simplified interface
    match EventStreamBuilder::new(speakers) {
        Ok(builder) => {
            // Create shared state for tracking events
            let event_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
            let start_time = std::time::Instant::now();

            let state_cache_for_handler = state_cache.clone();
            let event_count_for_handler = event_count.clone();

            match builder
                .with_state_cache(state_cache.clone())
                .with_event_handler(move |_event| {
                    // Increment event count and trigger display update
                    let count = event_count_for_handler
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                        + 1;
                    let _ =
                        display_topology_with_stats(&state_cache_for_handler, count, start_time);
                })
                .start()
            {
                Ok(_stream) => {
                    println!("✅ Event streaming active - monitoring topology changes\n");

                    // Display initial topology
                    display_topology_with_stats(&state_cache, 0, start_time)?;

                    println!("⏳ Waiting for topology changes...");
                    println!("   Try playing/pausing music or grouping speakers\n");

                    // Keep the stream alive - no manual event processing needed!
                    loop {
                        std::thread::sleep(Duration::from_secs(1));
                    }
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

fn display_topology_with_stats(
    state_cache: &Arc<StateCache>,
    event_count: u32,
    start_time: std::time::Instant,
) -> Result<(), Box<dyn std::error::Error>> {
    // Clear screen
    print!("\x1B[2J\x1B[H");
    io::stdout().flush()?;

    println!("🎵 Sonos Topology Monitor - LIVE");
    println!(
        "Events: {} | Runtime: {:.1}s | Press Ctrl+C to exit",
        event_count,
        start_time.elapsed().as_secs_f32()
    );
    println!("═══════════════════════════════════════════════════");

    display_topology(state_cache);

    println!("\n💡 Tip: Play/pause music or group speakers to see updates!");
    Ok(())
}

fn display_topology(state_cache: &Arc<StateCache>) {
    let groups = state_cache.get_all_groups();
    let all_speakers = state_cache.get_all_speakers();

    if groups.is_empty() {
        println!("📊 No groups found");
        display_all_speakers(&all_speakers);
        return;
    }

    println!("📊 Topology ({} groups):", groups.len());

    for (i, group) in groups.iter().enumerate() {
        let group_speakers = state_cache.get_speakers_in_group(group.id);

        if group_speakers.len() > 1 {
            println!("├─ 🏠 Group {} ({} speakers)", i + 1, group_speakers.len());
            for (j, speaker) in group_speakers.iter().enumerate() {
                let is_last = j == group_speakers.len() - 1;
                let prefix = if is_last { "└─" } else { "├─" };
                let role = if speaker.is_coordinator { " 👑" } else { "" };

                println!(
                    "│  {} 🔊 {}{} - {} - {}",
                    prefix,
                    speaker.speaker.room_name,
                    role,
                    format_playback_state(speaker.playback_state),
                    format_volume(speaker.volume, speaker.muted)
                );
            }
        } else if let Some(speaker) = group_speakers.first() {
            println!(
                "├─ 🔊 {} (Solo) - {} - {}",
                speaker.speaker.room_name,
                format_playback_state(speaker.playback_state),
                format_volume(speaker.volume, speaker.muted)
            );
        }
    }

    // Summary
    let playing_count = all_speakers
        .iter()
        .filter(|s| s.playback_state == PlaybackState::Playing)
        .count();

    println!(
        "\n📈 Summary: {} speakers, {} playing",
        all_speakers.len(),
        playing_count
    );
}

fn display_all_speakers(speakers: &[SpeakerState]) {
    println!("🔊 All Speakers:");
    for speaker in speakers {
        println!(
            "├─ {} - {} - {}",
            speaker.speaker.room_name,
            format_playback_state(speaker.playback_state),
            format_volume(speaker.volume, speaker.muted)
        );
    }
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
