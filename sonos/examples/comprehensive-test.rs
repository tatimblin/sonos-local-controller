use sonos::{
    discover_speakers_with_timeout, ServiceType, SonosError, StreamConfig,
};
use sonos::streaming::{SubscriptionManager, EventStream};
use std::sync::mpsc;
use std::time::Duration;
use std::io::{self, Write};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Comprehensive Sonos Event Test 🔧");
    println!("═══════════════════════════════════════════════════════════════");

    // Discover speakers
    println!("📡 Discovering speakers...");
    let speakers = match discover_speakers_with_timeout(Duration::from_secs(5)) {
        Ok(speakers) => speakers,
        Err(SonosError::DiscoveryFailed(_)) => {
            println!("❌ No Sonos speakers found");
            return Ok(());
        }
        Err(e) => return Err(Box::new(e)),
    };

    println!("✅ Found {} speakers", speakers.len());
    for (i, speaker) in speakers.iter().enumerate() {
        println!("   {}. {} at {}", i + 1, speaker.name, speaker.ip_address);
    }

    // Try each speaker until we find one that works
    for (i, speaker) in speakers.iter().enumerate() {
        println!("\n🎯 Testing speaker {}: {}", i + 1, speaker.name);
        
        match test_speaker_events(speaker.clone()) {
            Ok(event_count) => {
                if event_count > 0 {
                    println!("🎉 SUCCESS! Received {} events from {}", event_count, speaker.name);
                    println!("✅ Event streaming is working!");
                    return Ok(());
                } else {
                    println!("⚠️  No events received from {} (but subscription worked)", speaker.name);
                }
            }
            Err(e) => {
                println!("❌ Failed with {}: {:?}", speaker.name, e);
            }
        }
    }

    println!("\n📈 Summary: Tested all {} speakers, no events received", speakers.len());
    println!("💡 This might be normal if no music is playing or speakers are idle");
    
    Ok(())
}

fn test_speaker_events(speaker: sonos::Speaker) -> Result<usize, Box<dyn std::error::Error>> {
    println!("   📡 Creating event stream for {}...", speaker.name);
    
    let config = StreamConfig {
        enabled_services: vec![ServiceType::AVTransport],
        callback_port_range: (8080, 8090),
        subscription_timeout: Duration::from_secs(1800),
        retry_attempts: 3,
        retry_backoff: Duration::from_secs(1),
        buffer_size: 1000,
    };

    let event_stream = EventStream::new(vec![speaker.clone()], config)?;
    
    println!("   ✅ Event stream created, listening for 10 seconds...");
    println!("   💡 Try playing/pausing music on {} now!", speaker.name);
    
    let mut event_count = 0;
    let start_time = std::time::Instant::now();
    
    while start_time.elapsed() < Duration::from_secs(10) {
        if let Some(event) = event_stream.recv_timeout(Duration::from_millis(100)) {
            event_count += 1;
            println!("   🎵 Event #{}: {:?}", event_count, event);
        }
        
        // Show progress
        if start_time.elapsed().as_secs() % 2 == 0 && start_time.elapsed().as_millis() % 2000 < 100 {
            print!("\r   ⏳ {}s elapsed, {} events...", start_time.elapsed().as_secs(), event_count);
            io::stdout().flush()?;
        }
    }
    
    println!("\r   ⏱️  Test completed: {} events in 10 seconds", event_count);
    Ok(event_count)
}