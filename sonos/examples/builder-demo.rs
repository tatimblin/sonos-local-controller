use sonos::models::{Speaker, SpeakerId};
use sonos::streaming::{EventStreamBuilder, ServiceType};
use std::time::Duration;

fn create_test_speaker(id: &str, name: &str) -> Speaker {
    Speaker {
        id: SpeakerId::from_udn(id),
        udn: id.to_string(),
        name: name.to_string(),
        room_name: name.to_string(),
        ip_address: "192.168.1.100".to_string(),
        port: 1400,
        model_name: "Test Model".to_string(),
        satellites: vec![],
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎵 EventStreamBuilder Demo");

    // Create some test speakers
    let speakers = vec![
        create_test_speaker("uuid:RINCON_123456789::1", "Living Room"),
        create_test_speaker("uuid:RINCON_987654321::1", "Kitchen"),
    ];

    println!(
        "📋 Creating EventStreamBuilder with {} speakers",
        speakers.len()
    );

    // Demonstrate the builder pattern
    let builder = EventStreamBuilder::new(speakers)?
        .with_services(&[ServiceType::AVTransport, ServiceType::RenderingControl])
        .with_event_handler(|event| {
            println!("📢 Received event: {:?}", event);
        })
        .with_timeouts(Duration::from_secs(3600), Duration::from_secs(2))
        .with_callback_ports(9000, 9010);

    println!("✅ EventStreamBuilder configured successfully");
    println!("🚀 Attempting to start stream (may fail in test environment)...");

    // Try to start the stream (this will likely fail in a test environment)
    match builder.start() {
        Ok(stream) => {
            println!("✅ Stream started successfully!");
            println!("📊 Stream stats: {:?}", stream.stats());

            // Gracefully shutdown
            println!("🛑 Shutting down stream...");
            stream.shutdown()?;
            println!("✅ Stream shutdown complete");
        }
        Err(e) => {
            println!(
                "⚠️  Stream failed to start (expected in test environment): {:?}",
                e
            );
            println!("   This is normal when running without actual Sonos speakers");
        }
    }

    println!("🎯 Demo complete!");
    Ok(())
}
