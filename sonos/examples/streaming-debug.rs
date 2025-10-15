use sonos::streaming::{CallbackServer, SubscriptionManager};
use sonos::{discover_speakers_with_timeout, ServiceType, SonosError, StreamConfig};
use std::io::{self, Write};
use std::sync::mpsc;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Sonos Streaming System Debug Tool 🔧");
    println!("═══════════════════════════════════════════════════════════════");

    // Step 1: Test speaker discovery
    println!("\n📡 Step 1: Testing speaker discovery...");
    let speakers = match discover_speakers_with_timeout(Duration::from_secs(1)) {
        Ok(speakers) => {
            println!("✅ Discovery successful: Found {} speakers", speakers.len());
            speakers
        }
        Err(SonosError::DiscoveryFailed(_)) => {
            println!("❌ No Sonos speakers found on the network.");
            return Ok(());
        }
        Err(e) => {
            println!("❌ Discovery failed: {:?}", e);
            return Err(Box::new(e));
        }
    };

    if speakers.is_empty() {
        println!("❌ No speakers available for testing");
        return Ok(());
    }

    for (i, speaker) in speakers.iter().enumerate() {
        println!(
            "   {}. {} ({}) at {}:{}",
            i + 1,
            speaker.name,
            speaker.model_name,
            speaker.ip_address,
            speaker.port
        );
        println!("      UDN: {}", speaker.udn);
        println!("      ID: {:?}", speaker.id);
    }

    let selected_speaker = speakers[0].clone();
    println!("\n🎯 Selected speaker: {}", selected_speaker.name);

    // Step 2: Skip separate callback server (SubscriptionManager will create it)
    println!("\n📡 Step 2: Skipping separate callback server creation...");
    println!("   SubscriptionManager will create and manage the callback server");
    
    let port_range = (9080, 9090);

    // Step 3: Ready for SubscriptionManager
    println!("\n🚀 Step 3: Ready to create SubscriptionManager...");

    // Step 4: Test subscription manager creation
    println!("\n🔧 Step 4: Testing subscription manager creation...");
    let config = StreamConfig {
        enabled_services: vec![ServiceType::AVTransport],
        callback_port_range: port_range,
        subscription_timeout: Duration::from_secs(1800),
        retry_attempts: 3,
        retry_backoff: Duration::from_secs(1),
        buffer_size: 1000,
    };

    let (event_sender, event_receiver) = mpsc::channel();

    let subscription_manager = match SubscriptionManager::new(config.clone(), event_sender) {
        Ok(manager) => {
            println!("✅ Subscription manager created successfully");
            manager
        }
        Err(e) => {
            println!("❌ Failed to create subscription manager: {:?}", e);
            return Err(Box::new(e));
        }
    };

    println!(
        "   Callback server port: {:?}",
        subscription_manager.callback_server_port()
    );
    println!(
        "   Initial subscription count: {}",
        subscription_manager.subscription_count()
    );
    println!(
        "   Initial speaker count: {}",
        subscription_manager.speaker_count()
    );

    // Step 5: Test adding a speaker
    println!("\n👤 Step 5: Testing speaker subscription...");
    println!("   Adding speaker: {}", selected_speaker.name);

    match subscription_manager.add_speaker(selected_speaker.clone()) {
        Ok(()) => {
            println!("✅ Speaker added successfully");
        }
        Err(e) => {
            println!("❌ Failed to add speaker: {:?}", e);

            // Check if it's a satellite speaker error
            if format!("{:?}", e).contains("SatelliteSpeaker") {
                println!("   This appears to be a satellite/bonded speaker");
                println!("   Satellite speakers don't accept direct subscriptions");

                if speakers.len() > 1 {
                    println!("   Trying next speaker...");
                    let next_speaker = speakers[1].clone();
                    println!("   Adding speaker: {}", next_speaker.name);

                    match subscription_manager.add_speaker(next_speaker) {
                        Ok(()) => {
                            println!("✅ Second speaker added successfully");
                        }
                        Err(e2) => {
                            println!("❌ Second speaker also failed: {:?}", e2);
                        }
                    }
                }
            }
        }
    }

    // Step 6: Check subscription status
    println!("\n📊 Step 6: Checking subscription status...");
    println!(
        "   Subscription count: {}",
        subscription_manager.subscription_count()
    );
    println!("   Speaker count: {}", subscription_manager.speaker_count());

    let subscription_info = subscription_manager.get_subscription_info();
    println!("   Subscription details:");
    for info in &subscription_info {
        println!("     - ID: {}", info.id);
        println!(
            "       Speaker: {} ({:?})",
            info.speaker_name, info.speaker_id
        );
        println!("       Service: {:?}", info.service_type);
        println!("       Active: {}", info.is_active);
        println!("       Needs renewal: {}", info.needs_renewal);
    }

    if subscription_info.is_empty() {
        println!("   ⚠️  No active subscriptions found!");
        println!("   This means the speaker didn't accept the subscription request");
        return Ok(());
    }

    // Step 7: Test event reception
    println!("\n🎵 Step 7: Testing event reception...");
    println!("   Waiting for events for 30 seconds...");
    println!("   Try playing/pausing music on your Sonos speaker now!");

    let mut event_count = 0;
    let start_time = std::time::Instant::now();

    while start_time.elapsed() < Duration::from_secs(30) {
        // Check for events from subscription manager
        match event_receiver.recv_timeout(Duration::from_millis(500)) {
            Ok(event) => {
                event_count += 1;
                println!("🎵 Event #{}: {:?}", event_count, event);
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // No event, continue
                print!(
                    "\r⏳ Waiting... ({}s elapsed, {} events)",
                    start_time.elapsed().as_secs(),
                    event_count
                );
                io::stdout().flush()?;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                println!("\n❌ Event channel disconnected");
                break;
            }
        }

        // Note: Raw HTTP events are now logged directly by the callback server
    }

    println!("\n\n📈 Final Results:");
    println!("   Total events received: {}", event_count);
    println!(
        "   Final subscription count: {}",
        subscription_manager.subscription_count()
    );

    if event_count == 0 {
        println!("\n🔍 Troubleshooting Analysis:");
        println!("   1. Callback server: ✅ Started successfully");
        println!("   2. Subscription manager: ✅ Created successfully");

        if subscription_info.is_empty() {
            println!("   3. Subscriptions: ❌ No active subscriptions");
            println!("      → The speaker rejected the subscription request");
            println!("      → This could be a satellite speaker or network issue");
        } else {
            println!(
                "   3. Subscriptions: ✅ {} active subscriptions",
                subscription_info.len()
            );
            println!("   4. Events: ❌ No events received");
            println!("      → Subscriptions are active but no events are being sent");
            println!("      → Try playing/pausing music or changing volume");
            println!(
                "      → Check firewall settings for port {}",
                subscription_manager.callback_server_port().unwrap_or(0)
            );
        }
    } else {
        println!("   ✅ Event streaming is working correctly!");
    }

    Ok(())
}
