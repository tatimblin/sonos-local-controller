use sonos::ServiceType;
use sonos::{discover_speakers_with_timeout, streaming::EventStreamBuilder, SonosError};
use std::io::{self, Write};
use std::sync::atomic::{AtomicU32, Ordering};
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

    // Setup event streaming with new simplified interface
    println!("Using all {} speakers for monitoring", speakers.len());

    match EventStreamBuilder::new(speakers) {
        Ok(builder) => {
            let start_time = std::time::Instant::now();
            let event_count = Arc::new(AtomicU32::new(0));

            println!("🚀 Starting event stream builder...");
            match builder
                .with_services(&[ServiceType::ZoneGroupTopology])
                .with_event_handler({
                    let event_count = event_count.clone();
                    move |event| {
                        let count = event_count.fetch_add(1, Ordering::Relaxed) + 1;
                        let elapsed = start_time.elapsed();
                        println!(
                            "[{:>8.1}s] Event #{}: {:?}",
                            elapsed.as_secs_f64(),
                            count,
                            event
                        );
                        io::stdout().flush().unwrap();
                    }
                })
                .start()
            {
                Ok(_stream) => {
                    println!("✅ Event streaming active - monitoring topology changes\n");
                    println!("⏳ Waiting for topology changes...");
                    println!("   Try playing/pausing music or grouping speakers");
                    println!("   Press Ctrl+C to exit\n");

                    // Keep the program alive - events will be printed by the handler
                    loop {
                        std::thread::park();
                    }
                }
                Err(e) => {
                    println!("⚠️  Streaming failed: {:?}", e);
                    return Err(Box::new(e));
                }
            }
        }
        Err(e) => {
            println!("⚠️  Failed to create event stream: {:?}", e);
            println!("Displaying static topology...\n");
        }
    }

    Ok(())
}
