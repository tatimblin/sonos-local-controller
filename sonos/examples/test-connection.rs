use sonos::{discover_speakers_with_timeout, SonosError};
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing connection to Sonos devices...");

    // Discover speakers with timeout
    let speakers = match discover_speakers_with_timeout(Duration::from_secs(5)) {
        Ok(speakers) => speakers,
        Err(SonosError::DiscoveryFailed(_)) => {
            println!("No Sonos speakers found on the network.");
            return Ok(());
        }
        Err(e) => return Err(Box::new(e)),
    };

    if speakers.is_empty() {
        println!("No Sonos speakers found on the network.");
        return Ok(());
    }

    println!("Found {} speakers:", speakers.len());
    for speaker in &speakers {
        println!(
            "  - {} ({}) at {}:{}",
            speaker.name, speaker.model_name, speaker.ip_address, speaker.port
        );
    }

    // Test basic HTTP connectivity to each speaker
    for speaker in &speakers {
        println!("\n🔍 Testing connectivity to {}...", speaker.name);
        
        let device_url = format!("http://{}:{}", speaker.ip_address, speaker.port);
        let event_sub_url = "/MediaRenderer/AVTransport/Event";
        let full_url = format!("{}{}", device_url, event_sub_url);
        
        println!("   Target URL: {}", full_url);
        
        // Create HTTP client with short timeout
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;
        
        // Try a simple GET request first to see if the device responds
        println!("   Trying GET request...");
        match client.get(&device_url).send() {
            Ok(response) => {
                println!("   ✅ GET response: {} {}", response.status(), 
                    response.status().canonical_reason().unwrap_or(""));
            }
            Err(e) => {
                println!("   ❌ GET request failed: {}", e);
                continue;
            }
        }
        
        // Now try the SUBSCRIBE request (this is what's actually failing)
        println!("   Trying SUBSCRIBE request...");
        match client
            .request(
                reqwest::Method::from_bytes(b"SUBSCRIBE").unwrap(),
                &full_url,
            )
            .header("HOST", format!("{}:{}", speaker.ip_address, speaker.port))
            .header("CALLBACK", "<http://192.168.1.100:8080/test>")
            .header("NT", "upnp:event")
            .header("TIMEOUT", "Second-1800")
            .send()
        {
            Ok(response) => {
                println!("   ✅ SUBSCRIBE response: {} {}", response.status(),
                    response.status().canonical_reason().unwrap_or(""));
                
                // Print response headers for debugging
                println!("   Response headers:");
                for (name, value) in response.headers() {
                    println!("     {}: {:?}", name, value);
                }
            }
            Err(e) => {
                println!("   ❌ SUBSCRIBE request failed: {}", e);
            }
        }
    }

    Ok(())
}