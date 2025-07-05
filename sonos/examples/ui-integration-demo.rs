use sonos::{System, SystemEvent};
use sonos::speaker::SpeakerTrait;

/// Comprehensive example demonstrating UI integration patterns
/// 
/// This example shows:
/// 1. Event-driven discovery with real-time updates
/// 2. Nested group display using topology
/// 3. Flat speaker operations for control
/// 4. Error handling and state management
/// 5. Migration from old API patterns

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎵 Sonos UI Integration Demo");
    println!("═══════════════════════════════");
    
    // Example 1: Basic discovery with new API
    basic_discovery_example()?;
    
    // Example 2: Event-driven discovery
    event_driven_discovery_example()?;
    
    // Example 3: Nested group display
    nested_group_display_example()?;
    
    // Example 4: Flat speaker operations
    flat_speaker_operations_example()?;
    
    // Example 5: Combined patterns
    combined_patterns_example()?;
    
    Ok(())
}

/// Example 1: Basic discovery showing API migration
fn basic_discovery_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📍 Example 1: Basic Discovery");
    println!("─────────────────────────────");
    
    // NEW API: System uses &mut self instead of consuming self
    let mut system = System::new()?;
    
    println!("Starting discovery...");
    let events: Vec<_> = system.discover().collect();
    
    // System is still available after discovery (this is the key improvement)
    println!("Discovery completed. Found {} speakers", system.speaker_count());
    
    // Process events
    let speaker_count = events.iter()
        .filter(|e| matches!(e, SystemEvent::SpeakerFound(_)))
        .count();
    let has_topology = events.iter()
        .any(|e| matches!(e, SystemEvent::TopologyReady(_)));
    let error_count = events.iter()
        .filter(|e| matches!(e, SystemEvent::Error(_)))
        .count();
    
    println!("Events summary:");
    println!("  - Speakers found: {}", speaker_count);
    println!("  - Topology available: {}", has_topology);
    println!("  - Errors encountered: {}", error_count);
    
    Ok(())
}

/// Example 2: Event-driven discovery with real-time UI updates
fn event_driven_discovery_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📍 Example 2: Event-Driven Discovery");
    println!("───────────────────────────────────");
    
    let mut system = System::new()?;
    let mut ui_state = UIState::new();
    
    println!("Processing discovery events in real-time...");
    
    for event in system.discover() {
        // Update UI state based on event
        ui_state.handle_event(&event);
        
        // Simulate UI update
        ui_state.render();
        
        // Small delay to show progression (remove in real UI)
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    
    // Final state
    ui_state.finalize(&system);
    
    Ok(())
}

/// Example 3: Nested group display using topology
fn nested_group_display_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📍 Example 3: Nested Group Display");
    println!("──────────────────────────────────");
    
    let mut system = System::new()?;
    let _events: Vec<_> = system.discover().collect();
    
    if system.has_topology() {
        display_nested_groups(&system)?;
    } else {
        println!("⚠️  No topology available - speakers may be offline or network issues");
        println!("Falling back to flat display...");
        display_flat_speakers(&system);
    }
    
    Ok(())
}

/// Example 4: Flat speaker operations for direct control
fn flat_speaker_operations_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📍 Example 4: Flat Speaker Operations");
    println!("────────────────────────────────────");
    
    let mut system = System::new()?;
    let _events: Vec<_> = system.discover().collect();
    
    if system.speaker_count() == 0 {
        println!("No speakers found for operations demo");
        return Ok(());
    }
    
    // Direct speaker access
    println!("🔊 All Speakers ({} total):", system.speaker_count());
    for (uuid, speaker) in system.speakers() {
        println!("  • {} ({}) - {}", speaker.name(), speaker.ip(), uuid);
    }
    
    // Find speakers by criteria
    println!("\n🔍 Speaker Search Examples:");
    
    // Find by name (case-insensitive)
    let living_room_speakers: Vec<_> = system.speakers()
        .values()
        .filter(|s| s.name().to_lowercase().contains("living"))
        .collect();
    
    if !living_room_speakers.is_empty() {
        println!("  Living room speakers:");
        for speaker in living_room_speakers {
            println!("    - {} ({})", speaker.name(), speaker.ip());
        }
    }
    
    // Find by IP range
    let local_speakers: Vec<_> = system.speakers()
        .values()
        .filter(|s| s.ip().starts_with("192.168.1."))
        .collect();
    
    println!("  Speakers in 192.168.1.x range: {}", local_speakers.len());
    
    // Direct UUID lookup
    if let Some((first_uuid, _)) = system.speakers().iter().next() {
        if let Some(speaker) = system.get_speaker_by_uuid(first_uuid) {
            println!("  Direct UUID lookup example: {} found", speaker.name());
        }
    }
    
    // Bulk operations example
    println!("\n⚡ Bulk Operations:");
    perform_bulk_operation(&system, |speaker| {
        println!("    Processing: {} at {}", speaker.name(), speaker.ip());
        // In real implementation: speaker.get_volume(), speaker.get_state(), etc.
        Ok(())
    })?;
    
    Ok(())
}

/// Example 5: Combined patterns - topology for display, flat for operations
fn combined_patterns_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📍 Example 5: Combined Patterns");
    println!("──────────────────────────────");
    
    let mut system = System::new()?;
    let _events: Vec<_> = system.discover().collect();
    
    if !system.has_topology() || system.speaker_count() == 0 {
        println!("Insufficient data for combined patterns demo");
        return Ok(());
    }
    
    let topology = system.topology().unwrap();
    
    println!("🏠 Group-based Operations:");
    
    for zone_group in &topology.zone_groups {
        println!("\n  Group: {}", zone_group.id);
        
        // Use topology to identify group structure
        let coordinator_uuid = &zone_group.coordinator;
        
        // Use flat access to get speaker details and perform operations
        if let Some(coordinator) = system.get_speaker_by_uuid(coordinator_uuid) {
            println!("    👑 Coordinator: {} ({})", coordinator.name(), coordinator.ip());
            
            // Example operation on coordinator
            println!("      → Checking coordinator status...");
            // coordinator.get_transport_info(), etc.
        }
        
        // Process group members
        let member_count = zone_group.members.len();
        println!("    📊 Group has {} members", member_count);
        
        for member in &zone_group.members {
            if member.uuid != *coordinator_uuid {
                if let Some(speaker) = system.get_speaker_by_uuid(&member.uuid) {
                    println!("    🔊 Member: {} ({})", speaker.name(), speaker.ip());
                    
                    // Example operation on member
                    println!("      → Syncing with coordinator...");
                    // speaker.join_group(), etc.
                }
            }
        }
    }
    
    Ok(())
}

// Helper functions and types

struct UIState {
    speakers_found: usize,
    topology_ready: bool,
    errors: Vec<String>,
    discovery_complete: bool,
}

impl UIState {
    fn new() -> Self {
        Self {
            speakers_found: 0,
            topology_ready: false,
            errors: Vec::new(),
            discovery_complete: false,
        }
    }
    
    fn handle_event(&mut self, event: &SystemEvent) {
        match event {
            SystemEvent::SpeakerFound(_) => {
                self.speakers_found += 1;
            },
            SystemEvent::TopologyReady(_) => {
                self.topology_ready = true;
            },
            SystemEvent::Error(msg) => {
                self.errors.push(msg.clone());
            },
            SystemEvent::DiscoveryComplete => {
                self.discovery_complete = true;
            },
            SystemEvent::GroupUpdate(_, _) => {
                // Handle group updates
            },
        }
    }
    
    fn render(&self) {
        print!("  Status: ");
        if self.discovery_complete {
            print!("✅ Complete");
        } else {
            print!("🔄 Discovering");
        }
        
        print!(" | Speakers: {} | Topology: {}", 
               self.speakers_found,
               if self.topology_ready { "✅" } else { "⏳" });
        
        if !self.errors.is_empty() {
            print!(" | Errors: {}", self.errors.len());
        }
        
        println!();
    }
    
    fn finalize(&self, system: &System) {
        println!("\n🏁 Final Results:");
        println!("  Speakers discovered: {}", self.speakers_found);
        println!("  Topology available: {}", self.topology_ready);
        println!("  System speaker count: {}", system.speaker_count());
        println!("  System has topology: {}", system.has_topology());
        
        if !self.errors.is_empty() {
            println!("  Errors encountered:");
            for error in &self.errors {
                println!("    - {}", error);
            }
        }
    }
}

fn display_nested_groups(system: &System) -> Result<(), String> {
    let topology = system.topology().ok_or("No topology available")?;
    
    println!("🏠 Speaker Groups ({} groups):", topology.zone_group_count());
    
    for (index, zone_group) in topology.zone_groups.iter().enumerate() {
        println!("\n  📁 Group {} - {}", index + 1, zone_group.id);
        
        // Display coordinator
        if let Some(coordinator) = system.get_speaker_by_uuid(&zone_group.coordinator) {
            println!("    👑 {} (Coordinator) - {}", coordinator.name(), coordinator.ip());
        }
        
        // Display other members
        let other_members: Vec<_> = zone_group.members.iter()
            .filter(|m| m.uuid != zone_group.coordinator)
            .collect();
        
        for member in other_members {
            if let Some(speaker) = system.get_speaker_by_uuid(&member.uuid) {
                println!("    🔊 {} (Member) - {}", speaker.name(), speaker.ip());
                
                // Display satellites if any
                for satellite in &member.satellites {
                    if let Some(satellite_speaker) = system.get_speaker_by_uuid(&satellite.uuid) {
                        println!("      📡 {} (Satellite) - {}", satellite_speaker.name(), satellite_speaker.ip());
                    }
                }
            }
        }
    }
    
    // Display vanished devices
    if let Some(vanished) = &topology.vanished_devices {
        if !vanished.devices.is_empty() {
            println!("\n  👻 Vanished Devices:");
            for device in &vanished.devices {
                println!("    - {} ({}): {}", device.zone_name, device.uuid, device.reason);
            }
        }
    }
    
    Ok(())
}

fn display_flat_speakers(system: &System) {
    println!("🔊 All Speakers ({} total):", system.speaker_count());
    
    if system.speaker_count() == 0 {
        println!("  No speakers found");
        return;
    }
    
    for (uuid, speaker) in system.speakers() {
        println!("  • {} ({}) - {}", speaker.name(), speaker.ip(), uuid);
    }
}

fn perform_bulk_operation<F>(system: &System, operation: F) -> Result<(), String>
where
    F: Fn(&Box<dyn sonos::speaker::SpeakerTrait>) -> Result<(), String>,
{
    let mut errors = Vec::new();
    
    for (uuid, speaker) in system.speakers() {
        if let Err(e) = operation(speaker) {
            errors.push(format!("  ❌ {} ({}): {}", speaker.name(), uuid, e));
        }
    }
    
    if errors.is_empty() {
        println!("  ✅ All operations completed successfully");
        Ok(())
    } else {
        println!("  ⚠️  Some operations failed:");
        for error in &errors {
            println!("{}", error);
        }
        Err(format!("{} operations failed", errors.len()))
    }
}

/// Migration example showing old vs new patterns
#[allow(dead_code)]
fn migration_example() {
    println!("\n📍 Migration Example");
    println!("───────────────────");
    
    // OLD WAY (would not compile with new API):
    // let system = System::new().unwrap();
    // let events: Vec<_> = system.discover().collect();
    // // system is consumed here - cannot use it anymore
    
    // NEW WAY:
    let mut system = System::new().unwrap();
    let events: Vec<_> = system.discover().collect();
    // system is still available for use
    let _speaker_count = system.speaker_count();
    
    // Event handling updates:
    for event in events {
        match event {
            // OLD: SystemEvent::Found(speaker) => { ... }
            SystemEvent::SpeakerFound(speaker) => {
                println!("Found: {}", speaker.name());
            },
            
            // NEW events to handle:
            SystemEvent::TopologyReady(topology) => {
                println!("Topology ready: {} groups", topology.zone_group_count());
            },
            
            SystemEvent::DiscoveryComplete => {
                println!("Discovery finished");
            },
            
            SystemEvent::Error(msg) => {
                println!("Error: {}", msg);
            },
            
            SystemEvent::GroupUpdate(group_id, members) => {
                println!("Group {} updated: {} members", group_id, members.len());
            },
        }
    }
}