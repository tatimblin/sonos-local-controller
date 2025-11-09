use sonos::model::{Speaker, SpeakerId, StateChange};
use sonos::streaming::{
    EventStreamBuilder, ServiceType, StreamError, LifecycleHandlers
};
use sonos::streaming::interface::ConfigOverrides;
use sonos::state::StateCache;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Helper function to create a test speaker
fn create_test_speaker(udn: &str, name: &str, ip: &str) -> Speaker {
    Speaker {
        id: SpeakerId::from_udn(udn),
        udn: udn.to_string(),
        name: name.to_string(),
        room_name: format!("{} Room", name),
        ip_address: ip.to_string(),
        port: 1400,
        model_name: "PLAY:1".to_string(),
        satellites: vec![],
    }
}

/// Helper function to create multiple test speakers
fn create_test_speakers(count: usize) -> Vec<Speaker> {
    (0..count)
        .map(|i| {
            create_test_speaker(
                &format!("uuid:RINCON_{}::1", 100000000 + i),
                &format!("Speaker{}", i + 1),
                &format!("192.168.1.{}", 100 + i),
            )
        })
        .collect()
}

#[test]
fn test_builder_basic_creation() {
    let speakers = create_test_speakers(1);
    
    // Test successful builder creation
    let builder = EventStreamBuilder::new(speakers).expect("Failed to create builder");
    
    // Verify builder has expected defaults
    assert_eq!(format!("{:?}", builder).contains("speakers"), true);
}

#[test]
fn test_builder_empty_speakers_validation() {
    // Test that empty speaker list fails validation
    let empty_speakers = vec![];
    let result = EventStreamBuilder::new(empty_speakers);
    
    assert!(result.is_err());
    match result.unwrap_err() {
        StreamError::ConfigurationError(msg) => {
            assert!(msg.contains("At least one speaker"));
        }
        _ => panic!("Expected ConfigurationError for empty speakers"),
    }
}

#[test]
fn test_builder_with_state_cache() {
    let speakers = create_test_speakers(1);
    let state_cache = Arc::new(StateCache::new());
    
    let builder = EventStreamBuilder::new(speakers)
        .expect("Failed to create builder")
        .with_state_cache(state_cache.clone());
    
    // Builder should accept state cache without error
    // We can't directly inspect the internal state, but we can verify
    // the builder pattern works by chaining methods
    let _builder = builder.with_services(&[ServiceType::AVTransport]);
}

#[test]
fn test_builder_with_services() {
    let speakers = create_test_speakers(1);
    
    // Test with single service
    let _builder = EventStreamBuilder::new(speakers.clone())
        .expect("Failed to create builder")
        .with_services(&[ServiceType::AVTransport]);
    
    // Test with multiple services
    let _builder = EventStreamBuilder::new(speakers.clone())
        .expect("Failed to create builder")
        .with_services(&[
            ServiceType::AVTransport,
            ServiceType::RenderingControl,
            ServiceType::ContentDirectory,
        ]);
    
    // Test with empty services (should still work, just use defaults)
    let builder = EventStreamBuilder::new(speakers)
        .expect("Failed to create builder")
        .with_services(&[]);
    
    // All should succeed as the builder pattern is fluent
    let _final_builder = builder.with_services(&[ServiceType::RenderingControl]);
}

#[test]
fn test_builder_with_event_handler() {
    let speakers = create_test_speakers(1);
    let event_received = Arc::new(Mutex::new(false));
    let event_received_clone = event_received.clone();
    
    let builder = EventStreamBuilder::new(speakers)
        .expect("Failed to create builder")
        .with_event_handler(move |_event| {
            *event_received_clone.lock().unwrap() = true;
        });
    
    // Test multiple event handlers
    let builder = builder.with_event_handler(|event| {
        println!("Second handler: {:?}", event);
    });
    
    // Builder should accept event handlers without error
    let _final_builder = builder.with_services(&[ServiceType::AVTransport]);
}

#[test]
fn test_builder_with_lifecycle_handlers() {
    let speakers = create_test_speakers(1);
    
    let connected_called = Arc::new(Mutex::new(false));
    let connected_called_clone = connected_called.clone();
    
    let disconnected_called = Arc::new(Mutex::new(false));
    let disconnected_called_clone = disconnected_called.clone();
    
    let error_called = Arc::new(Mutex::new(false));
    let error_called_clone = error_called.clone();
    
    let lifecycle_handlers = LifecycleHandlers::new()
        .with_speaker_connected(move |_id| {
            *connected_called_clone.lock().unwrap() = true;
        })
        .with_speaker_disconnected(move |_id| {
            *disconnected_called_clone.lock().unwrap() = true;
        })
        .with_error(move |_err| {
            *error_called_clone.lock().unwrap() = true;
        })
        .with_stream_started(|| {
            println!("Stream started");
        })
        .with_stream_stopped(|| {
            println!("Stream stopped");
        });
    
    let builder = EventStreamBuilder::new(speakers)
        .expect("Failed to create builder")
        .with_lifecycle_handlers(lifecycle_handlers);
    
    // Builder should accept lifecycle handlers without error
    let _final_builder = builder.with_services(&[ServiceType::AVTransport]);
}

#[test]
fn test_builder_with_timeouts() {
    let speakers = create_test_speakers(1);
    
    let builder = EventStreamBuilder::new(speakers)
        .expect("Failed to create builder")
        .with_timeouts(
            Duration::from_secs(3600), // 1 hour subscription timeout
            Duration::from_secs(2),    // 2 second retry backoff
        );
    
    // Builder should accept timeout configuration without error
    let _final_builder = builder.with_services(&[ServiceType::AVTransport]);
}

#[test]
fn test_builder_with_callback_ports() {
    let speakers = create_test_speakers(1);
    
    let builder = EventStreamBuilder::new(speakers)
        .expect("Failed to create builder")
        .with_callback_ports(9000, 9010);
    
    // Builder should accept port configuration without error
    let _final_builder = builder.with_services(&[ServiceType::AVTransport]);
}

#[test]
fn test_builder_method_chaining() {
    let speakers = create_test_speakers(2);
    let state_cache = Arc::new(StateCache::new());
    
    // Test that all builder methods can be chained together
    let builder = EventStreamBuilder::new(speakers)
        .expect("Failed to create builder")
        .with_state_cache(state_cache)
        .with_services(&[ServiceType::AVTransport, ServiceType::RenderingControl])
        .with_event_handler(|event| {
            match event {
                StateChange::PlaybackStateChanged { .. } => println!("Playback changed"),
                _ => {}
            }
        })
        .with_event_handler(|_| {
            // Second handler
        })
        .with_lifecycle_handlers(
            LifecycleHandlers::new()
                .with_stream_started(|| println!("Started"))
                .with_stream_stopped(|| println!("Stopped"))
        )
        .with_timeouts(Duration::from_secs(1800), Duration::from_secs(1))
        .with_callback_ports(8080, 8090);
    
    // All methods should chain successfully
    let _final_builder = builder.with_services(&[ServiceType::ContentDirectory]);
}

#[test]
fn test_config_overrides_validation() {
    let _speakers = create_test_speakers(1);
    
    // Test valid configuration
    let valid_config = ConfigOverrides::new()
        .with_subscription_timeout(Duration::from_secs(1800))
        .with_retry_backoff(Duration::from_secs(2))
        .with_callback_port_range(8080, 8090)
        .with_buffer_size(1000)
        .with_max_retry_attempts(3);
    
    assert!(valid_config.validate().is_ok());
    
    // Test invalid subscription timeout (too short)
    let invalid_config = ConfigOverrides::new()
        .with_subscription_timeout(Duration::from_secs(30));
    
    let result = invalid_config.validate();
    assert!(result.is_err());
    if let Err(StreamError::ConfigurationError(msg)) = result {
        assert!(msg.contains("at least 60 seconds"));
    } else {
        panic!("Expected ConfigurationError for short timeout");
    }
    
    // Test invalid subscription timeout (too long)
    let invalid_config = ConfigOverrides::new()
        .with_subscription_timeout(Duration::from_secs(90000));
    
    let result = invalid_config.validate();
    assert!(result.is_err());
    if let Err(StreamError::ConfigurationError(msg)) = result {
        assert!(msg.contains("too long"));
    } else {
        panic!("Expected ConfigurationError for long timeout");
    }
    
    // Test invalid buffer size (zero)
    let invalid_config = ConfigOverrides::new().with_buffer_size(0);
    
    let result = invalid_config.validate();
    assert!(result.is_err());
    if let Err(StreamError::ConfigurationError(msg)) = result {
        assert!(msg.contains("greater than 0"));
    } else {
        panic!("Expected ConfigurationError for zero buffer size");
    }
    
    // Test invalid buffer size (too large)
    let invalid_config = ConfigOverrides::new().with_buffer_size(200_000);
    
    let result = invalid_config.validate();
    assert!(result.is_err());
    if let Err(StreamError::ConfigurationError(msg)) = result {
        assert!(msg.contains("too large"));
    } else {
        panic!("Expected ConfigurationError for large buffer size");
    }
    
    // Test invalid port range (start >= end)
    let invalid_config = ConfigOverrides::new().with_callback_port_range(8080, 8080);
    
    let result = invalid_config.validate();
    assert!(result.is_err());
    if let Err(StreamError::ConfigurationError(msg)) = result {
        assert!(msg.contains("start must be less than end"));
    } else {
        panic!("Expected ConfigurationError for invalid port range");
    }
    
    // Test invalid port range (start too low)
    let invalid_config = ConfigOverrides::new().with_callback_port_range(1000, 1010);
    
    let result = invalid_config.validate();
    assert!(result.is_err());
    if let Err(StreamError::ConfigurationError(msg)) = result {
        assert!(msg.contains("must be >= 1024"));
    } else {
        panic!("Expected ConfigurationError for low port range");
    }
    
    // Test invalid retry attempts (too many)
    let invalid_config = ConfigOverrides::new().with_max_retry_attempts(15);
    
    let result = invalid_config.validate();
    assert!(result.is_err());
    if let Err(StreamError::ConfigurationError(msg)) = result {
        assert!(msg.contains("Too many retry attempts"));
    } else {
        panic!("Expected ConfigurationError for too many retry attempts");
    }
}

#[test]
fn test_builder_creates_correct_internal_config() {
    let speakers = create_test_speakers(1);
    
    // Create builder with specific configuration
    let builder = EventStreamBuilder::new(speakers)
        .expect("Failed to create builder")
        .with_services(&[ServiceType::AVTransport, ServiceType::RenderingControl])
        .with_timeouts(Duration::from_secs(3600), Duration::from_secs(2))
        .with_callback_ports(9000, 9010);
    
    // We can't directly inspect the internal StreamConfig, but we can test
    // that the builder accepts valid configurations and would fail on invalid ones
    
    // Test that the builder would fail with invalid configuration
    let invalid_builder = EventStreamBuilder::new(create_test_speakers(1))
        .expect("Failed to create builder")
        .with_timeouts(Duration::from_secs(30), Duration::from_secs(1)); // Invalid timeout
    
    // The validation happens during start(), so we can't test it here without
    // actually starting the stream (which requires real network setup)
    // This test documents the expected behavior
    let _valid_builder = builder;
    let _invalid_builder = invalid_builder;
}

#[test]
fn test_builder_error_handling_for_invalid_configurations() {
    let speakers = create_test_speakers(1);
    
    // Test that builder methods themselves don't fail (validation happens at start())
    let builder = EventStreamBuilder::new(speakers)
        .expect("Failed to create builder")
        .with_timeouts(Duration::from_secs(30), Duration::from_secs(1)) // This will be invalid
        .with_callback_ports(1000, 1010) // This will be invalid
        .with_services(&[ServiceType::AVTransport]);
    
    // The builder should accept these values, but validation should fail at start()
    // We can't test start() here because it requires actual network setup,
    // but this test documents that the builder pattern allows invalid values
    // to be set and validates them later
    let _builder_with_invalid_config = builder;
}

#[test]
fn test_lifecycle_handlers_builder_pattern() {
    // Test that LifecycleHandlers can be built using the builder pattern
    let handlers = LifecycleHandlers::new();
    assert!(handlers.on_speaker_connected.is_none());
    assert!(handlers.on_speaker_disconnected.is_none());
    assert!(handlers.on_error.is_none());
    assert!(handlers.on_stream_started.is_none());
    assert!(handlers.on_stream_stopped.is_none());
    
    let handlers = LifecycleHandlers::new()
        .with_speaker_connected(|id| println!("Connected: {:?}", id))
        .with_speaker_disconnected(|id| println!("Disconnected: {:?}", id))
        .with_error(|err| println!("Error: {:?}", err))
        .with_stream_started(|| println!("Started"))
        .with_stream_stopped(|| println!("Stopped"));
    
    assert!(handlers.on_speaker_connected.is_some());
    assert!(handlers.on_speaker_disconnected.is_some());
    assert!(handlers.on_error.is_some());
    assert!(handlers.on_stream_started.is_some());
    assert!(handlers.on_stream_stopped.is_some());
}

#[test]
fn test_config_overrides_builder_pattern() {
    // Test that ConfigOverrides can be built using the builder pattern
    let config = ConfigOverrides::new();
    assert!(config.subscription_timeout.is_none());
    assert!(config.retry_backoff.is_none());
    assert!(config.callback_port_range.is_none());
    assert!(config.buffer_size.is_none());
    assert!(config.max_retry_attempts.is_none());
    
    let config = ConfigOverrides::new()
        .with_subscription_timeout(Duration::from_secs(3600))
        .with_retry_backoff(Duration::from_secs(2))
        .with_callback_port_range(9000, 9010)
        .with_buffer_size(2000)
        .with_max_retry_attempts(5);
    
    assert_eq!(config.subscription_timeout, Some(Duration::from_secs(3600)));
    assert_eq!(config.retry_backoff, Some(Duration::from_secs(2)));
    assert_eq!(config.callback_port_range, Some((9000, 9010)));
    assert_eq!(config.buffer_size, Some(2000));
    assert_eq!(config.max_retry_attempts, Some(5));
}

#[test]
fn test_builder_with_multiple_speakers() {
    let speakers = create_test_speakers(5);
    
    let builder = EventStreamBuilder::new(speakers)
        .expect("Failed to create builder with multiple speakers")
        .with_services(&[ServiceType::AVTransport, ServiceType::RenderingControl])
        .with_event_handler(|event| {
            println!("Event from any speaker: {:?}", event);
        });
    
    // Builder should handle multiple speakers without issue
    let _final_builder = builder.with_timeouts(Duration::from_secs(1800), Duration::from_secs(1));
}

#[test]
fn test_builder_debug_formatting() {
    let speakers = create_test_speakers(3);
    let builder = EventStreamBuilder::new(speakers)
        .expect("Failed to create builder")
        .with_services(&[ServiceType::AVTransport])
        .with_event_handler(|_| {});
    
    let debug_str = format!("{:?}", builder);
    
    // Debug output should contain useful information
    assert!(debug_str.contains("EventStreamBuilder"));
    assert!(debug_str.contains("speakers"));
    assert!(debug_str.contains("services"));
    assert!(debug_str.contains("event_handlers_count"));
}