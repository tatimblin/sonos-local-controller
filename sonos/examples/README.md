# Sonos Examples

This directory contains example programs demonstrating various features of the Sonos library.

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