# Sonos Examples

This directory contains example programs demonstrating various features of the Sonos library.

## Stream All Speakers

The `stream-all-speakers.rs` example demonstrates how to:
- Discover all Sonos speakers on your network
- Create an event stream for all speakers simultaneously
- Iterate over the event stream as a user would in a real application
- Handle different types of events with comprehensive formatting

### Running the Stream All Speakers Example

```bash
cargo run --example stream-all-speakers
```

### Features

- **Multi-Speaker Streaming**: Connects to all discovered speakers simultaneously
- **Comprehensive Event Handling**: Processes all types of events (playback, volume, track changes, etc.)
- **Multiple Iteration Methods**: Shows different ways to consume events (recv_timeout, try_recv, iterator)
- **Rich Event Formatting**: Detailed event descriptions with icons and context
- **Robust Error Handling**: Gracefully handles speaker connection failures
- **Troubleshooting Tips**: Provides helpful guidance when no events are received

### Output Format

```
🎵 Sonos Stream All Speakers Example 🎵
═══════════════════════════════════════════════════════════════
🔍 Discovering all Sonos speakers on the network...
✅ Found 3 speakers:
   1. Living Room (Sonos One) at 192.168.1.100:1400
   2. Kitchen (Sonos Play:1) at 192.168.1.101:1400
   3. Bedroom (Sonos One SL) at 192.168.1.102:1400

🔧 Stream Configuration:
   • Services: [AVTransport, RenderingControl]
   • Callback ports: (8080, 8090)
   • Subscription timeout: 1800s
   • Buffer size: 2000

🚀 Starting event stream for all speakers...
✅ Event stream created successfully!
✅ Event stream is active and ready to receive events

📡 Listening for events from all speakers...
   Press Ctrl+C to stop
   Try playing/pausing music, changing volume, or grouping speakers!
═══════════════════════════════════════════════════════════════

🎵 Event #1: ▶️ Started playing (Speaker: SpeakerId(123456))
🎵 Event #2: 🔊 Volume changed to 45% [████░░░░░░] (Speaker: SpeakerId(123456))
🎵 Event #3: 🎶 Now playing: "Song Title" by Artist Name from Album Name (Speaker: SpeakerId(123456))
🎵 Event #4: 👥 Group topology changed: 2 groups with 3 total speakers
🎵 Event #5: 🔇 Muted (Speaker: SpeakerId(789012))
   ─────────────────────────────────────────────────
```

### Event Types Handled

- **Playback State Changes**: Play/pause/stop with descriptive text and icons
- **Volume Changes**: Volume level with visual bars and appropriate volume icons
- **Mute Changes**: Mute/unmute status with clear descriptions
- **Track Changes**: Detailed track information including artist, title, and album
- **Position Updates**: Playback position in MM:SS format
- **Group Topology**: Changes to speaker grouping with member counts
- **Transport Info**: Detailed transport state and status information
- **Subscription Errors**: Clear error reporting with troubleshooting context

### Configuration

The example uses comprehensive streaming settings:
- **Services**: AVTransport and RenderingControl (for full event coverage)
- **Callback Ports**: 8080-8090 range
- **Subscription Timeout**: 30 minutes
- **Buffer Size**: 2000 events
- **Retry Attempts**: 3 with exponential backoff

### Usage Patterns

The example demonstrates three ways to consume events:

1. **recv_timeout()** (recommended): Non-blocking with timeout
```rust
if let Some(event) = event_stream.recv_timeout(Duration::from_millis(1000)) {
    println!("Event: {}", format_event(&event));
}
```

2. **try_recv()**: Completely non-blocking for other work
```rust
if let Some(event) = event_stream.try_recv() {
    println!("Event: {}", format_event(&event));
} else {
    // Do other work
}
```

3. **iter()**: Blocking iterator (simplest but blocks thread)
```rust
for event in event_stream.iter() {
    println!("Event: {}", format_event(&event));
}
```

### Notes

- Discovers all speakers with a 5-second timeout
- Continues even if some speakers fail to connect
- Provides troubleshooting tips after 30 seconds with no events
- Shows periodic status updates when waiting for events
- Handles satellite/bonded speakers gracefully

## Streaming Events

The `streaming-events.rs` example demonstrates how to:
- Set up real-time event streaming from a Sonos speaker
- Subscribe to UPnP events using the streaming manager
- Display live events as they occur (playback changes, volume changes, etc.)

### Running the Streaming Events Example

```bash
cargo run --example streaming-events
```

### Features

- **Real-time Events**: Receives live UPnP events from Sonos speakers
- **Single Speaker Focus**: Connects to one speaker for focused event monitoring
- **Event Formatting**: Pretty-prints different types of events with icons and details
- **Automatic Discovery**: Finds speakers and selects the first one automatically
- **Subscription Management**: Handles UPnP subscription lifecycle automatically

### Output Format

The example displays events as they occur:
```
🎵 Sonos Streaming Events Demo 🎵
═══════════════════════════════════════════════════════════════
🔍 Discovering Sonos speakers...
✅ Found 2 speakers:
   1. Living Room (Sonos One) at 192.168.1.100:1400
   2. Kitchen (Sonos Play:1) at 192.168.1.101:1400

🎯 Selected speaker: Living Room for event streaming
📡 Listening for events from Living Room...
   Press Ctrl+C to stop
   Try playing/pausing music on your Sonos speaker to see events!
═══════════════════════════════════════════════════════════════

🎵 Event #1: ▶️ Playback: Playing (Speaker: SpeakerId(123456))
🎵 Event #2: 🔊 Volume: 45% [████░░░░░░] (Speaker: SpeakerId(123456))
🎵 Event #3: 🎶 Track: Artist Name - Song Title (Speaker: SpeakerId(123456))
🎵 Event #4: ⏱️  Position: 01:23 (Speaker: SpeakerId(123456))
   ─────────────────────────────────────────────────
```

### Event Types

The example handles these event types:
- **Playback State**: Play/pause/stop changes with icons (▶️⏸️⏹️🔄)
- **Volume Changes**: Volume level with visual bar
- **Mute Changes**: Mute on/off status (🔇🔊)
- **Track Changes**: New track information with artist and title
- **Position Updates**: Playback position in minutes:seconds
- **Group Topology**: Changes to speaker grouping
- **Transport Info**: Detailed transport state changes
- **Subscription Errors**: Connection or subscription issues

### Configuration

The example uses these streaming settings:
- **Services**: AVTransport only (for playback events)
- **Callback Ports**: 8080-8090 range
- **Subscription Timeout**: 30 minutes
- **Retry Attempts**: 3 with exponential backoff

### Notes

- Requires a Sonos speaker on the same network
- The callback server needs an available port in the 8080-8090 range
- Events are received in real-time as they occur on the speaker
- Try playing/pausing music or changing volume to see events
- Press Ctrl+C to stop the example

## State Monitor

The `state-monitor.rs` example demonstrates how to:
- Discover Sonos speakers on your network
- Fetch real zone groups from the Sonos system
- Initialize and use the StateCache
- Display a live, updating view of your Sonos system state

### Running the State Monitor

```bash
cargo run --example state-monitor
```

### Features

- **Real-time Discovery**: Automatically discovers all Sonos speakers on your network
- **Live Groups**: Fetches actual zone group topology from your Sonos system
- **Dynamic Display**: Shows a nested tree view of groups and speakers with live updates
- **State Visualization**: Displays playback state, volume levels with visual bars, and speaker details
- **Overwriting Output**: Uses terminal control sequences to update the display in place

### Output Format

The monitor displays:
```
🎵 Sonos State Monitor (Update #1) 🎵
Press Ctrl+C to exit
═══════════════════════════════════════════════════════════════
📊 Groups and Speakers:
├─ 🏠 Group 1 (2 speakers)
│  ├─ 🔊 Living Room 👑
│  │  │  State: ▶️ Playing
│  │  │  Volume: 45% [████░░░░░░] 
│  │  │  Model: Sonos One
│  │  │  IP: 192.168.1.100
│  └─ 🔊 Kitchen
│     │  State: ▶️ Playing
│     │  Volume: 45% [████░░░░░░]
│     │  Model: Sonos Play:1
│     │  IP: 192.168.1.101
├─ 🔊 Bedroom (Solo)
│  State: ⏸️ Paused
│  Volume: 30% [███░░░░░░░]
│  Model: Sonos One SL
│  IP: 192.168.1.102

📈 Summary:
├─ Total Speakers: 3
├─ Total Groups: 2
└─ Currently Playing: 2
```

### Icons Used

- 🎵 - Application header
- 📊 - Groups section
- 🏠 - Multi-speaker group
- 🔊 - Individual speaker
- 👑 - Group coordinator
- ▶️ - Playing
- ⏸️ - Paused  
- ⏹️ - Stopped
- 🔄 - Transitioning
- 📈 - Summary section

### Notes

- The example will automatically discover speakers with a 5-second timeout
- If no speakers are found, it will exit gracefully
- Group information is fetched from the first discovered speaker
- The display updates every 2 seconds with simulated state changes for demonstration
- Press Ctrl+C to exit the monitor