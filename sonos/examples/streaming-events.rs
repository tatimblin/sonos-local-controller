use sonos::{discover_speakers_with_timeout, EventStream, ServiceType, SonosError, StreamConfig};
use std::io::{self, Write};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎵 Sonos Streaming Events Demo 🎵");
    println!("═══════════════════════════════════════════════════════════════");

    // Discover speakers with timeout
    println!("🔍 Discovering Sonos speakers...");
    let speakers = match discover_speakers_with_timeout(Duration::from_secs(1)) {
        Ok(speakers) => speakers,
        Err(SonosError::DiscoveryFailed(_)) => {
            println!("❌ No Sonos speakers found on the network.");
            return Ok(());
        }
        Err(e) => return Err(Box::new(e)),
    };

    if speakers.is_empty() {
        println!("❌ No Sonos speakers found on the network.");
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
    }

    // Select the first speaker for streaming
    let selected_speaker = speakers[0].clone();
    println!(
        "\n🎯 Selected speaker: {} for event streaming",
        selected_speaker.name
    );
    println!("   IP: {}:{}", selected_speaker.ip_address, selected_speaker.port);
    println!("   Model: {}", selected_speaker.model_name);
    
    if speakers.len() > 1 {
        println!("   💡 If no events are received, try restarting to test other speakers");
    }

    // Configure streaming with AVTransport service only
    let config = StreamConfig {
        enabled_services: vec![ServiceType::AVTransport],
        callback_port_range: (8080, 8090),
        subscription_timeout: Duration::from_secs(1800), // 30 minutes
        retry_attempts: 3,
        retry_backoff: Duration::from_secs(1),
        buffer_size: 1000,
    };

    println!("\n🔧 Stream Configuration:");
    println!("   • Enabled services: {:?}", config.enabled_services);
    println!("   • Callback port range: {:?}", config.callback_port_range);
    println!(
        "   • Subscription timeout: {:?}",
        config.subscription_timeout
    );

    // Try to create event stream, testing multiple speakers if needed
    println!("\n🚀 Creating event stream...");
    let mut event_stream = None;
    let mut working_speaker = None;
    
    for (i, speaker) in speakers.iter().enumerate() {
        println!("   Trying speaker {}: {}", i + 1, speaker.name);
        
        match EventStream::new(vec![speaker.clone()], config.clone()) {
            Ok(stream) => {
                println!("   ✅ Successfully created event stream with {}", speaker.name);
                event_stream = Some(stream);
                working_speaker = Some(speaker.clone());
                break;
            }
            Err(e) => {
                println!("   ❌ Failed with {}: {:?}", speaker.name, e);
                if i < speakers.len() - 1 {
                    println!("   Trying next speaker...");
                }
            }
        }
    }
    
    let event_stream = match event_stream {
        Some(stream) => stream,
        None => {
            println!("❌ Could not create event stream with any speaker");
            return Ok(());
        }
    };
    
    let selected_speaker = working_speaker.unwrap();

    println!(
        "\n📡 Listening for events from {}...",
        selected_speaker.name
    );
    println!("   Press Ctrl+C to stop");
    println!("   Try playing/pausing music on your Sonos speaker to see events!");
    println!("   Waiting for initial events (this may take a few seconds)...");
    
    // Check if the event stream is active
    if event_stream.is_active() {
        println!("✅ Event stream is active and ready");
    } else {
        println!("⚠️  Event stream may not be active");
    }
    
    println!("═══════════════════════════════════════════════════════════════\n");

    // Event counter for display
    let mut event_count = 0;
    let mut no_event_count = 0;
    let start_time = std::time::Instant::now();

    // Give subscriptions a moment to establish
    println!("⏳ Waiting 3 seconds for subscriptions to establish...");
    std::thread::sleep(Duration::from_secs(3));
    
    // Main event loop
    loop {
        // Try to receive an event with a longer timeout for better responsiveness
        if let Some(event) = event_stream.recv_timeout(Duration::from_millis(500)) {
            event_count += 1;
            no_event_count = 0; // Reset no-event counter

            // Clear the current line and print event
            print!("\r\x1B[K"); // Clear line
            io::stdout().flush()?;

            println!("🎵 Event #{}: {}", event_count, format_event(&event));

            // Print a separator for readability
            if event_count % 5 == 0 {
                println!("   ─────────────────────────────────────────────────");
            }
        } else {
            no_event_count += 1;
            
            // Show periodic status updates when no events are received
            if no_event_count % 20 == 0 { // Every 10 seconds (500ms * 20)
                let elapsed = start_time.elapsed();
                print!("\r\x1B[K"); // Clear line
                print!("⏳ Waiting for events... ({}s elapsed, {} events received)", 
                    elapsed.as_secs(), event_count);
                io::stdout().flush()?;
            }
            
            // After 30 seconds with no events, show troubleshooting info
            if no_event_count == 60 && event_count == 0 { // 30 seconds
                println!("\n\n🔍 No events received after 30 seconds. Troubleshooting tips:");
                println!("   1. Make sure your Sonos speaker is on the same network");
                println!("   2. Try playing/pausing music on the speaker");
                println!("   3. Check if firewall is blocking ports 8080-8090");
                println!("   4. Some speakers may be satellite/bonded speakers that don't send events");
                println!("   5. The speaker might not be actively playing content");
                println!("   6. Try restarting the example to test with a different speaker");
                println!("   7. Make sure the speaker is not grouped as a satellite");
                println!("\n   💡 Tip: Events are most common when music is playing/pausing");
                println!("   💡 Tip: Volume changes also generate events");
                println!("   💡 Tip: Check the console output above for subscription errors");
                println!("\n   Continuing to listen for events...\n");
            }
        }

        // Check for Ctrl+C (this is a simple approach - in a real app you'd use signal handling)
        // For now, we'll just run indefinitely until the user stops the program
    }
}

/// Format a StateChange event for display
fn format_event(event: &sonos::StateChange) -> String {
    match event {
        sonos::StateChange::PlaybackStateChanged { speaker_id, state } => {
            let state_icon = match state {
                sonos::PlaybackState::Playing => "▶️",
                sonos::PlaybackState::Paused => "⏸️",
                sonos::PlaybackState::Stopped => "⏹️",
                sonos::PlaybackState::Transitioning => "🔄",
            };
            format!(
                "{} Playback: {:?} (Speaker: {:?})",
                state_icon, state, speaker_id
            )
        }
        sonos::StateChange::VolumeChanged { speaker_id, volume } => {
            let volume_bar = create_volume_bar(*volume);
            format!(
                "🔊 Volume: {}% {} (Speaker: {:?})",
                volume, volume_bar, speaker_id
            )
        }
        sonos::StateChange::MuteChanged { speaker_id, muted } => {
            let mute_icon = if *muted { "🔇" } else { "🔊" };
            format!(
                "{} Mute: {} (Speaker: {:?})",
                mute_icon,
                if *muted { "ON" } else { "OFF" },
                speaker_id
            )
        }
        sonos::StateChange::TrackChanged {
            speaker_id,
            track_info,
        } => {
            if let Some(track) = track_info {
                format!(
                    "🎶 Track: {} - {} (Speaker: {:?})",
                    track.artist.as_deref().unwrap_or("Unknown Artist"),
                    track.title.as_deref().unwrap_or("Unknown Title"),
                    speaker_id
                )
            } else {
                format!("🎶 Track: No track info (Speaker: {:?})", speaker_id)
            }
        }
        sonos::StateChange::PositionChanged {
            speaker_id,
            position_ms,
        } => {
            let position_secs = position_ms / 1000;
            format!(
                "⏱️  Position: {:02}:{:02} (Speaker: {:?})",
                position_secs / 60,
                position_secs % 60,
                speaker_id
            )
        }
        sonos::StateChange::GroupTopologyChanged { groups } => {
            format!("👥 Group Topology Changed: {} groups", groups.len())
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
                "{} Transport: {:?} (Status: {:?}, Speaker: {:?})",
                state_icon, transport_state, transport_status, speaker_id
            )
        }
        sonos::StateChange::SubscriptionError {
            speaker_id,
            service,
            error,
        } => {
            format!(
                "⚠️  Subscription Error: {:?} service on speaker {:?} - {}",
                service, speaker_id, error
            )
        }
    }
}

/// Create a visual volume bar
fn create_volume_bar(volume: u8) -> String {
    let bar_length = 10;
    let filled = (volume as f32 / 100.0 * bar_length as f32) as usize;
    let empty = bar_length - filled;

    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}
