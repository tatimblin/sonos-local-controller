# UI Integration Guide

This guide demonstrates how to integrate the Sonos System API with user interfaces, covering both nested group displays and flat speaker operations.

## Overview

The System API provides two complementary data access patterns:

1. **Nested Group Display**: Use topology information to show speakers organized by groups and hierarchies
2. **Flat Speaker Operations**: Direct access to individual speakers for control operations

## API Changes Summary

### Breaking Changes

- `discover()` method now takes `&mut self` instead of consuming `self`
- `SystemEvent::Found` renamed to `SystemEvent::SpeakerFound`
- New events: `TopologyReady` and `DiscoveryComplete`

### New Methods

```rust
impl System {
    pub fn speakers(&self) -> &HashMap<String, Box<dyn SpeakerTrait>>
    pub fn topology(&self) -> Option<&Topology>
    pub fn has_topology(&self) -> bool
    pub fn speaker_count(&self) -> usize
    pub fn get_speaker_by_uuid(&self, uuid: &str) -> Option<&Box<dyn SpeakerTrait>>
}
```

### New Events

```rust
#[derive(Debug)]
pub enum SystemEvent {
    SpeakerFound(Speaker),           // Individual speaker discovered
    TopologyReady(Topology),         // Complete topology available
    Error(String),                   // General errors (including topology failures)
    DiscoveryComplete,               // Discovery process finished
    GroupUpdate(String, Vec<String>), // Group membership changes
}
```

## Pattern 1: Nested Group Display

Use this pattern when you want to display speakers organized by their group relationships, showing coordinators, members, and satellites in a hierarchical structure.

### Basic Implementation

```rust
use sonos::{System, SystemEvent};
use std::collections::HashMap;

fn display_nested_groups(system: &System) -> Result<(), String> {
    // Check if topology is available
    if !system.has_topology() {
        return Err("Topology not available. Run discovery first.".to_string());
    }
    
    let topology = system.topology().unwrap();
    let speakers = system.speakers();
    
    println!("Sonos System - {} Groups, {} Total Speakers", 
             topology.zone_group_count(), 
             system.speaker_count());
    
    // Iterate through zone groups
    for (group_index, zone_group) in topology.zone_groups.iter().enumerate() {
        println!("\n📁 Group {} (ID: {})", group_index + 1, zone_group.id);
        
        // Display coordinator
        if let Some(coordinator) = system.get_speaker_by_uuid(&zone_group.coordinator) {
            println!("  👑 Coordinator: {} ({})", 
                     coordinator.name(), 
                     coordinator.ip());
        }
        
        // Display all members
        for member in &zone_group.members {
            if let Some(speaker) = system.get_speaker_by_uuid(&member.uuid) {
                let role = if member.uuid == zone_group.coordinator {
                    "Coordinator"
                } else {
                    "Member"
                };
                
                println!("  🔊 {}: {} ({})", 
                         role,
                         speaker.name(), 
                         speaker.ip());
                
                // Display satellites if any
                for satellite_uuid in &member.satellites {
                    if let Some(satellite) = system.get_speaker_by_uuid(satellite_uuid) {
                        println!("    📡 Satellite: {} ({})", 
                                 satellite.name(), 
                                 satellite.ip());
                    }
                }
            }
        }
    }
    
    // Display vanished devices if any
    if let Some(vanished) = &topology.vanished_devices {
        if !vanished.devices.is_empty() {
            println!("\n👻 Vanished Devices:");
            for device in &vanished.devices {
                println!("  - {} ({}): {}", 
                         device.zone_name, 
                         device.uuid, 
                         device.reason);
            }
        }
    }
    
    Ok(())
}
```

### Advanced Nested Display with Group Operations

```rust
use sonos::{System, SystemEvent};

struct GroupDisplayManager {
    system: System,
}

impl GroupDisplayManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            system: System::new()?,
        })
    }
    
    pub fn discover_and_display(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔍 Starting discovery...");
        
        // Collect all events from discovery
        let events: Vec<_> = self.system.discover().collect();
        
        // Process events to show discovery progress
        self.process_discovery_events(&events)?;
        
        // Display the final nested structure
        self.display_group_hierarchy()?;
        
        Ok(())
    }
    
    fn process_discovery_events(&self, events: &[SystemEvent]) -> Result<(), String> {
        for event in events {
            match event {
                SystemEvent::SpeakerFound(speaker) => {
                    println!("✅ Found speaker: {} at {}", speaker.name(), speaker.ip());
                },
                SystemEvent::TopologyReady(_) => {
                    println!("🗺️  Topology retrieved successfully");
                },
                SystemEvent::Error(msg) => {
                    println!("❌ Error: {}", msg);
                },
                SystemEvent::DiscoveryComplete => {
                    println!("🏁 Discovery completed");
                },
                SystemEvent::GroupUpdate(group_id, members) => {
                    println!("🔄 Group {} updated with {} members", group_id, members.len());
                },
            }
        }
        Ok(())
    }
    
    fn display_group_hierarchy(&self) -> Result<(), String> {
        if !self.system.has_topology() {
            println!("⚠️  No topology available - showing flat speaker list");
            return self.display_flat_speakers();
        }
        
        let topology = self.system.topology().unwrap();
        
        println!("\n🏠 Sonos System Hierarchy");
        println!("═══════════════════════════");
        
        for zone_group in &topology.zone_groups {
            self.display_single_group(zone_group)?;
        }
        
        Ok(())
    }
    
    fn display_single_group(&self, zone_group: &crate::topology::types::ZoneGroup) -> Result<(), String> {
        println!("\n📁 Zone Group: {}", zone_group.id);
        
        // Find and display coordinator
        if let Some(coordinator) = self.system.get_speaker_by_uuid(&zone_group.coordinator) {
            println!("├── 👑 {} (Coordinator)", coordinator.name());
            println!("│   ├── IP: {}", coordinator.ip());
            println!("│   └── UUID: {}", coordinator.uuid());
        }
        
        // Display other members
        let other_members: Vec<_> = zone_group.members.iter()
            .filter(|member| member.uuid != zone_group.coordinator)
            .collect();
        
        for (index, member) in other_members.iter().enumerate() {
            let is_last = index == other_members.len() - 1;
            let prefix = if is_last { "└──" } else { "├──" };
            let continuation = if is_last { "    " } else { "│   " };
            
            if let Some(speaker) = self.system.get_speaker_by_uuid(&member.uuid) {
                println!("{} 🔊 {} (Member)", prefix, speaker.name());
                println!("{}├── IP: {}", continuation, speaker.ip());
                println!("{}└── UUID: {}", continuation, speaker.uuid());
                
                // Display satellites
                for satellite_uuid in &member.satellites {
                    if let Some(satellite) = self.system.get_speaker_by_uuid(satellite_uuid) {
                        println!("{}    └── 📡 {} (Satellite)", continuation, satellite.name());
                    }
                }
            }
        }
        
        Ok(())
    }
    
    fn display_flat_speakers(&self) -> Result<(), String> {
        println!("\n🔊 All Speakers ({} total)", self.system.speaker_count());
        println!("═══════════════════════");
        
        for (uuid, speaker) in self.system.speakers() {
            println!("• {} ({}) - {}", speaker.name(), speaker.ip(), uuid);
        }
        
        Ok(())
    }
}

// Usage example
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut manager = GroupDisplayManager::new()?;
    manager.discover_and_display()?;
    Ok(())
}
```

## Pattern 2: Flat Speaker Operations

Use this pattern for direct speaker control operations, where you need quick access to individual speakers without group hierarchy concerns.

### Basic Speaker Operations

```rust
use sonos::{System, SystemEvent};

struct SpeakerController {
    system: System,
}

impl SpeakerController {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            system: System::new()?,
        })
    }
    
    pub fn discover(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let events: Vec<_> = self.system.discover().collect();
        
        let speaker_count = events.iter()
            .filter(|e| matches!(e, SystemEvent::SpeakerFound(_)))
            .count();
        
        println!("Discovered {} speakers", speaker_count);
        Ok(())
    }
    
    pub fn list_all_speakers(&self) {
        println!("\n🔊 Available Speakers:");
        println!("═══════════════════════");
        
        if self.system.speaker_count() == 0 {
            println!("No speakers found. Run discovery first.");
            return;
        }
        
        for (index, (uuid, speaker)) in self.system.speakers().iter().enumerate() {
            println!("{}. {} ({})", 
                     index + 1, 
                     speaker.name(), 
                     speaker.ip());
            println!("   UUID: {}", uuid);
        }
    }
    
    pub fn find_speaker_by_name(&self, name: &str) -> Option<&Box<dyn crate::speaker::SpeakerTrait>> {
        self.system.speakers()
            .values()
            .find(|speaker| speaker.name().to_lowercase().contains(&name.to_lowercase()))
    }
    
    pub fn find_speakers_by_ip_range(&self, ip_prefix: &str) -> Vec<&Box<dyn crate::speaker::SpeakerTrait>> {
        self.system.speakers()
            .values()
            .filter(|speaker| speaker.ip().starts_with(ip_prefix))
            .collect()
    }
    
    pub fn get_speaker_by_uuid(&self, uuid: &str) -> Option<&Box<dyn crate::speaker::SpeakerTrait>> {
        self.system.get_speaker_by_uuid(uuid)
    }
    
    pub fn perform_bulk_operation<F>(&self, operation: F) -> Result<(), String>
    where
        F: Fn(&Box<dyn crate::speaker::SpeakerTrait>) -> Result<(), String>,
    {
        let mut errors = Vec::new();
        
        for (uuid, speaker) in self.system.speakers() {
            if let Err(e) = operation(speaker) {
                errors.push(format!("Speaker {} ({}): {}", speaker.name(), uuid, e));
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(format!("Errors occurred:\n{}", errors.join("\n")))
        }
    }
}

// Usage examples
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut controller = SpeakerController::new()?;
    
    // Discover speakers
    controller.discover()?;
    
    // List all speakers
    controller.list_all_speakers();
    
    // Find specific speaker
    if let Some(speaker) = controller.find_speaker_by_name("living room") {
        println!("Found Living Room speaker: {} ({})", speaker.name(), speaker.ip());
    }
    
    // Find speakers in IP range
    let local_speakers = controller.find_speakers_by_ip_range("192.168.1.");
    println!("Found {} speakers in local network", local_speakers.len());
    
    // Perform bulk operation (example: get volume from all speakers)
    controller.perform_bulk_operation(|speaker| {
        println!("Speaker: {} at {}", speaker.name(), speaker.ip());
        // In real implementation, you would call speaker.get_volume() or similar
        Ok(())
    })?;
    
    Ok(())
}
```

### Advanced Speaker Management

```rust
use sonos::{System, SystemEvent};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SpeakerInfo {
    pub uuid: String,
    pub name: String,
    pub ip: String,
    pub is_coordinator: bool,
    pub group_id: Option<String>,
}

pub struct AdvancedSpeakerManager {
    system: System,
    speaker_info_cache: HashMap<String, SpeakerInfo>,
}

impl AdvancedSpeakerManager {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            system: System::new()?,
            speaker_info_cache: HashMap::new(),
        })
    }
    
    pub fn discover_and_analyze(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Run discovery
        let events: Vec<_> = self.system.discover().collect();
        
        // Build speaker info cache
        self.build_speaker_cache()?;
        
        // Analyze the system
        self.analyze_system();
        
        Ok(())
    }
    
    fn build_speaker_cache(&mut self) -> Result<(), String> {
        self.speaker_info_cache.clear();
        
        // Get basic speaker info
        for (uuid, speaker) in self.system.speakers() {
            let mut info = SpeakerInfo {
                uuid: uuid.clone(),
                name: speaker.name().to_string(),
                ip: speaker.ip().to_string(),
                is_coordinator: false,
                group_id: None,
            };
            
            // Add topology information if available
            if let Some(topology) = self.system.topology() {
                for zone_group in &topology.zone_groups {
                    if zone_group.coordinator == *uuid {
                        info.is_coordinator = true;
                        info.group_id = Some(zone_group.id.clone());
                    } else if zone_group.members.iter().any(|m| m.uuid == *uuid) {
                        info.group_id = Some(zone_group.id.clone());
                    }
                }
            }
            
            self.speaker_info_cache.insert(uuid.clone(), info);
        }
        
        Ok(())
    }
    
    fn analyze_system(&self) {
        println!("\n📊 System Analysis");
        println!("═══════════════════");
        
        let total_speakers = self.system.speaker_count();
        let coordinators = self.speaker_info_cache.values()
            .filter(|info| info.is_coordinator)
            .count();
        let grouped_speakers = self.speaker_info_cache.values()
            .filter(|info| info.group_id.is_some())
            .count();
        let standalone_speakers = total_speakers - grouped_speakers;
        
        println!("Total Speakers: {}", total_speakers);
        println!("Coordinators: {}", coordinators);
        println!("Grouped Speakers: {}", grouped_speakers);
        println!("Standalone Speakers: {}", standalone_speakers);
        
        if self.system.has_topology() {
            let topology = self.system.topology().unwrap();
            println!("Zone Groups: {}", topology.zone_group_count());
            
            if let Some(vanished) = &topology.vanished_devices {
                println!("Vanished Devices: {}", vanished.devices.len());
            }
        } else {
            println!("⚠️  Topology not available");
        }
    }
    
    pub fn get_coordinators(&self) -> Vec<&SpeakerInfo> {
        self.speaker_info_cache.values()
            .filter(|info| info.is_coordinator)
            .collect()
    }
    
    pub fn get_speakers_in_group(&self, group_id: &str) -> Vec<&SpeakerInfo> {
        self.speaker_info_cache.values()
            .filter(|info| info.group_id.as_ref() == Some(&group_id.to_string()))
            .collect()
    }
    
    pub fn get_standalone_speakers(&self) -> Vec<&SpeakerInfo> {
        self.speaker_info_cache.values()
            .filter(|info| info.group_id.is_none())
            .collect()
    }
    
    pub fn find_speaker_info(&self, query: &str) -> Vec<&SpeakerInfo> {
        let query_lower = query.to_lowercase();
        self.speaker_info_cache.values()
            .filter(|info| {
                info.name.to_lowercase().contains(&query_lower) ||
                info.ip.contains(query) ||
                info.uuid.to_lowercase().contains(&query_lower)
            })
            .collect()
    }
    
    pub fn display_detailed_info(&self) {
        println!("\n🔍 Detailed Speaker Information");
        println!("═══════════════════════════════");
        
        for info in self.speaker_info_cache.values() {
            println!("\n🔊 {}", info.name);
            println!("   UUID: {}", info.uuid);
            println!("   IP: {}", info.ip);
            println!("   Role: {}", if info.is_coordinator { "Coordinator" } else { "Member" });
            
            if let Some(group_id) = &info.group_id {
                println!("   Group: {}", group_id);
            } else {
                println!("   Group: Standalone");
            }
            
            // Get actual speaker reference for additional operations
            if let Some(speaker) = self.system.get_speaker_by_uuid(&info.uuid) {
                // You can perform additional operations here
                println!("   Status: Available");
            }
        }
    }
}

// Usage example
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut manager = AdvancedSpeakerManager::new()?;
    
    // Discover and analyze
    manager.discover_and_analyze()?;
    
    // Display detailed information
    manager.display_detailed_info();
    
    // Find coordinators
    let coordinators = manager.get_coordinators();
    println!("\n👑 Coordinators:");
    for coord in coordinators {
        println!("  - {} ({})", coord.name, coord.ip);
    }
    
    // Find standalone speakers
    let standalone = manager.get_standalone_speakers();
    println!("\n🔊 Standalone Speakers:");
    for speaker in standalone {
        println!("  - {} ({})", speaker.name, speaker.ip);
    }
    
    Ok(())
}
```

## Event Handling Examples

### Real-time Discovery Updates

```rust
use sonos::{System, SystemEvent};

pub struct RealTimeDiscoveryHandler {
    system: System,
}

impl RealTimeDiscoveryHandler {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            system: System::new()?,
        })
    }
    
    pub fn start_discovery_with_updates(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 Starting real-time discovery...\n");
        
        // Process events as they come
        for event in self.system.discover() {
            self.handle_event(&event)?;
        }
        
        println!("\n✅ Discovery completed successfully!");
        self.display_final_summary();
        
        Ok(())
    }
    
    fn handle_event(&self, event: &SystemEvent) -> Result<(), Box<dyn std::error::Error>> {
        match event {
            SystemEvent::SpeakerFound(speaker) => {
                println!("🔊 Speaker discovered: {} at {}", 
                         speaker.name(), 
                         speaker.ip());
                
                // Update UI with new speaker
                self.update_speaker_list(speaker);
            },
            
            SystemEvent::TopologyReady(topology) => {
                println!("🗺️  Topology loaded: {} groups, {} total speakers", 
                         topology.zone_group_count(),
                         topology.total_speaker_count());
                
                // Update UI with group information
                self.update_group_display(topology);
            },
            
            SystemEvent::Error(msg) => {
                println!("❌ Error occurred: {}", msg);
                
                // Handle error in UI (show notification, etc.)
                self.handle_error(msg);
            },
            
            SystemEvent::DiscoveryComplete => {
                println!("🏁 Discovery process completed");
                
                // Finalize UI updates
                self.finalize_discovery();
            },
            
            SystemEvent::GroupUpdate(group_id, members) => {
                println!("🔄 Group {} updated: {} members", 
                         group_id, 
                         members.len());
                
                // Update specific group in UI
                self.update_group_members(group_id, members);
            },
        }
        
        Ok(())
    }
    
    fn update_speaker_list(&self, speaker: &crate::speaker::Speaker) {
        // In a real UI, you would update your speaker list widget/component
        println!("   └── Added to speaker list");
    }
    
    fn update_group_display(&self, topology: &crate::topology::Topology) {
        // In a real UI, you would update your group hierarchy display
        println!("   └── Group hierarchy updated");
        
        // Example: Update each group
        for group in &topology.zone_groups {
            println!("       Group {}: {} members", 
                     group.id, 
                     group.members.len());
        }
    }
    
    fn handle_error(&self, error_msg: &str) {
        // In a real UI, you would show error notifications
        println!("   └── Error logged and user notified");
    }
    
    fn finalize_discovery(&self) {
        // In a real UI, you would hide loading indicators, enable controls, etc.
        println!("   └── UI finalized and ready for interaction");
    }
    
    fn update_group_members(&self, group_id: &str, members: &[String]) {
        // In a real UI, you would update the specific group's member list
        println!("   └── Group {} member list updated", group_id);
    }
    
    fn display_final_summary(&self) {
        println!("\n📋 Final Summary");
        println!("═══════════════");
        println!("Speakers found: {}", self.system.speaker_count());
        println!("Topology available: {}", if self.system.has_topology() { "Yes" } else { "No" });
        
        if self.system.has_topology() {
            let topology = self.system.topology().unwrap();
            println!("Zone groups: {}", topology.zone_group_count());
        }
    }
}

// Usage example
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut handler = RealTimeDiscoveryHandler::new()?;
    handler.start_discovery_with_updates()?;
    Ok(())
}
```

### Event-Driven UI State Management

```rust
use sonos::{System, SystemEvent};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum UIState {
    Idle,
    Discovering,
    Ready,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct UIModel {
    pub state: UIState,
    pub speakers: HashMap<String, SpeakerDisplayInfo>,
    pub groups: Vec<GroupDisplayInfo>,
    pub discovery_progress: f32,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SpeakerDisplayInfo {
    pub uuid: String,
    pub name: String,
    pub ip: String,
    pub is_online: bool,
}

#[derive(Debug, Clone)]
pub struct GroupDisplayInfo {
    pub id: String,
    pub coordinator_uuid: String,
    pub member_uuids: Vec<String>,
}

pub struct EventDrivenUI {
    system: System,
    model: UIModel,
    expected_speakers: Option<usize>,
}

impl EventDrivenUI {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            system: System::new()?,
            model: UIModel {
                state: UIState::Idle,
                speakers: HashMap::new(),
                groups: Vec::new(),
                discovery_progress: 0.0,
                error_message: None,
            },
            expected_speakers: None,
        })
    }
    
    pub fn start_discovery(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Update UI state to discovering
        self.model.state = UIState::Discovering;
        self.model.speakers.clear();
        self.model.groups.clear();
        self.model.discovery_progress = 0.0;
        self.model.error_message = None;
        
        self.render_ui();
        
        // Process discovery events
        let events: Vec<_> = self.system.discover().collect();
        
        // Calculate expected speakers for progress tracking
        let speaker_events = events.iter()
            .filter(|e| matches!(e, SystemEvent::SpeakerFound(_)))
            .count();
        self.expected_speakers = Some(speaker_events);
        
        // Process each event and update UI
        for (index, event) in events.iter().enumerate() {
            self.handle_discovery_event(event, index, events.len())?;
            self.render_ui();
            
            // Simulate UI update delay (remove in real implementation)
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        
        Ok(())
    }
    
    fn handle_discovery_event(
        &mut self, 
        event: &SystemEvent, 
        event_index: usize, 
        total_events: usize
    ) -> Result<(), Box<dyn std::error::Error>> {
        
        match event {
            SystemEvent::SpeakerFound(speaker) => {
                // Add speaker to UI model
                let speaker_info = SpeakerDisplayInfo {
                    uuid: speaker.uuid().to_string(),
                    name: speaker.name().to_string(),
                    ip: speaker.ip().to_string(),
                    is_online: true,
                };
                
                self.model.speakers.insert(speaker.uuid().to_string(), speaker_info);
                
                // Update progress
                let speaker_count = self.model.speakers.len();
                if let Some(expected) = self.expected_speakers {
                    self.model.discovery_progress = (speaker_count as f32 / expected as f32) * 0.8; // 80% for speakers
                }
            },
            
            SystemEvent::TopologyReady(topology) => {
                // Update groups in UI model
                self.model.groups.clear();
                
                for zone_group in &topology.zone_groups {
                    let group_info = GroupDisplayInfo {
                        id: zone_group.id.clone(),
                        coordinator_uuid: zone_group.coordinator.clone(),
                        member_uuids: zone_group.members.iter()
                            .map(|m| m.uuid.clone())
                            .collect(),
                    };
                    
                    self.model.groups.push(group_info);
                }
                
                // Update progress to 90% when topology is ready
                self.model.discovery_progress = 0.9;
            },
            
            SystemEvent::Error(msg) => {
                self.model.error_message = Some(msg.clone());
                
                // Don't change state to Error unless it's a critical error
                // Topology errors are non-critical
                if !msg.contains("Topology retrieval failed") {
                    self.model.state = UIState::Error(msg.clone());
                }
            },
            
            SystemEvent::DiscoveryComplete => {
                self.model.state = UIState::Ready;
                self.model.discovery_progress = 1.0;
                self.model.error_message = None; // Clear non-critical errors
            },
            
            SystemEvent::GroupUpdate(group_id, members) => {
                // Update specific group
                if let Some(group) = self.model.groups.iter_mut()
                    .find(|g| g.id == *group_id) {
                    group.member_uuids = members.clone();
                }
            },
        }
        
        Ok(())
    }
    
    fn render_ui(&self) {
        // Clear screen (in a real UI framework, you'd update components)
        print!("\x1B[2J\x1B[1;1H");
        
        println!("🎵 Sonos Controller");
        println!("═══════════════════");
        
        // Display state
        match &self.model.state {
            UIState::Idle => println!("Status: Ready to discover"),
            UIState::Discovering => {
                println!("Status: Discovering... ({:.0}%)", self.model.discovery_progress * 100.0);
                self.render_progress_bar();
            },
            UIState::Ready => println!("Status: Ready"),
            UIState::Error(msg) => println!("Status: Error - {}", msg),
        }
        
        // Display error message if any
        if let Some(error) = &self.model.error_message {
            println!("⚠️  Warning: {}", error);
        }
        
        // Display speakers
        if !self.model.speakers.is_empty() {
            println!("\n🔊 Speakers ({}):", self.model.speakers.len());
            for speaker in self.model.speakers.values() {
                let status = if speaker.is_online { "🟢" } else { "🔴" };
                println!("  {} {} ({})", status, speaker.name, speaker.ip);
            }
        }
        
        // Display groups
        if !self.model.groups.is_empty() {
            println!("\n📁 Groups ({}):", self.model.groups.len());
            for group in &self.model.groups {
                println!("  Group: {}", group.id);
                
                // Show coordinator
                if let Some(coord) = self.model.speakers.get(&group.coordinator_uuid) {
                    println!("    👑 {}", coord.name);
                }
                
                // Show members
                for member_uuid in &group.member_uuids {
                    if member_uuid != &group.coordinator_uuid {
                        if let Some(member) = self.model.speakers.get(member_uuid) {
                            println!("    🔊 {}", member.name);
                        }
                    }
                }
            }
        }
        
        println!();
    }
    
    fn render_progress_bar(&self) {
        let width = 30;
        let filled = (self.model.discovery_progress * width as f32) as usize;
        let empty = width - filled;
        
        print!("Progress: [");
        print!("{}", "█".repeat(filled));
        print!("{}", "░".repeat(empty));
        println!("]");
    }
    
    pub fn get_model(&self) -> &UIModel {
        &self.model
    }
    
    pub fn get_system(&self) -> &System {
        &self.system
    }
}

// Usage example
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut ui = EventDrivenUI::new()?;
    
    println!("Starting event-driven discovery...");
    ui.start_discovery()?;
    
    println!("Discovery completed! Final state:");
    let model = ui.get_model();
    println!("State: {:?}", model.state);
    println!("Speakers: {}", model.speakers.len());
    println!("Groups: {}", model.groups.len());
    
    Ok(())
}
```

## Migration Guide

### Updating Existing Code

#### Before (Old API)
```rust
// Old way - system was consumed
let system = System::new()?;
let events: Vec<_> = system.discover().collect();
// system is no longer available here
```

#### After (New API)
```rust
// New way - system uses mutable reference
let mut system = System::new()?;
let events: Vec<_> = system.discover().collect();
// system is still available for further operations
let speaker_count = system.speaker_count();
```

#### Event Handling Updates
```rust
// Update event matching
for event in events {
    match event {
        // Old: SystemEvent::Found(speaker) => { ... }
        SystemEvent::SpeakerFound(speaker) => {
            println!("Found speaker: {}", speaker.name());
        },
        
        // New events to handle
        SystemEvent::TopologyReady(topology) => {
            println!("Topology ready with {} groups", topology.zone_group_count());
        },
        
        SystemEvent::DiscoveryComplete => {
            println!("Discovery finished");
        },
        
        SystemEvent::Error(msg) => {
            println!("Error: {}", msg);
        },
        
        SystemEvent::GroupUpdate(group_id, members) => {
            println!("Group {} updated", group_id);
        },
    }
}
```

## Best Practices

### 1. Always Check Topology Availability
```rust
if system.has_topology() {
    // Use nested group display
    display_nested_groups(&system)?;
} else {
    // Fall back to flat display
    display_flat_speakers(&system)?;
}
```

### 2. Handle Discovery Errors Gracefully
```rust
let events: Vec<_> = system.discover().collect();
let errors: Vec<_> = events.iter()
    .filter_map(|e| match e {
        SystemEvent::Error(msg) => Some(msg),
        _ => None,
    })
    .collect();

if !errors.is_empty() {
    println!("Warnings during discovery:");
    for error in errors {
        println!("  - {}", error);
    }
}
```

### 3. Combine Both Patterns
```rust
// Use topology for display, flat access for operations
fn control_speaker_in_group(system: &System, group_id: &str, speaker_name: &str) -> Result<(), String> {
    // Find the group using topology
    if let Some(topology) = system.topology() {
        if let Some(zone_group) = topology.zone_groups.iter().find(|g| g.id == group_id) {
            // Find the specific speaker in the group
            for member in &zone_group.members {
                if let Some(speaker) = system.get_speaker_by_uuid(&member.uuid) {
                    if speaker.name().to_lowercase().contains(&speaker_name.to_lowercase()) {
                        // Perform operation using flat access
                        println!("Controlling {} in group {}", speaker.name(), group_id);
                        // speaker.play(), speaker.pause(), etc.
                        return Ok(());
                    }
                }
            }
        }
    }
    
    Err(format!("Speaker '{}' not found in group '{}'", speaker_name, group_id))
}
```

### 4. Efficient Speaker Lookups
```rust
// Cache frequently accessed speakers
let mut speaker_cache: HashMap<String, &Box<dyn SpeakerTrait>> = HashMap::new();

// Build cache once
for (uuid, speaker) in system.speakers() {
    speaker_cache.insert(speaker.name().to_lowercase(), speaker);
}

// Fast lookups by name
if let Some(speaker) = speaker_cache.get(&"living room".to_string()) {
    // Use speaker
}
```

This documentation provides comprehensive examples for both UI integration patterns and demonstrates how to handle the new event system effectively. The examples show real-world usage scenarios and best practices for building robust Sonos controller applications.

## Complete Working Examples

For complete, runnable examples demonstrating these patterns, see:

- `examples/ui-integration-demo.rs` - Comprehensive demonstration of all integration patterns
- `examples/sonos-rs-demo.rs` - Basic usage example

Run examples with:
```bash
cargo run --example ui-integration-demo
```

## Testing Your Integration

### Unit Testing with Mock Speakers

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sonos::speaker::mock::MockSpeakerBuilder;

    #[test]
    fn test_nested_display_with_mock_data() {
        let mut system = System::new().unwrap();
        
        // Add mock speakers for testing
        let speaker1 = Box::new(
            MockSpeakerBuilder::new()
                .uuid("RINCON_123")
                .name("Living Room")
                .ip("192.168.1.100")
                .build()
        );
        
        system.add_speaker_for_test(speaker1);
        
        // Test your display logic
        assert_eq!(system.speaker_count(), 1);
        assert!(system.get_speaker_by_uuid("RINCON_123").is_some());
    }

    #[test]
    fn test_group_hierarchy_access() {
        let mut system = System::new().unwrap();
        
        // Add test speakers and topology
        let speaker1 = Box::new(
            MockSpeakerBuilder::new()
                .uuid("RINCON_123")
                .name("Living Room")
                .ip("192.168.1.100")
                .build()
        );
        
        let speaker2 = Box::new(
            MockSpeakerBuilder::new()
                .uuid("RINCON_456")
                .name("Kitchen")
                .ip("192.168.1.101")
                .build()
        );
        
        system.add_speaker_for_test(speaker1);
        system.add_speaker_for_test(speaker2);
        
        // Test nested group access patterns
        if system.has_topology() {
            let topology = system.topology().unwrap();
            for zone_group in &topology.zone_groups {
                // Verify coordinator access
                if let Some(coordinator) = system.get_speaker_by_uuid(&zone_group.coordinator) {
                    assert!(!coordinator.name().is_empty());
                }
                
                // Verify member access
                for member in &zone_group.members {
                    if let Some(speaker) = system.get_speaker_by_uuid(&member.uuid) {
                        assert!(!speaker.ip().is_empty());
                    }
                }
            }
        }
    }
}
```

### Integration Testing

```rust
#[test]
fn test_discovery_flow() {
    let mut system = System::new().unwrap();
    let events: Vec<_> = system.discover().collect();
    
    // Verify discovery completed
    assert!(events.iter().any(|e| matches!(e, SystemEvent::DiscoveryComplete)));
    
    // Verify system state is consistent
    let speaker_events = events.iter()
        .filter(|e| matches!(e, SystemEvent::SpeakerFound(_)))
        .count();
    
    // System should have same number of speakers as events
    assert_eq!(system.speaker_count(), speaker_events);
    
    // Verify topology handling
    let topology_events = events.iter()
        .filter(|e| matches!(e, SystemEvent::TopologyReady(_)))
        .count();
    
    if topology_events > 0 {
        assert!(system.has_topology());
    }
}

#[test]
fn test_event_handling_robustness() {
    let mut system = System::new().unwrap();
    let events: Vec<_> = system.discover().collect();
    
    // Test that all events are properly formed
    for event in &events {
        match event {
            SystemEvent::SpeakerFound(speaker) => {
                assert!(!speaker.name().is_empty());
                assert!(!speaker.ip().is_empty());
                assert!(!speaker.uuid().is_empty());
            },
            SystemEvent::TopologyReady(topology) => {
                assert!(topology.zone_group_count() > 0);
            },
            SystemEvent::Error(msg) => {
                assert!(!msg.is_empty());
            },
            SystemEvent::DiscoveryComplete => {
                // Should be the last event
                let last_event = events.last().unwrap();
                assert!(matches!(last_event, SystemEvent::DiscoveryComplete));
            },
            SystemEvent::GroupUpdate(group_id, members) => {
                assert!(!group_id.is_empty());
                assert!(!members.is_empty());
            },
        }
    }
}
```

### Performance Testing

```rust
#[test]
fn test_speaker_lookup_performance() {
    let mut system = System::new().unwrap();
    
    // Add many test speakers
    for i in 0..1000 {
        let speaker = Box::new(
            MockSpeakerBuilder::new()
                .uuid(&format!("RINCON_{:03}", i))
                .name(&format!("Speaker {}", i))
                .ip(&format!("192.168.1.{}", i % 255))
                .build()
        );
        system.add_speaker_for_test(speaker);
    }
    
    // Test lookup performance
    let start = std::time::Instant::now();
    
    for i in 0..1000 {
        let uuid = format!("RINCON_{:03}", i);
        assert!(system.get_speaker_by_uuid(&uuid).is_some());
    }
    
    let duration = start.elapsed();
    println!("1000 UUID lookups took: {:?}", duration);
    
    // Should be very fast (under 1ms for 1000 lookups)
    assert!(duration.as_millis() < 10);
}
```

## Troubleshooting

### Common Issues

#### 1. No Speakers Found
```rust
if system.speaker_count() == 0 {
    println!("No speakers found. Check:");
    println!("- Network connectivity");
    println!("- Firewall settings");
    println!("- Speakers are powered on");
    println!("- Speakers are on same network");
}
```

#### 2. Topology Not Available
```rust
if !system.has_topology() {
    println!("Topology not available. This could mean:");
    println!("- No speakers were found");
    println!("- Network communication issues");
    println!("- Speakers are in setup mode");
    
    // Fall back to flat display
    display_flat_speakers(&system);
}
```

#### 3. Handling Discovery Errors
```rust
let events: Vec<_> = system.discover().collect();
let errors: Vec<_> = events.iter()
    .filter_map(|e| match e {
        SystemEvent::Error(msg) => Some(msg),
        _ => None,
    })
    .collect();

if !errors.is_empty() {
    println!("Discovery completed with warnings:");
    for error in errors {
        if error.contains("Topology retrieval failed") {
            println!("  ⚠️  Topology unavailable: {}", error);
        } else {
            println!("  ❌ Error: {}", error);
        }
    }
}
```

### Debug Logging

Enable debug logging to troubleshoot discovery issues:

```rust
use log::LevelFilter;

fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(LevelFilter::Debug)
        .init();
    
    // Your discovery code here
}
```

### Network Diagnostics

```rust
fn diagnose_network_issues(system: &System) {
    println!("🔍 Network Diagnostics:");
    
    // Check if any speakers were found
    if system.speaker_count() == 0 {
        println!("  ❌ No speakers discovered");
        println!("     - Check network connectivity");
        println!("     - Verify speakers are powered on");
        return;
    }
    
    // Check topology availability
    if !system.has_topology() {
        println!("  ⚠️  Speakers found but no topology");
        println!("     - May indicate network communication issues");
        println!("     - Try accessing speakers individually");
    } else {
        println!("  ✅ Topology available");
    }
    
    // Check speaker distribution
    let mut ip_ranges: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for (_, speaker) in system.speakers() {
        let ip_parts: Vec<&str> = speaker.ip().split('.').collect();
        if ip_parts.len() >= 3 {
            let range = format!("{}.{}.{}.x", ip_parts[0], ip_parts[1], ip_parts[2]);
            *ip_ranges.entry(range).or_insert(0) += 1;
        }
    }
    
    println!("  📊 Speaker distribution:");
    for (range, count) in ip_ranges {
        println!("     {} range: {} speakers", range, count);
    }
}
```

This comprehensive documentation covers all aspects of UI integration with the enhanced Sonos System API, providing practical examples, testing strategies, and troubleshooting guidance for developers building Sonos controller applications.