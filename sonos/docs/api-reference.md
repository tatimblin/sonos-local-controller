# API Reference

## System Struct

The main entry point for Sonos speaker discovery and management.

### Constructor

```rust
impl System {
    pub fn new() -> Result<Self, std::io::Error>
}
```

Creates a new System instance for speaker discovery and management.

### Discovery

```rust
pub fn discover(&mut self) -> impl Iterator<Item = SystemEvent> + '_
```

**Breaking Change**: Now takes `&mut self` instead of consuming `self`.

Discovers Sonos speakers on the network and returns an iterator of events. The System instance remains available after discovery completes.

**Returns**: Iterator yielding `SystemEvent` instances during discovery process.

### State Access Methods

```rust
pub fn speakers(&self) -> &HashMap<String, Box<dyn SpeakerTrait>>
```

Returns a reference to the internal speaker HashMap. Keys are speaker UUIDs.

```rust
pub fn topology(&self) -> Option<&Topology>
```

Returns an optional reference to the current topology information.

```rust
pub fn has_topology(&self) -> bool
```

Returns `true` if topology information is available.

```rust
pub fn speaker_count(&self) -> usize
```

Returns the number of discovered speakers.

```rust
pub fn get_speaker_by_uuid(&self, uuid: &str) -> Option<&Box<dyn SpeakerTrait>>
```

Retrieves a speaker by its UUID. Returns `None` if not found.

## SystemEvent Enum

Events emitted during discovery and system changes.

```rust
#[derive(Debug)]
pub enum SystemEvent {
    SpeakerFound(Speaker),           // Individual speaker discovered
    TopologyReady(Topology),         // Complete topology available  
    Error(String),                   // General errors
    DiscoveryComplete,               // Discovery process finished
    GroupUpdate(String, Vec<String>), // Group membership changes
}
```

### Event Details

#### `SpeakerFound(Speaker)`
- **When**: Emitted for each successfully discovered speaker
- **Data**: Complete `Speaker` instance with name, IP, UUID, etc.
- **Usage**: Update UI speaker lists, enable speaker-specific controls

#### `TopologyReady(Topology)`
- **When**: Emitted when topology is successfully retrieved (typically after first speaker)
- **Data**: Complete `Topology` with zone groups and hierarchy information
- **Usage**: Enable group-based displays and operations

#### `Error(String)`
- **When**: Emitted for various error conditions (speaker creation failures, topology retrieval failures, etc.)
- **Data**: Human-readable error message
- **Usage**: Log errors, show user notifications, handle gracefully

#### `DiscoveryComplete`
- **When**: Always emitted as the final event, regardless of success/failure
- **Data**: None
- **Usage**: Hide loading indicators, finalize UI state, enable user interactions

#### `GroupUpdate(String, Vec<String>)`
- **When**: Emitted when group membership changes (future use)
- **Data**: Group ID and list of member UUIDs
- **Usage**: Update group displays in real-time

## Breaking Changes from Previous Versions

### Method Signatures

| Old | New | Impact |
|-----|-----|--------|
| `discover(self)` | `discover(&mut self)` | System instance remains available after discovery |

### Event Names

| Old | New | Impact |
|-----|-----|--------|
| `SystemEvent::Found(Speaker)` | `SystemEvent::SpeakerFound(Speaker)` | Update pattern matching |

### New Events

- `SystemEvent::TopologyReady(Topology)` - Handle topology availability
- `SystemEvent::DiscoveryComplete` - Handle discovery completion

## Migration Guide

### Before (Old API)
```rust
let system = System::new()?;
let events: Vec<_> = system.discover().collect();
// system is consumed - cannot use anymore

for event in events {
    match event {
        SystemEvent::Found(speaker) => {
            println!("Found: {}", speaker.name());
        },
        SystemEvent::Error(msg) => {
            println!("Error: {}", msg);
        },
    }
}
```

### After (New API)
```rust
let mut system = System::new()?;
let events: Vec<_> = system.discover().collect();
// system is still available

for event in events {
    match event {
        SystemEvent::SpeakerFound(speaker) => {
            println!("Found: {}", speaker.name());
        },
        SystemEvent::TopologyReady(topology) => {
            println!("Topology: {} groups", topology.zone_group_count());
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

// System is still available for queries
println!("Total speakers: {}", system.speaker_count());
if system.has_topology() {
    // Use topology information
}
```

## Usage Patterns

### Pattern 1: Event-Driven Discovery
```rust
let mut system = System::new()?;

for event in system.discover() {
    match event {
        SystemEvent::SpeakerFound(speaker) => {
            // Update UI immediately as speakers are found
            update_speaker_list(speaker);
        },
        SystemEvent::TopologyReady(topology) => {
            // Enable group-based UI features
            enable_group_controls(topology);
        },
        SystemEvent::DiscoveryComplete => {
            // Finalize UI state
            hide_loading_indicator();
        },
        SystemEvent::Error(msg) => {
            // Handle errors gracefully
            show_warning(msg);
        },
        _ => {}
    }
}
```

### Pattern 2: Batch Processing
```rust
let mut system = System::new()?;
let events: Vec<_> = system.discover().collect();

// Process all events at once
let speakers: Vec<_> = events.iter()
    .filter_map(|e| match e {
        SystemEvent::SpeakerFound(speaker) => Some(speaker),
        _ => None,
    })
    .collect();

let has_topology = events.iter()
    .any(|e| matches!(e, SystemEvent::TopologyReady(_)));

// Update UI with complete state
update_ui(speakers, has_topology);
```

### Pattern 3: State-Based Access
```rust
let mut system = System::new()?;
let _events: Vec<_> = system.discover().collect();

// Query final state
println!("Speakers: {}", system.speaker_count());

// Access individual speakers
for (uuid, speaker) in system.speakers() {
    println!("Speaker: {} ({})", speaker.name(), uuid);
}

// Access topology if available
if let Some(topology) = system.topology() {
    for group in &topology.zone_groups {
        println!("Group: {}", group.id);
    }
}
```

## Error Handling

### Non-Fatal Errors
- Topology retrieval failures (discovery continues)
- Individual speaker creation failures
- Network timeouts for specific speakers

### Error Recovery
```rust
let events: Vec<_> = system.discover().collect();

let errors: Vec<_> = events.iter()
    .filter_map(|e| match e {
        SystemEvent::Error(msg) => Some(msg),
        _ => None,
    })
    .collect();

if !errors.is_empty() {
    println!("Discovery completed with {} warnings", errors.len());
    for error in errors {
        if error.contains("Topology retrieval failed") {
            println!("  Warning: {}", error);
        } else {
            println!("  Error: {}", error);
        }
    }
}

// System is still functional even with errors
if system.speaker_count() > 0 {
    println!("Found {} speakers despite errors", system.speaker_count());
}
```

## Performance Considerations

### Speaker Lookups
- `get_speaker_by_uuid()` is O(1) HashMap lookup
- `speakers()` returns reference to avoid cloning
- Speaker iteration is efficient over the HashMap

### Memory Usage
- Speakers stored once in HashMap
- Topology references speaker UUIDs (no duplication)
- Events contain owned data for flexibility

### Network Efficiency
- Topology retrieved only once (from first speaker)
- SSDP discovery uses standard multicast
- Concurrent speaker processing where possible

## Thread Safety

The `System` struct is **not** thread-safe. Use appropriate synchronization if accessing from multiple threads:

```rust
use std::sync::{Arc, Mutex};

let system = Arc::new(Mutex::new(System::new()?));

// In thread 1
{
    let mut sys = system.lock().unwrap();
    let _events: Vec<_> = sys.discover().collect();
}

// In thread 2
{
    let sys = system.lock().unwrap();
    println!("Speakers: {}", sys.speaker_count());
}
```