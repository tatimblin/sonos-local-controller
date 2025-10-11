use sonos::transport::discovery;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Discovering Sonos speakers...");
    
    // Simple discovery with default timeout
    let speakers = discovery::discover_speakers()?;
    
    println!("Found {} speakers:", speakers.len());
    for speaker in &speakers {
        println!("  - {} ({}) at {}", 
                 speaker.name, 
                 speaker.model_name, 
                 speaker.ip_address);
    }
    
    // Discovery with custom timeout
    println!("\nDiscovering with 5 second timeout...");
    let speakers_custom = discovery::discover_speakers_with_timeout(Duration::from_secs(5))?;
    
    println!("Found {} speakers with custom timeout:", speakers_custom.len());
    for speaker in &speakers_custom {
        println!("  - Room: {}, UDN: {}", 
                 speaker.room_name, 
                 speaker.udn);
    }
    
    Ok(())
}