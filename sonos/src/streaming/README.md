# Sonos Streaming Module

The streaming module provides real-time event monitoring for Sonos speakers through UPnP subscriptions. It enables applications to receive live updates about playback state, volume changes, group formations, and other speaker events.

## Architecture Overview

The streaming system consists of several interconnected components that work together to provide a robust, scalable event streaming solution:

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Application   │    │  EventStreamBuilder │    │ ActiveEventStream│
│                 │───▶│                  │───▶│                 │
│ - Event handlers│    │ - Configuration  │    │ - Runtime mgmt  │
│ - StateCache    │    │ - Validation     │    │ - Shutdown      │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                                         │
                       ┌─────────────────────────────────┘
                       ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Event Processing Loop                        │
│ - StateCache updates                                            │
│ - User event handlers                                           │
│ - Lifecycle callbacks                                           │
└─────────────────────────────────────────────────────────────────┘
                       │
                       ▼
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│SubscriptionMgr  │    │  CallbackServer  │    │  EventStream    │
│                 │◀──▶│                  │◀──▶│                 │
│ - Per-speaker   │    │ - HTTP server    │    │ - StateCache    │
│ - Network-wide  │    │ - Event routing  │    │ - Event proc.   │
│ - Lifecycle     │    │ - Port binding   │    │ - Integration   │
└─────────────────┘    └──────────────────┘    └─────────────────┘
         │                       │
         ▼                       ▼
┌─────────────────┐    ┌──────────────────┐
│  Subscriptions  │    │   Sonos Devices  │
│                 │◀──▶│                  │
│ - AVTransport   │    │ - Event notify   │
│ - RenderingCtrl │    │ - HTTP callbacks │
│ - ZoneTopology  │    │ - UPnP protocol  │
└─────────────────┘    └──────────────────┘
```

## Core Components

### 1. EventStreamBuilder (`builder.rs`)

The main entry point for creating event streams. Provides a fluent interface for configuration:

- **Purpose**: Simplifies event stream creation with sensible defaults
- **Key Features**:
  - Service selection (AVTransport, RenderingControl, ZoneGroupTopology)
  - StateCache integration
  - Event handler registration
  - Lifecycle callback configuration
  - Timeout and port range customization

**Usage Example**:
```rust
let stream = EventStreamBuilder::new(speakers)?
    .with_state_cache(state_cache)
    .with_services(&[ServiceType::AVTransport, ServiceType::RenderingControl])
    .with_event_handler(|event| println!("Event: {:?}", event))
    .with_lifecycle_handlers(
        LifecycleHandlers::new()
            .with_speaker_connected(|id| println!("Connected: {:?}", id))
            .with_error(|err| eprintln!("Error: {:?}", err))
    )
    .start()?;
```

### 2. SubscriptionManager (`manager.rs`)

Manages UPnP subscriptions across multiple speakers and services:

- **Purpose**: Coordinates subscription lifecycle and event routing
- **Key Features**:
  - Per-speaker subscriptions (AVTransport, RenderingControl)
  - Network-wide subscriptions (ZoneGroupTopology)
  - Automatic renewal and error recovery
  - Service isolation (failures don't affect other services)
  - Background thread for subscription management

**Subscription Scopes**:
- **PerSpeaker**: Each speaker gets its own subscription (AVTransport, RenderingControl)
- **NetworkWide**: Single subscription covers all speakers (ZoneGroupTopology)

### 3. CallbackServer (`callback_server.rs`)

HTTP server that receives UPnP event notifications from Sonos devices:

- **Purpose**: Receives and routes incoming UPnP events
- **Key Features**:
  - Automatic port binding within specified range
  - Local IP address detection for device accessibility
  - Event routing to appropriate subscriptions
  - Request logging and debugging support
  - Graceful shutdown handling

**Network Requirements**:
- Binds to `0.0.0.0` (all interfaces) so Sonos devices can reach it
- Attempts to detect local IP address for callback URLs
- Falls back to localhost if IP detection fails (may not work with devices)

### 4. Service Subscriptions

Individual subscription implementations for each UPnP service:

#### AVTransport (`av_transport.rs`)
- **Events**: Playback state, track changes, transport info
- **Scope**: PerSpeaker (each speaker needs individual subscription)
- **Key State**: Playing/Paused/Stopped, current track, position

#### RenderingControl (`rendering_control.rs`)
- **Events**: Volume changes, mute state, audio settings
- **Scope**: PerSpeaker (each speaker has independent volume)
- **Key State**: Volume level, mute status, audio properties

#### ZoneGroupTopology (`zone_group_topology.rs`)
- **Events**: Group formation/dissolution, coordinator changes
- **Scope**: NetworkWide (single subscription covers entire network)
- **Key State**: Group membership, coordinator assignments, topology changes

### 5. Type System (`types.rs`)

Core types and configuration structures:

- **ServiceType**: Enum defining available UPnP services
- **SubscriptionScope**: PerSpeaker vs NetworkWide classification
- **StreamConfig**: Overall streaming system configuration
- **SubscriptionConfig**: Individual subscription settings
- **RawEvent**: Unprocessed event data from devices

### 6. Public Interface (`interface.rs`)

Simplified error types and configuration for external use:

- **StreamError**: User-friendly error messages with actionable guidance
- **LifecycleHandlers**: Callbacks for connection events and errors
- **StreamStats**: Runtime statistics and health monitoring
- **ConfigOverrides**: Advanced configuration options

## Event Flow

1. **Subscription Establishment**:
   ```
   Application → EventStreamBuilder → SubscriptionManager → Individual Subscriptions
   ```

2. **Event Reception**:
   ```
   Sonos Device → CallbackServer → EventRouter → SubscriptionManager → Event Processing
   ```

3. **Event Processing**:
   ```
   Raw Event → Service Subscription → StateChange → StateCache + User Handlers
   ```

## Service Isolation

The streaming system implements service isolation to ensure that failures in one service don't affect others:

- **PerSpeaker Services**: AVTransport and RenderingControl failures are isolated per speaker
- **NetworkWide Services**: ZoneGroupTopology failures don't affect individual speaker services
- **Error Handling**: Each service type has dedicated error handling and recovery logic
- **Registry Management**: Separate tracking for different subscription scopes

## Configuration

### Default Configuration
```rust
StreamConfig {
    buffer_size: 1000,
    subscription_timeout: Duration::from_secs(1800), // 30 minutes
    retry_attempts: 3,
    retry_backoff: Duration::from_secs(1),
    enabled_services: vec![
        ServiceType::AVTransport,
        ServiceType::RenderingControl,
        ServiceType::ZoneGroupTopology
    ],
    callback_port_range: (8080, 8090),
}
```

### Preset Configurations
- **Minimal**: Basic functionality, low resource usage
- **Production**: Balanced performance and reliability
- **Comprehensive**: All services enabled, extended timeouts

## Error Handling

The streaming system provides comprehensive error handling with service isolation:

### Error Types
- **NetworkError**: Connection issues, timeouts
- **ConfigurationError**: Invalid settings, validation failures
- **SpeakerOperationFailed**: Device-specific errors
- **SubscriptionError**: UPnP subscription failures
- **InitializationFailed**: Startup errors

### Error Recovery
- **Automatic Retry**: Exponential backoff for transient failures
- **Service Isolation**: Failures don't cascade between services
- **Graceful Degradation**: Continue with available services
- **User Notification**: Actionable error messages through lifecycle handlers

## Thread Safety

All components are designed for concurrent access:

- **Arc<RwLock<>>**: Shared state with reader-writer locks
- **mpsc Channels**: Thread-safe event communication
- **Background Threads**: Non-blocking subscription management
- **Async/Await**: Efficient I/O handling in callback server

## Testing

Each component includes comprehensive unit tests:

- **Mock Implementations**: Service subscription testing
- **Integration Tests**: End-to-end event flow
- **Error Scenarios**: Failure handling and recovery
- **Configuration Validation**: Settings validation and edge cases

## Usage Patterns

### Basic Event Monitoring
```rust
let stream = EventStreamBuilder::new(speakers)?
    .with_event_handler(|event| {
        match event {
            StateChange::PlaybackStateChanged { speaker_id, state } => {
                println!("Speaker {:?} is now {:?}", speaker_id, state);
            }
            StateChange::VolumeChanged { speaker_id, volume } => {
                println!("Speaker {:?} volume: {}%", speaker_id, volume);
            }
            _ => {}
        }
    })
    .start()?;
```

### StateCache Integration
```rust
let state_cache = Arc::new(StateCache::new());
let stream = EventStreamBuilder::new(speakers)?
    .with_state_cache(Arc::clone(&state_cache))
    .start()?;

// StateCache is automatically updated with events
let speaker_state = state_cache.get_speaker(speaker_id)?;
println!("Current volume: {}", speaker_state.volume);
```

### Error Monitoring
```rust
let stream = EventStreamBuilder::new(speakers)?
    .with_lifecycle_handlers(
        LifecycleHandlers::new()
            .with_error(|error| {
                match error {
                    StreamError::NetworkError(msg) => {
                        eprintln!("Network issue: {}", msg);
                    }
                    StreamError::SpeakerOperationFailed(msg) => {
                        eprintln!("Speaker error: {}", msg);
                    }
                    _ => eprintln!("Other error: {:?}", error),
                }
            })
    )
    .start()?;
```

## Performance Considerations

- **Event Buffering**: Configurable buffer sizes for high-throughput scenarios
- **Non-blocking I/O**: Async processing prevents event queue backup
- **Service Isolation**: Failures don't impact overall system performance
- **Efficient Parsing**: Optimized XML parsing for UPnP events
- **Connection Pooling**: Reused HTTP connections for subscription management

## Debugging

The streaming system provides extensive logging and debugging support:

- **Request Logging**: All HTTP requests to callback server are logged
- **Event Tracing**: Raw events and parsed state changes are traced
- **Subscription Status**: Active subscriptions and their health
- **Error Context**: Detailed error information with service identification
- **Statistics**: Runtime metrics through StreamStats

Enable debug logging to see detailed operation:
```rust
env_logger::init(); // or your preferred logging setup
```

## Future Enhancements

- **Dynamic Service Discovery**: Automatic detection of available services
- **Load Balancing**: Multiple callback servers for high availability
- **Event Filtering**: Client-side filtering to reduce processing overhead
- **Metrics Collection**: Detailed performance and reliability metrics
- **Configuration Hot-reload**: Runtime configuration updates without restart