use sonos::streaming::{EventStreamBuilder, LifecycleHandlers, ServiceType};
use sonos::state::StateCache;
use sonos::models::{Speaker, SpeakerId, StateChange, PlaybackState};
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎵 Event Processing System Demo");
    println!("This demo shows the unified event processing system with:");
    println!("- Automatic StateCache updates");
    println!("- Multiple user event handlers");
    println!("- Lifecycle event callbacks");
    println!("- Error mapping and detection");
    
    // Create a test speaker
    let test_speaker = Speaker {
        id: SpeakerId::from_udn("uuid:RINCON_DEMO123456::1"),
        udn: "uuid:RINCON_DEMO123456::1".to_string(),
        name: "Demo Speaker".to_string(),
        room_name: "Demo Room".to_string(),
        ip_address: "192.168.1.100".to_string(),
        port: 1400,
        model_name: "Demo Model".to_string(),
        satellites: vec![],
    };
    
    // Create StateCache for automatic updates
    let state_cache = Arc::new(StateCache::new());
    state_cache.initialize(vec![test_speaker.clone()], vec![]);
    
    // Event counters for demonstration
    let event_counter = Arc::new(AtomicUsize::new(0));
    let handler1_counter = Arc::new(AtomicUsize::new(0));
    let handler2_counter = Arc::new(AtomicUsize::new(0));
    
    // Create lifecycle handlers
    let lifecycle_handlers = LifecycleHandlers::new()
        .with_stream_started(|| {
            println!("🚀 Stream started successfully!");
        })
        .with_stream_stopped(|| {
            println!("🛑 Stream stopped gracefully");
        })
        .with_speaker_connected(|speaker_id| {
            println!("🔗 Speaker {:?} connected", speaker_id);
        })
        .with_speaker_disconnected(|speaker_id| {
            println!("❌ Speaker {:?} disconnected", speaker_id);
        })
        .with_error(|error| {
            println!("⚠️  Stream error: {:?}", error);
        });
    
    // Clone counters for use in closures
    let event_counter_clone = event_counter.clone();
    let handler1_counter_clone = handler1_counter.clone();
    let handler2_counter_clone = handler2_counter.clone();
    let state_cache_clone = state_cache.clone();
    
    println!("\n📡 Creating EventStream with multiple handlers...");
    
    // Create EventStream with the new unified event processing system
    let _stream = EventStreamBuilder::new(vec![test_speaker])?
        .with_state_cache(state_cache.clone())
        .with_services(&[ServiceType::AVTransport, ServiceType::RenderingControl])
        .with_event_handler(move |event| {
            let count = event_counter_clone.fetch_add(1, Ordering::SeqCst) + 1;
            println!("📨 Event #{}: {:?}", count, event);
            
            // Demonstrate StateCache integration
            match event {
                StateChange::PlaybackStateChanged { speaker_id, state } => {
                    if let Some(speaker_state) = state_cache_clone.get_speaker(speaker_id) {
                        println!("   📊 StateCache updated - Speaker playback state: {:?}", speaker_state.playback_state);
                    }
                }
                StateChange::VolumeChanged { speaker_id, volume } => {
                    if let Some(speaker_state) = state_cache_clone.get_speaker(speaker_id) {
                        println!("   📊 StateCache updated - Speaker volume: {}", speaker_state.volume);
                    }
                }
                _ => {}
            }
        })
        .with_event_handler(move |event| {
            let count = handler1_counter_clone.fetch_add(1, Ordering::SeqCst) + 1;
            println!("   🎯 Handler 1 processed event #{}: {}", count, match event {
                StateChange::PlaybackStateChanged { .. } => "Playback State Change",
                StateChange::VolumeChanged { .. } => "Volume Change",
                StateChange::MuteChanged { .. } => "Mute Change",
                StateChange::PositionChanged { .. } => "Position Change",
                StateChange::TrackChanged { .. } => "Track Change",
                StateChange::TransportInfoChanged { .. } => "Transport Info Change",
                StateChange::GroupTopologyChanged { .. } => "Group Topology Change",
                StateChange::SubscriptionError { .. } => "Subscription Error",
            });
        })
        .with_event_handler(move |_event| {
            let count = handler2_counter_clone.fetch_add(1, Ordering::SeqCst) + 1;
            println!("   🎯 Handler 2 processed event #{}", count);
        })
        .with_lifecycle_handlers(lifecycle_handlers)
        .start();
    
    match _stream {
        Ok(stream) => {
            println!("✅ EventStream created successfully!");
            println!("📊 Stream stats: {:?}", stream.stats());
            
            println!("\n⏳ Waiting for events (this demo will timeout since no real speakers are available)...");
            println!("   In a real scenario, events would be received from Sonos speakers");
            
            // Wait a bit to see if any events come through
            std::thread::sleep(Duration::from_secs(2));
            
            println!("\n📈 Final Statistics:");
            println!("   Total events processed: {}", event_counter.load(Ordering::SeqCst));
            println!("   Handler 1 calls: {}", handler1_counter.load(Ordering::SeqCst));
            println!("   Handler 2 calls: {}", handler2_counter.load(Ordering::SeqCst));
            
            println!("\n🔧 Demonstrating graceful shutdown...");
            stream.shutdown()?;
            println!("✅ Stream shutdown completed");
        }
        Err(e) => {
            println!("❌ Failed to create EventStream: {:?}", e);
            println!("   This is expected in demo mode without real Sonos speakers");
            println!("   The error demonstrates the error mapping system:");
            
            // Show how errors are mapped to user-friendly messages
            match e {
                sonos::streaming::StreamError::InitializationFailed(msg) => {
                    println!("   - Initialization error with actionable message: {}", msg);
                }
                sonos::streaming::StreamError::NetworkError(msg) => {
                    println!("   - Network error with troubleshooting info: {}", msg);
                }
                sonos::streaming::StreamError::ConfigurationError(msg) => {
                    println!("   - Configuration error with fix suggestions: {}", msg);
                }
                _ => {
                    println!("   - Other error type: {:?}", e);
                }
            }
        }
    }
    
    println!("\n🎯 Demo completed!");
    println!("The unified event processing system provides:");
    println!("✅ Background thread for event processing");
    println!("✅ Automatic StateCache updates using existing logic");
    println!("✅ Multiple event handlers called in registration order");
    println!("✅ Lifecycle callbacks for connection events");
    println!("✅ Error mapping to user-friendly messages");
    println!("✅ Graceful shutdown handling");
    
    Ok(())
}