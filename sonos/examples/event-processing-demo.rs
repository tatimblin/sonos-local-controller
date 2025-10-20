use sonos::streaming::{EventStreamBuilder, LifecycleHandlers, ServiceType};
use sonos::state::StateCache;
use sonos::models::{StateChange};
use sonos::transport::discovery;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎵 Event Processing System Demo with Real Speakers");
    println!("This demo shows the unified event processing system with:");
    println!("- Automatic StateCache updates");
    println!("- Multiple user event handlers");
    println!("- Lifecycle event callbacks");
    println!("- Error mapping and detection");
    println!("- ZoneGroupTopology network-wide service integration");
    
    // Discover real Sonos speakers
    println!("\n🔍 Discovering Sonos speakers on the network...");
    let speakers = discovery::discover_speakers_with_timeout(Duration::from_secs(1))?;
    
    if speakers.is_empty() {
        println!("❌ No Sonos speakers found on the network!");
        println!("   Make sure you have Sonos speakers connected to the same network.");
        return Ok(());
    }
    
    println!("✅ Found {} speaker(s):", speakers.len());
    for speaker in &speakers {
        println!("  - {} ({}) at {} in room '{}'", 
                 speaker.name, 
                 speaker.model_name, 
                 speaker.ip_address,
                 speaker.room_name);
    }
    
    // Create StateCache for automatic updates
    let state_cache = Arc::new(StateCache::new());
    state_cache.initialize(speakers.clone(), vec![]);
    
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
    let _stream = EventStreamBuilder::new(speakers)?
        .with_state_cache(state_cache.clone())
        .with_services(&[ServiceType::AVTransport, ServiceType::RenderingControl, ServiceType::ZoneGroupTopology])
        .with_event_handler(move |event| {
            let count = event_counter_clone.fetch_add(1, Ordering::SeqCst) + 1;
            println!("📨 Event #{}: {:?}", count, event);
            
            // Demonstrate StateCache integration
            match event {
                StateChange::PlaybackStateChanged { speaker_id, state: _ } => {
                    if let Some(speaker_state) = state_cache_clone.get_speaker(speaker_id) {
                        println!("   📊 StateCache updated - Speaker playback state: {:?}", speaker_state.playback_state);
                    }
                }
                StateChange::VolumeChanged { speaker_id, volume: _ } => {
                    if let Some(speaker_state) = state_cache_clone.get_speaker(speaker_id) {
                        println!("   📊 StateCache updated - Speaker volume: {}", speaker_state.volume);
                    }
                }
                StateChange::GroupTopologyChanged { groups, speakers_joined, speakers_left, coordinator_changes } => {
                    println!("   🏠 Zone topology changed:");
                    println!("      Groups: {}, Joined: {}, Left: {}, Coordinator changes: {}", 
                             groups.len(), speakers_joined.len(), speakers_left.len(), coordinator_changes.len());
                }
                StateChange::SpeakerJoinedGroup { speaker_id, group_id, coordinator_id } => {
                    println!("   ➕ Speaker {:?} joined group {:?} (coordinator: {:?})", speaker_id, group_id, coordinator_id);
                }
                StateChange::SpeakerLeftGroup { speaker_id, former_group_id } => {
                    println!("   ➖ Speaker {:?} left group {:?}", speaker_id, former_group_id);
                }
                StateChange::GroupFormed { group_id, coordinator_id, initial_members } => {
                    println!("   🆕 New group {:?} formed with coordinator {:?} and {} members", 
                             group_id, coordinator_id, initial_members.len());
                }
                StateChange::GroupDissolved { group_id, former_coordinator, former_members } => {
                    println!("   💥 Group {:?} dissolved (was coordinated by {:?}, had {} members)", 
                             group_id, former_coordinator, former_members.len());
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
                StateChange::SpeakerJoinedGroup { .. } => "Speaker Joined Group",
                StateChange::SpeakerLeftGroup { .. } => "Speaker Left Group",
                StateChange::CoordinatorChanged { .. } => "Coordinator Changed",
                StateChange::GroupFormed { .. } => "Group Formed",
                StateChange::GroupDissolved { .. } => "Group Dissolved",
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
            
            println!("\n⏳ Listening for events from real Sonos speakers...");
            println!("   Try playing/pausing music, changing volume, or switching tracks on your Sonos speakers");
            println!("   Press Ctrl+C to stop or wait 30 seconds for automatic shutdown");
            
            // Wait for events from real speakers
            std::thread::sleep(Duration::from_secs(30));
            
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
            println!("   This could happen if speakers become unavailable or network issues occur");
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