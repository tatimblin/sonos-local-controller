use sonos::{
    discover_speakers_with_timeout, get_zone_groups_from_speaker, streaming::EventStreamBuilder,
    PlaybackState, SonosError, StateCache,
};
use std::sync::Arc;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing State Cache Updates");
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

    // Use only the first speaker
    let test_speakers = vec![speakers[0].clone()];
    let speaker_id = test_speakers[0].id;
    
    println!("Testing with speaker: {} (ID: {:?})", test_speakers[0].name, speaker_id);

    // Get initial state
    let initial_state = state_cache.get_speaker(speaker_id);
    println!("Initial state: {:?}", initial_state);

    // Setup event streaming with detailed logging
    let event_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let state_cache_clone = Arc::clone(&state_cache);

    match EventStreamBuilder::new(test_speakers) {
        Ok(builder) => {
            match builder
                .with_state_cache(state_cache.clone())
                .with_event_handler(move |event| {
                    let count = event_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                    println!("🎯 Event #{}: {:?}", count, event);
                    
                    // Check state cache after each event
                    if let Some(updated_state) = state_cache_clone.get_speaker(speaker_id) {
                        println!("   📊 Updated state: Volume={}, Muted={}, PlaybackState={:?}", 
                            updated_state.volume, updated_state.muted, updated_state.playback_state);
                    } else {
                        println!("   ❌ No state found for speaker after event");
                    }
                })
                .start()
            {
                Ok(_stream) => {
                    println!("✅ Event streaming started successfully!");

                    // Monitor for 10 seconds with frequent state checks
                    for i in 1..=10 {
                        std::thread::sleep(Duration::from_secs(1));
                        
                        if let Some(current_state) = state_cache.get_speaker(speaker_id) {
                            println!("⏰ {}s - State: Volume={}, Muted={}, PlaybackState={:?}", 
                                i, current_state.volume, current_state.muted, current_state.playback_state);
                        } else {
                            println!("⏰ {}s - ❌ No state found for speaker", i);
                        }
                    }

                    println!("🏁 Test complete!");
                }
                Err(e) => {
                    println!("⚠️  Streaming failed: {:?}", e);
                }
            }
        }
        Err(e) => {
            println!("⚠️  Failed to create event stream: {:?}", e);
        }
    }

    Ok(())
}