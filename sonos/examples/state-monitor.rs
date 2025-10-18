use sonos::{
    discover_speakers_with_timeout, get_zone_groups_from_speaker,
    streaming::{EventStream, ServiceType, StreamConfig},
    Group, PlaybackState, SonosError, SpeakerState, StateCache,
};
use std::io::{self, Write};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Discovering Sonos speakers...");

    // Discover speakers with timeout
    let speakers = match discover_speakers_with_timeout(Duration::from_secs(1)) {
        Ok(speakers) => speakers,
        Err(SonosError::DiscoveryFailed(_)) => {
            println!("No Sonos speakers found on the network.");
            return Ok(());
        }
        Err(e) => return Err(Box::new(e)),
    };

    if speakers.is_empty() {
        println!("No Sonos speakers found on the network.");
        return Ok(());
    }

    println!("Found {} speakers:", speakers.len());
    for speaker in &speakers {
        println!(
            "  - {} ({}) at {}",
            speaker.name, speaker.model_name, speaker.ip_address
        );
    }

    // Get real groups from the first speaker
    println!("\nFetching zone groups...");
    let groups = match get_zone_groups_from_speaker(&speakers[0]) {
        Ok(groups) => {
            println!("Found {} groups", groups.len());
            for group in &groups {
                println!(
                    "  - Group with {} members (coordinator: {:?})",
                    group.members.len(),
                    group.coordinator
                );
            }
            groups
        }
        Err(e) => {
            println!("Failed to fetch groups from {}: {:?}", speakers[0].name, e);
            println!("Continuing with empty groups...");
            vec![]
        }
    };

    println!("\nInitializing state monitor with real-time streaming...\n");

    // Initialize state cache
    let state_cache = Arc::new(StateCache::new());

    // Initialize the cache with discovered speakers and real groups
    state_cache.initialize(speakers.clone(), groups);

    // Initialize streaming system
    println!("Setting up real-time event streaming...");

    // Create a configuration with only AVTransport (RenderingControl not implemented yet)
    let stream_config =
        StreamConfig::default().with_enabled_services(vec![ServiceType::AVTransport]);

    // Validate configuration
    if let Err(e) = stream_config.validate() {
        println!("⚠️  Invalid streaming configuration: {}", e);
        println!("   Using default configuration instead...");
        let stream_config = StreamConfig::default();
        if let Err(e) = stream_config.validate() {
            println!("❌ Default configuration is also invalid: {}", e);
            return Ok(());
        }
    }

    println!("📋 Streaming configuration:");
    println!("   - Buffer size: {}", stream_config.buffer_size);
    println!(
        "   - Subscription timeout: {:?}",
        stream_config.subscription_timeout
    );
    println!("   - Retry attempts: {}", stream_config.retry_attempts);
    println!(
        "   - Enabled services: {:?}",
        stream_config.enabled_services
    );
    println!(
        "   - Callback port range: {:?}",
        stream_config.callback_port_range
    );

    match EventStream::new(speakers, stream_config) {
        Ok(event_stream) => {
            println!("✅ Event streaming initialized successfully");
            println!("🎵 Now monitoring real-time changes from your Sonos speakers!");
            println!("   Try playing/pausing music on your speakers to see live updates.\n");

            // Start automatic state updates with event notifications
            let (_update_handle, event_notifications) =
                event_stream.start_state_updates_with_notifications(state_cache.clone());

            // Monitor state with event-driven updates
            monitor_state_with_event_updates(&state_cache, event_notifications)?;
        }
        Err(e) => {
            println!("⚠️  Failed to initialize streaming: {:?}", e);
            println!("   Falling back to static state monitoring...\n");

            // Fallback to original monitoring with simulated changes
            monitor_state(&state_cache)?;
        }
    }

    Ok(())
}

fn monitor_state_with_event_updates(
    state_cache: &Arc<StateCache>,
    event_rx: mpsc::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut event_count = 0;
    let start_time = std::time::Instant::now();
    let mut last_event_time = std::time::Instant::now();

    // Display initial state
    display_current_state(state_cache, event_count, start_time, last_event_time)?;

    println!("⏳ Waiting for events from Sonos devices...");
    println!("   This may take a few moments as subscriptions are established.");
    println!("   Try playing/pausing music to trigger events.\n");

    // Wait for events and update display immediately - no polling!
    loop {
        match event_rx.recv() {
            Ok(()) => {
                // Event received, update display immediately
                event_count += 1;
                last_event_time = std::time::Instant::now();
                display_current_state(state_cache, event_count, start_time, last_event_time)?;
            }
            Err(mpsc::RecvError) => {
                println!("Event stream disconnected.");
                break;
            }
        }
    }

    Ok(())
}

fn display_current_state(
    state_cache: &Arc<StateCache>,
    event_count: u32,
    start_time: std::time::Instant,
    last_event_time: std::time::Instant,
) -> Result<(), Box<dyn std::error::Error>> {
    // Clear screen and move cursor to top
    print!("\x1B[2J\x1B[H");
    io::stdout().flush()?;

    // Display header
    println!("🎵 Sonos State Monitor - LIVE STREAMING 🎵");
    println!("Press Ctrl+C to exit");
    println!("Real-time event-driven updates from your Sonos speakers");
    println!(
        "Events received: {} | Last event: {:.1}s ago | Running: {:.1}s",
        event_count,
        last_event_time.elapsed().as_secs_f32(),
        start_time.elapsed().as_secs_f32()
    );
    println!("═══════════════════════════════════════════════════════════════");

    // Get current state
    let groups = state_cache.get_all_groups();
    let all_speakers = state_cache.get_all_speakers();

    if groups.is_empty() {
        println!("No groups found.");
    } else {
        display_groups_and_speakers(state_cache, &groups);
    }

    // Display ungrouped speakers
    display_ungrouped_speakers(&all_speakers, &groups);

    // Display summary
    println!("\n📈 Summary:");
    println!("├─ Total Speakers: {}", all_speakers.len());
    println!("├─ Total Groups: {}", groups.len());
    let playing_count = all_speakers
        .iter()
        .filter(|s| s.playback_state == PlaybackState::Playing)
        .count();
    println!("└─ Currently Playing: {}", playing_count);

    println!("\n📡 Streaming Status: Active (event-driven updates)");
    println!("💡 Tip: Play/pause music on your Sonos speakers to see instant updates!");

    Ok(())
}

fn monitor_state(state_cache: &Arc<StateCache>) -> Result<(), Box<dyn std::error::Error>> {
    let mut counter = 0;

    loop {
        // Clear screen and move cursor to top
        print!("\x1B[2J\x1B[H");
        io::stdout().flush()?;

        // Display header
        println!("🎵 Sonos State Monitor (Update #{}) 🎵", counter);
        println!("Press Ctrl+C to exit");
        println!("═══════════════════════════════════════════════════════════════");

        // Get current state
        let groups = state_cache.get_all_groups();
        let all_speakers = state_cache.get_all_speakers();

        if groups.is_empty() {
            println!("No groups found.");
        } else {
            display_groups_and_speakers(state_cache, &groups);
        }

        // Display ungrouped speakers
        display_ungrouped_speakers(&all_speakers, &groups);

        // Display summary
        println!("\n📈 Summary:");
        println!("├─ Total Speakers: {}", all_speakers.len());
        println!("├─ Total Groups: {}", groups.len());
        let playing_count = all_speakers
            .iter()
            .filter(|s| s.playback_state == PlaybackState::Playing)
            .count();
        println!("└─ Currently Playing: {}", playing_count);

        counter += 1;
        thread::sleep(Duration::from_secs(2));
    }
}

fn display_groups_and_speakers(state_cache: &Arc<StateCache>, groups: &[Group]) {
    println!("📊 Groups and Speakers:");

    for (group_idx, group) in groups.iter().enumerate() {
        let group_speakers = state_cache.get_speakers_in_group(group.id);
        let is_last_group = group_idx == groups.len() - 1;

        if group_speakers.len() > 1 {
            // Multi-speaker group
            println!(
                "├─ 🏠 Group {} ({} speakers)",
                group_idx + 1,
                group_speakers.len()
            );

            for (i, speaker_state) in group_speakers.iter().enumerate() {
                let is_last_speaker = i == group_speakers.len() - 1;
                let speaker_prefix = if is_last_speaker { "└─" } else { "├─" };
                let role = if speaker_state.is_coordinator {
                    " 👑"
                } else {
                    ""
                };

                println!(
                    "│  {} 🔊 {}{}",
                    speaker_prefix, speaker_state.speaker.room_name, role
                );

                let detail_indent = if is_last_speaker {
                    "│     "
                } else {
                    "│  │  "
                };
                display_speaker_details(&speaker_state, detail_indent);
            }
        } else if let Some(speaker_state) = group_speakers.first() {
            // Single speaker group
            println!("├─ 🔊 {} (Solo)", speaker_state.speaker.room_name);
            display_speaker_details(&speaker_state, "│  ");
        }

        if !is_last_group {
            println!("│");
        }
    }
}

fn display_ungrouped_speakers(all_speakers: &[SpeakerState], groups: &[Group]) {
    let grouped_speaker_ids: std::collections::HashSet<_> =
        groups.iter().flat_map(|g| g.all_speaker_ids()).collect();

    let ungrouped: Vec<_> = all_speakers
        .iter()
        .filter(|s| !grouped_speaker_ids.contains(&s.speaker.id))
        .collect();

    if !ungrouped.is_empty() {
        println!("🔍 Ungrouped Speakers:");
        for speaker_state in ungrouped {
            println!("├─ 🔊 {}", speaker_state.speaker.room_name);
            display_speaker_details(&speaker_state, "│  ");
        }
    }
}

fn display_speaker_details(speaker_state: &SpeakerState, indent: &str) {
    let playback_icon = match speaker_state.playback_state {
        PlaybackState::Playing => "▶️",
        PlaybackState::Paused => "⏸️",
        PlaybackState::Stopped => "⏹️",
        PlaybackState::Transitioning => "🔄",
    };

    let mute_status = if speaker_state.muted { " (Muted)" } else { "" };
    let volume_bar = create_volume_bar(speaker_state.volume);

    println!(
        "{}State: {} {:?}",
        indent, playback_icon, speaker_state.playback_state
    );
    println!(
        "{}Volume: {}% {}{}",
        indent, speaker_state.volume, volume_bar, mute_status
    );
    println!("{}Model: {}", indent, speaker_state.speaker.model_name);
    println!("{}IP: {}", indent, speaker_state.speaker.ip_address);
}

fn create_volume_bar(volume: u8) -> String {
    let bar_length = 10;
    let filled = (volume as f32 / 100.0 * bar_length as f32) as usize;
    let empty = bar_length - filled;

    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}
