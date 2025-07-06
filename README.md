# Sonos Local Controller

A Rust library and CLI application for discovering and controlling Sonos speakers on your local network.

## Features

- **Speaker Discovery**: Automatic discovery of Sonos speakers using SSDP
- **Topology Integration**: Retrieve and manage speaker group hierarchies
- **Dual Access Patterns**: Support for both nested group displays and flat speaker operations
- **Event-Driven Architecture**: Real-time updates during discovery and system changes
- **CLI Interface**: Ready-to-use command-line interface for speaker control

## Quick Start

### Library Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
sonos = { path = "./sonos" }
```

### Basic Discovery

```rust
use sonos::{System, SystemEvent};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut system = System::new()?;
    
    // Discover speakers and collect events
    let events: Vec<_> = system.discover().collect();
    
    // Process discovery results
    for event in events {
        match event {
            SystemEvent::SpeakerFound(speaker) => {
                println!("Found: {} at {}", speaker.name(), speaker.ip());
            },
            SystemEvent::TopologyReady(topology) => {
                println!("Topology: {} groups", topology.zone_group_count());
            },
            SystemEvent::DiscoveryComplete => {
                println!("Discovery finished!");
            },
            SystemEvent::Error(msg) => {
                println!("Error: {}", msg);
            },
            _ => {}
        }
    }
    
    // System remains available after discovery
    println!("Total speakers found: {}", system.speaker_count());
    
    Ok(())
}
```

### Nested Group Display

```rust
use sonos::System;

fn display_speaker_groups(system: &System) -> Result<(), String> {
    if !system.has_topology() {
        return Err("No topology available".to_string());
    }
    
    let topology = system.topology().unwrap();
    
    for zone_group in &topology.zone_groups {
        println!("Group: {}", zone_group.id);
        
        // Display coordinator
        if let Some(coordinator) = system.get_speaker_by_uuid(&zone_group.coordinator) {
            println!("  👑 {} (Coordinator)", coordinator.name());
        }
        
        // Display members
        for member in &zone_group.members {
            if member.uuid != zone_group.coordinator {
                if let Some(speaker) = system.get_speaker_by_uuid(&member.uuid) {
                    println!("  🔊 {} (Member)", speaker.name());
                }
            }
        }
    }
    
    Ok(())
}
```

### Flat Speaker Operations

```rust
use sonos::System;

fn control_speakers(system: &System) {
    // List all speakers
    for (uuid, speaker) in system.speakers() {
        println!("Speaker: {} ({}) - {}", speaker.name(), speaker.ip(), uuid);
    }
    
    // Find specific speaker
    if let Some(speaker) = system.get_speaker_by_uuid("RINCON_123456") {
        println!("Found speaker: {}", speaker.name());
        // Perform operations: speaker.play(), speaker.pause(), etc.
    }
    
    // Find speaker by name
    let living_room = system.speakers()
        .values()
        .find(|s| s.name().to_lowercase().contains("living room"));
    
    if let Some(speaker) = living_room {
        println!("Living room speaker: {} at {}", speaker.name(), speaker.ip());
    }
}
```

## API Reference

### System Struct

The main entry point for speaker discovery and management.

```rust
impl System {
    pub fn new() -> Result<Self, std::io::Error>
    pub fn discover(&mut self) -> impl Iterator<Item = SystemEvent> + '_
    pub fn speakers(&self) -> &HashMap<String, Box<dyn SpeakerTrait>>
    pub fn topology(&self) -> Option<&Topology>
    pub fn has_topology(&self) -> bool
    pub fn speaker_count(&self) -> usize
    pub fn get_speaker_by_uuid(&self, uuid: &str) -> Option<&Box<dyn SpeakerTrait>>
}
```

### System Events

Events emitted during discovery and system changes:

```rust
#[derive(Debug)]
pub enum SystemEvent {
    SpeakerFound(Speaker),           // Individual speaker discovered
    TopologyReady(Topology),         // Complete topology available
    Error(String),                   // General errors
    DiscoveryComplete,               // Discovery process finished
}
```

## Examples

See the `examples/` directory for complete working examples:

- `sonos-rs-demo.rs` - Basic discovery and control example

## Development

### Building

```bash
cargo build
```

### Testing

```bash
cargo test
```

### Running Examples

```bash
cargo run --example sonos-rs-demo
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Built on top of the Sonos UPnP API
- Uses SSDP for speaker discovery