use sonos::{discover_speakers_with_timeout, EventStream, ServiceType, SonosError, StreamConfig};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎵 Sonos Stream All Speakers Example 🎵");
    println!("═══════════════════════════════════════════════════════════════");

    // Step 1: Find all speakers on the network
    println!("🔍 Discovering all Sonos speakers on the network...");
    let speakers = match discover_speakers_with_timeout(Duration::from_secs(5)) {
        Ok(speakers) => speakers,
        Err(SonosError::DiscoveryFailed(msg)) => {
            println!("❌ Discovery failed: {}", msg);
            println!("💡 Make sure you're on the same network as your Sonos speakers");
            return Ok(());
        }
        Err(e) => return Err(Box::new(e)),
    };

    if speakers.is_empty() {
        println!("❌ No Sonos speakers found on the network.");
        println!("💡 Make sure your Sonos speakers are powered on and connected to the network");
        return Ok(());
    }

    println!("✅ Found {} speakers:", speakers.len());
    for (i, speaker) in speakers.iter().enumerate() {
        println!(
            "   {}. {} ({}) at {}:{}",
            i + 1,
            speaker.name,
            speaker.model_name,
            speaker.ip_address,
            speaker.port
        );
        if !speaker.satellites.is_empty() {
            println!("      └─ Satellites: {:?}", speaker.satellites);
        }
    }

    // Step 2: Configure the stream with comprehensive settings
    let config = StreamConfig {
        enabled_services: vec![ServiceType::AVTransport, ServiceType::RenderingControl],
        callback_port_range: (8080, 8090),
        subscription_timeout: Duration::from_secs(1800), // 30 minutes
        retry_attempts: 3,
        retry_backoff: Duration::from_secs(1),
        buffer_size: 2000,
    };

    println!("\n🔧 Stream Configuration:");
    println!("   • Services: {:?}", config.enabled_services);
    println!("   • Callback ports: {:?}", config.callback_port_range);
    println!(
        "   • Subscription timeout: {:?}",
        config.subscription_timeout
    );
    println!("   • Buffer size: {}", config.buffer_size);

    // Step 3: Start the event stream with all speakers
    println!("\n🚀 Starting event stream for all speakers...");
    let event_stream = match EventStream::new(speakers.clone(), config) {
        Ok(stream) => {
            println!("✅ Event stream created successfully!");
            stream
        }
        Err(e) => {
            println!("❌ Failed to create event stream: {:?}", e);
            println!("💡 This might happen if speakers are offline or unreachable");
            return Ok(());
        }
    };

    // Verify the stream is active
    if event_stream.is_active() {
        println!("✅ Event stream is active and ready to receive events");
    } else {
        println!("⚠️  Event stream may not be fully active");
    }

    println!("\n📡 Listening for events from all speakers...");
    println!("   Press Ctrl+C to stop");
    println!("   Try playing/pausing music, changing volume, or grouping speakers!");
    println!("═══════════════════════════════════════════════════════════════\n");

    // Step 4: Iterate over the event stream as a user would
    let mut event_count = 0;
    let start_time = std::time::Instant::now();

    // Give subscriptions time to establish
    println!("⏳ Waiting 3 seconds for subscriptions to establish...");
    std::thread::sleep(Duration::from_secs(3));

    // Main event processing loop - this is how a user would iterate over events
    loop {
        // Method 1: Using recv_timeout (recommended for most use cases)
        match event_stream.recv_timeout(Duration::from_millis(1000)) {
            Some(event) => {
                event_count += 1;
                println!("🎵 Event #{}: {}", event_count, format_event(&event));

                // Show periodic separator for readability
                if event_count % 5 == 0 {
                    println!("   ─────────────────────────────────────────────────");
                }
            }
            None => {
                // No event received within timeout - show status
                let elapsed = start_time.elapsed();
                if elapsed.as_secs() % 10 == 0 && elapsed.as_millis() % 1000 < 100 {
                    println!(
                        "⏳ Listening... ({}s elapsed, {} events received)",
                        elapsed.as_secs(),
                        event_count
                    );
                }

                // Show tips after 30 seconds with no events
                if elapsed.as_secs() == 30 && event_count == 0 {
                    show_troubleshooting_tips();
                }
            }
        }

        // Alternative method: Using try_recv (non-blocking)
        // This is useful when you want to do other work in the same thread
        /*
        if let Some(event) = event_stream.try_recv() {
            event_count += 1;
            println!("🎵 Event #{}: {}", event_count, format_event(&event));
        } else {
            // Do other work here since try_recv doesn't block
            std::thread::sleep(Duration::from_millis(100));
        }
        */

        // Alternative method: Using the iterator (blocking)
        // This is the most straightforward way but blocks the thread
        /*
        for event in event_stream.iter() {
            event_count += 1;
            println!("🎵 Event #{}: {}", event_count, format_event(&event));
        }
        */
    }
}

/// Format a StateChange event for display with detailed information
fn format_event(event: &sonos::StateChange) -> String {
    match event {
        sonos::StateChange::PlaybackStateChanged { speaker_id, state } => {
            let (state_icon, state_desc) = match state {
                sonos::PlaybackState::Playing => ("▶️", "Started playing"),
                sonos::PlaybackState::Paused => ("⏸️", "Paused"),
                sonos::PlaybackState::Stopped => ("⏹️", "Stopped"),
                sonos::PlaybackState::Transitioning => ("🔄", "Transitioning"),
            };
            format!("{} {} (Speaker: {:?})", state_icon, state_desc, speaker_id)
        }
        sonos::StateChange::VolumeChanged { speaker_id, volume } => {
            let volume_bar = create_volume_bar(*volume);
            let volume_icon = match *volume {
                0 => "🔇",
                1..=33 => "🔈",
                34..=66 => "🔉",
                _ => "🔊",
            };
            format!(
                "{} Volume changed to {}% {} (Speaker: {:?})",
                volume_icon, volume, volume_bar, speaker_id
            )
        }
        sonos::StateChange::MuteChanged { speaker_id, muted } => {
            let (mute_icon, mute_desc) = if *muted {
                ("🔇", "Muted")
            } else {
                ("🔊", "Unmuted")
            };
            format!("{} {} (Speaker: {:?})", mute_icon, mute_desc, speaker_id)
        }
        sonos::StateChange::TrackChanged {
            speaker_id,
            track_info,
        } => {
            if let Some(track) = track_info {
                let artist = track.artist.as_deref().unwrap_or("Unknown Artist");
                let title = track.title.as_deref().unwrap_or("Unknown Title");
                let album = track.album.as_deref().unwrap_or("Unknown Album");
                format!(
                    "🎶 Now playing: \"{}\" by {} from {} (Speaker: {:?})",
                    title, artist, album, speaker_id
                )
            } else {
                format!("🎶 Track info cleared (Speaker: {:?})", speaker_id)
            }
        }
        sonos::StateChange::PositionChanged {
            speaker_id,
            position_ms,
        } => {
            let position_secs = position_ms / 1000;
            let minutes = position_secs / 60;
            let seconds = position_secs % 60;
            format!(
                "⏱️  Position: {:02}:{:02} (Speaker: {:?})",
                minutes, seconds, speaker_id
            )
        }
        sonos::StateChange::GroupTopologyChanged { groups } => {
            let total_speakers: usize = groups.iter().map(|g| g.members.len()).sum();
            format!(
                "👥 Group topology changed: {} groups with {} total speakers",
                groups.len(),
                total_speakers
            )
        }
        sonos::StateChange::TransportInfoChanged {
            speaker_id,
            transport_state,
            transport_status,
        } => {
            let state_icon = match transport_state {
                sonos::PlaybackState::Playing => "▶️",
                sonos::PlaybackState::Paused => "⏸️",
                sonos::PlaybackState::Stopped => "⏹️",
                sonos::PlaybackState::Transitioning => "🔄",
            };
            format!(
                "{} Transport state: {:?}, Status: {:?} (Speaker: {:?})",
                state_icon, transport_state, transport_status, speaker_id
            )
        }
        sonos::StateChange::SubscriptionError {
            speaker_id,
            service,
            error,
        } => {
            format!(
                "⚠️  Subscription error for {:?} service on speaker {:?}: {}",
                service, speaker_id, error
            )
        }
    }
}

/// Create a visual volume bar representation
fn create_volume_bar(volume: u8) -> String {
    let bar_length = 10;
    let filled = ((volume as f32 / 100.0) * bar_length as f32).round() as usize;
    let filled = filled.min(bar_length);
    let empty = bar_length - filled;

    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

/// Show troubleshooting tips when no events are received
fn show_troubleshooting_tips() {
    println!("\n🔍 No events received after 30 seconds. Troubleshooting tips:");
    println!("   1. Make sure your Sonos speakers are on the same network");
    println!("   2. Try playing/pausing music on any speaker");
    println!("   3. Try changing the volume on any speaker");
    println!("   4. Check if firewall is blocking ports 8080-8090");
    println!("   5. Some speakers may be satellites that don't send events");
    println!("   6. Try grouping/ungrouping speakers");
    println!("   7. Make sure speakers are not in sleep mode");
    println!("\n   💡 Events are most common during active playback");
    println!("   💡 Volume and mute changes also generate events");
    println!("   💡 Group changes generate topology events");
    println!("\n   Continuing to listen for events...\n");
}
