use sonos::{
    discover_speakers_with_timeout, streaming::EventStreamBuilder,
    SonosError,
};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Debug: What events are we actually receiving?");
    
    // Discover speakers
    let speakers = match discover_speakers_with_timeout(Duration::from_secs(3)) {
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

    // Use only the first speaker for focused debugging
    let test_speakers = vec![speakers[0].clone()];
    let speaker = &test_speakers[0];
    
    println!("\n🎯 Monitoring events for: {} ({}:{})", speaker.name, speaker.ip_address, speaker.port);
    println!("📝 Please play/pause music and change volume to generate events...\n");

    let event_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));

    match EventStreamBuilder::new(test_speakers) {
        Ok(builder) => {
            match builder
                .with_event_handler(move |event| {
                    let count = event_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                    let timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    
                    println!("🎯 Event #{} ({}): {:?}", count, timestamp, event);
                    
                    // Specifically highlight the events we care about
                    match event {
                        sonos::models::StateChange::VolumeChanged { speaker_id, volume } => {
                            println!("   🔊 VOLUME: Speaker {:?} -> {}%", speaker_id, volume);
                        }
                        sonos::models::StateChange::PlaybackStateChanged { speaker_id, state } => {
                            println!("   ▶️  PLAYBACK: Speaker {:?} -> {:?}", speaker_id, state);
                        }
                        sonos::models::StateChange::MuteChanged { speaker_id, muted } => {
                            println!("   🔇 MUTE: Speaker {:?} -> {}", speaker_id, if muted { "MUTED" } else { "UNMUTED" });
                        }
                        sonos::models::StateChange::SubscriptionError { speaker_id, service, error } => {
                            println!("   ❌ ERROR: Speaker {:?}, Service {:?} -> {}", speaker_id, service, error);
                        }
                        _ => {
                            println!("   ℹ️  Other event type");
                        }
                    }
                    println!();
                })
                .start()
            {
                Ok(_stream) => {
                    println!("✅ Event streaming started successfully!");
                    println!("⏳ Monitoring for 30 seconds...");
                    println!("   Try playing/pausing music or changing volume on the speaker");
                    println!("   Press Ctrl+C to stop early\n");

                    // Monitor for 30 seconds
                    for i in 1..=30 {
                        std::thread::sleep(Duration::from_secs(1));
                        if i % 5 == 0 {
                            let current_count = event_count.load(std::sync::atomic::Ordering::Relaxed);
                            println!("⏰ {}s elapsed - {} events received so far", i, current_count);
                        }
                    }

                    let final_count = event_count.load(std::sync::atomic::Ordering::Relaxed);
                    println!("\n🏁 Monitoring complete! Received {} total events", final_count);
                    
                    if final_count == 0 {
                        println!("❌ No events received - this suggests subscription issues");
                        println!("   Possible causes:");
                        println!("   - Speaker is not reachable");
                        println!("   - Firewall blocking callback server");
                        println!("   - Speaker doesn't support UPnP eventing");
                        println!("   - Network connectivity issues");
                    }
                }
                Err(e) => {
                    println!("❌ Streaming failed: {:?}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to create event stream: {:?}", e);
        }
    }

    Ok(())
}