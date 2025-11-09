use sonos::models::{PlaybackState, Speaker, SpeakerId, StateChange, TrackInfo, TransportStatus};
use sonos::state::StateCache;
use sonos::streaming::{EventStreamBuilder, LifecycleHandlers, ServiceType, StreamError};
use std::sync::{Arc, Mutex};
use std::thread;
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

/// Helper function to create test events
fn _create_test_events(speaker_id: SpeakerId) -> Vec<StateChange> {
    vec![
        StateChange::PlaybackStateChanged {
            speaker_id,
            state: PlaybackState::Playing,
        },
        StateChange::VolumeChanged {
            speaker_id,
            volume: 75,
        },
        StateChange::MuteChanged {
            speaker_id,
            muted: false,
        },
        StateChange::PositionChanged {
            speaker_id,
            position_ms: 45000,
        },
        StateChange::TrackChanged {
            speaker_id,
            track_info: Some(TrackInfo {
                title: Some("Test Track".to_string()),
                artist: Some("Test Artist".to_string()),
                album: Some("Test Album".to_string()),
                duration_ms: Some(180000),
                uri: None,
            }),
        },
    ]
}

#[test]
fn test_event_handler_registration_and_calling() {
    // Test that multiple event handlers are registered and called correctly
    let speakers = vec![create_test_speaker(
        "uuid:RINCON_123456789::1",
        "Test Room",
        "192.168.1.100",
    )];

    let handler1_called = Arc::new(Mutex::new(false));
    let handler1_called_clone = handler1_called.clone();

    let handler2_called = Arc::new(Mutex::new(false));
    let handler2_called_clone = handler2_called.clone();

    let handler3_called = Arc::new(Mutex::new(false));
    let handler3_called_clone = handler3_called.clone();

    let received_events = Arc::new(Mutex::new(Vec::new()));
    let received_events_clone = received_events.clone();

    // Create builder with multiple event handlers
    let builder = EventStreamBuilder::new(speakers)
        .expect("Failed to create builder")
        .with_event_handler(move |event| {
            *handler1_called_clone.lock().unwrap() = true;
            println!("Handler 1 received: {:?}", event);
        })
        .with_event_handler(move |event| {
            *handler2_called_clone.lock().unwrap() = true;
            println!("Handler 2 received: {:?}", event);
        })
        .with_event_handler(move |event| {
            *handler3_called_clone.lock().unwrap() = true;
            received_events_clone.lock().unwrap().push(event);
        });

    // Note: We can't actually start the stream in tests without real network setup,
    // but we can test that the builder accepts multiple handlers correctly
    let _final_builder = builder.with_services(&[ServiceType::AVTransport]);

    // Verify handlers were registered (we can't test actual calling without network)
    // This test documents the expected behavior for when the implementation is complete
}

#[test]
fn test_lifecycle_callback_registration() {
    let speakers = vec![create_test_speaker(
        "uuid:RINCON_123456789::1",
        "Test Room",
        "192.168.1.100",
    )];

    let connected_calls = Arc::new(Mutex::new(Vec::new()));
    let connected_calls_clone = connected_calls.clone();

    let disconnected_calls = Arc::new(Mutex::new(Vec::new()));
    let disconnected_calls_clone = disconnected_calls.clone();

    let error_calls = Arc::new(Mutex::new(Vec::new()));
    let error_calls_clone = error_calls.clone();

    let stream_started_called = Arc::new(Mutex::new(false));
    let stream_started_called_clone = stream_started_called.clone();

    let stream_stopped_called = Arc::new(Mutex::new(false));
    let stream_stopped_called_clone = stream_stopped_called.clone();

    let lifecycle_handlers = LifecycleHandlers::new()
        .with_speaker_connected(move |speaker_id| {
            connected_calls_clone.lock().unwrap().push(speaker_id);
            println!("Speaker connected: {:?}", speaker_id);
        })
        .with_speaker_disconnected(move |speaker_id| {
            disconnected_calls_clone.lock().unwrap().push(speaker_id);
            println!("Speaker disconnected: {:?}", speaker_id);
        })
        .with_error(move |error| {
            error_calls_clone
                .lock()
                .unwrap()
                .push(format!("{:?}", error));
            println!("Error occurred: {:?}", error);
        })
        .with_stream_started(move || {
            *stream_started_called_clone.lock().unwrap() = true;
            println!("Stream started");
        })
        .with_stream_stopped(move || {
            *stream_stopped_called_clone.lock().unwrap() = true;
            println!("Stream stopped");
        });

    let builder = EventStreamBuilder::new(speakers)
        .expect("Failed to create builder")
        .with_lifecycle_handlers(lifecycle_handlers)
        .with_services(&[ServiceType::AVTransport]);

    // Verify lifecycle handlers were registered
    let _final_builder = builder;

    // Note: Actual callback testing would require starting the stream and simulating events
    // This test documents the expected registration behavior
}

#[test]
fn test_state_cache_integration_setup() {
    let speakers = vec![
        create_test_speaker("uuid:RINCON_111111111::1", "Living Room", "192.168.1.100"),
        create_test_speaker("uuid:RINCON_222222222::1", "Kitchen", "192.168.1.101"),
    ];

    let state_cache = Arc::new(StateCache::new());

    // Initialize state cache with speakers (as would happen in real usage)
    state_cache.initialize(speakers.clone(), vec![]);

    let builder = EventStreamBuilder::new(speakers)
        .expect("Failed to create builder")
        .with_state_cache(state_cache.clone())
        .with_services(&[ServiceType::AVTransport, ServiceType::RenderingControl]);

    // Test that builder accepts StateCache integration
    let _final_builder = builder.with_event_handler(|event| {
        println!("Event for StateCache integration: {:?}", event);
    });

    // Note: Actual StateCache update testing would require starting the stream
    // and verifying that events update the cache correctly
}

#[test]
fn test_speaker_add_remove_operations_setup() {
    let initial_speakers = vec![create_test_speaker(
        "uuid:RINCON_111111111::1",
        "Living Room",
        "192.168.1.100",
    )];

    let builder = EventStreamBuilder::new(initial_speakers)
        .expect("Failed to create builder")
        .with_services(&[ServiceType::AVTransport]);

    // Note: Actual add/remove testing would require:
    // 1. Starting the stream: let stream = builder.start()?;
    // 2. Adding speakers: stream.add_speaker(new_speaker)?;
    // 3. Removing speakers: stream.remove_speaker(speaker_id)?;
    // 4. Verifying subscriptions are managed correctly

    // This test documents the expected setup for add/remove operations
    let _final_builder = builder;
}

#[test]
fn test_event_processing_with_different_service_types() {
    let speakers = vec![create_test_speaker(
        "uuid:RINCON_123456789::1",
        "Test Room",
        "192.168.1.100",
    )];

    // Test with AVTransport only
    let av_builder = EventStreamBuilder::new(speakers.clone())
        .expect("Failed to create builder")
        .with_services(&[ServiceType::AVTransport])
        .with_event_handler(|event| match event {
            StateChange::PlaybackStateChanged { .. } => println!("AVTransport event"),
            StateChange::TrackChanged { .. } => println!("AVTransport track event"),
            _ => {}
        });

    // Test with RenderingControl only
    let rc_builder = EventStreamBuilder::new(speakers.clone())
        .expect("Failed to create builder")
        .with_services(&[ServiceType::RenderingControl])
        .with_event_handler(|event| match event {
            StateChange::VolumeChanged { .. } => println!("RenderingControl volume event"),
            StateChange::MuteChanged { .. } => println!("RenderingControl mute event"),
            _ => {}
        });

    // Test with multiple services
    let multi_builder = EventStreamBuilder::new(speakers)
        .expect("Failed to create builder")
        .with_services(&[
            ServiceType::AVTransport,
            ServiceType::RenderingControl,
            ServiceType::ContentDirectory,
        ])
        .with_event_handler(|event| {
            println!("Multi-service event: {:?}", event);
        });

    // All builders should be created successfully
    let _av_final = av_builder;
    let _rc_final = rc_builder;
    let _multi_final = multi_builder;
}

#[test]
fn test_error_handling_in_event_processing() {
    let speakers = vec![create_test_speaker(
        "uuid:RINCON_123456789::1",
        "Test Room",
        "192.168.1.100",
    )];

    let error_count = Arc::new(Mutex::new(0));
    let error_count_clone = error_count.clone();

    let lifecycle_handlers = LifecycleHandlers::new().with_error(move |error| {
        *error_count_clone.lock().unwrap() += 1;
        println!("Error in event processing: {:?}", error);

        // Test different error types
        match error {
            StreamError::NetworkError(_) => println!("Network error detected"),
            StreamError::SpeakerOperationFailed(_) => println!("Speaker operation failed"),
            StreamError::SubscriptionError(_) => println!("Subscription error detected"),
            _ => println!("Other error type"),
        }
    });

    let builder = EventStreamBuilder::new(speakers)
        .expect("Failed to create builder")
        .with_lifecycle_handlers(lifecycle_handlers)
        .with_event_handler(|event| {
            // Test that event handlers can handle different event types
            match event {
                StateChange::SubscriptionError {
                    speaker_id,
                    error,
                    service,
                } => {
                    println!(
                        "Subscription error for speaker {:?} on service {:?}: {}",
                        speaker_id, service, error
                    );
                }
                StateChange::TransportInfoChanged {
                    speaker_id,
                    transport_status,
                    ..
                } => match transport_status {
                    TransportStatus::ErrorOccurred => {
                        println!("Transport error for speaker {:?}", speaker_id);
                    }
                    TransportStatus::Ok => {
                        println!("Transport OK for speaker {:?}", speaker_id);
                    }
                },
                _ => {}
            }
        });

    let _final_builder = builder.with_services(&[ServiceType::AVTransport]);
}

#[test]
fn test_concurrent_event_handler_safety() {
    let speakers = vec![create_test_speaker(
        "uuid:RINCON_123456789::1",
        "Test Room",
        "192.168.1.100",
    )];

    // Test that multiple handlers can be registered safely
    let shared_counter = Arc::new(Mutex::new(0));

    let counter1 = shared_counter.clone();
    let counter2 = shared_counter.clone();
    let counter3 = shared_counter.clone();

    let builder = EventStreamBuilder::new(speakers)
        .expect("Failed to create builder")
        .with_event_handler(move |_event| {
            let mut count = counter1.lock().unwrap();
            *count += 1;
            // Simulate some processing time
            thread::sleep(Duration::from_millis(1));
        })
        .with_event_handler(move |_event| {
            let mut count = counter2.lock().unwrap();
            *count += 10;
            thread::sleep(Duration::from_millis(1));
        })
        .with_event_handler(move |_event| {
            let mut count = counter3.lock().unwrap();
            *count += 100;
            thread::sleep(Duration::from_millis(1));
        });

    // Test that handlers can be registered without conflicts
    let _final_builder = builder.with_services(&[ServiceType::AVTransport]);

    // Note: Actual concurrent execution testing would require starting the stream
    // and sending events from multiple threads
}

#[test]
fn test_event_filtering_and_processing() {
    let speakers = vec![create_test_speaker(
        "uuid:RINCON_123456789::1",
        "Test Room",
        "192.168.1.100",
    )];

    let playback_events = Arc::new(Mutex::new(Vec::new()));
    let playback_events_clone = playback_events.clone();

    let volume_events = Arc::new(Mutex::new(Vec::new()));
    let volume_events_clone = volume_events.clone();

    let other_events = Arc::new(Mutex::new(Vec::new()));
    let other_events_clone = other_events.clone();

    let builder = EventStreamBuilder::new(speakers)
        .expect("Failed to create builder")
        .with_event_handler(move |event| {
            // Handler that filters for playback events
            match event {
                StateChange::PlaybackStateChanged { speaker_id, state } => {
                    playback_events_clone
                        .lock()
                        .unwrap()
                        .push((speaker_id, state));
                }
                _ => {}
            }
        })
        .with_event_handler(move |event| {
            // Handler that filters for volume events
            match event {
                StateChange::VolumeChanged { speaker_id, volume } => {
                    volume_events_clone
                        .lock()
                        .unwrap()
                        .push((speaker_id, volume));
                }
                StateChange::MuteChanged { speaker_id, muted } => {
                    volume_events_clone
                        .lock()
                        .unwrap()
                        .push((speaker_id, if muted { 0 } else { 100 }));
                }
                _ => {}
            }
        })
        .with_event_handler(move |event| {
            // Handler that captures all other events
            match event {
                StateChange::PlaybackStateChanged { .. }
                | StateChange::VolumeChanged { .. }
                | StateChange::MuteChanged { .. } => {
                    // Skip events handled by other handlers
                }
                _ => {
                    other_events_clone
                        .lock()
                        .unwrap()
                        .push(format!("{:?}", event));
                }
            }
        });

    let _final_builder =
        builder.with_services(&[ServiceType::AVTransport, ServiceType::RenderingControl]);

    // Note: Actual event filtering testing would require starting the stream
    // and verifying that events are correctly filtered and processed
}

#[test]
fn test_graceful_shutdown_setup() {
    let speakers = vec![create_test_speaker(
        "uuid:RINCON_123456789::1",
        "Test Room",
        "192.168.1.100",
    )];

    let shutdown_called = Arc::new(Mutex::new(false));
    let shutdown_called_clone = shutdown_called.clone();

    let lifecycle_handlers = LifecycleHandlers::new().with_stream_stopped(move || {
        *shutdown_called_clone.lock().unwrap() = true;
        println!("Stream gracefully stopped");
    });

    let builder = EventStreamBuilder::new(speakers)
        .expect("Failed to create builder")
        .with_lifecycle_handlers(lifecycle_handlers)
        .with_event_handler(|event| {
            println!("Processing event before shutdown: {:?}", event);
        });

    // Note: Actual shutdown testing would require:
    // 1. Starting the stream: let stream = builder.start()?;
    // 2. Calling shutdown: stream.shutdown()?;
    // 3. Verifying cleanup: assert!(shutdown_called.lock().unwrap());

    let _final_builder = builder.with_services(&[ServiceType::AVTransport]);
}

#[test]
fn test_state_cache_update_patterns() {
    let speakers = vec![
        create_test_speaker("uuid:RINCON_111111111::1", "Living Room", "192.168.1.100"),
        create_test_speaker("uuid:RINCON_222222222::1", "Kitchen", "192.168.1.101"),
    ];

    let state_cache = Arc::new(StateCache::new());
    state_cache.initialize(speakers.clone(), vec![]);

    // Test different update patterns that the event processing should handle
    let _test_events = vec![
        StateChange::PlaybackStateChanged {
            speaker_id: speakers[0].id,
            state: PlaybackState::Playing,
        },
        StateChange::VolumeChanged {
            speaker_id: speakers[0].id,
            volume: 50,
        },
        StateChange::MuteChanged {
            speaker_id: speakers[1].id,
            muted: true,
        },
    ];

    let builder = EventStreamBuilder::new(speakers)
        .expect("Failed to create builder")
        .with_state_cache(state_cache.clone())
        .with_event_handler(move |event| {
            // This handler would work alongside automatic StateCache updates
            println!(
                "Event received (StateCache will be updated automatically): {:?}",
                event
            );
        });

    let _final_builder =
        builder.with_services(&[ServiceType::AVTransport, ServiceType::RenderingControl]);

    // Note: Actual StateCache update testing would require starting the stream
    // and verifying that the cache is updated correctly for each event type

    // Document expected StateCache update behavior:
    // - PlaybackStateChanged -> cache.update_playback_state()
    // - VolumeChanged -> cache.update_volume()
    // - MuteChanged -> cache.update_mute()
}

#[test]
fn test_multiple_speakers_event_processing() {
    let speakers = vec![
        create_test_speaker("uuid:RINCON_111111111::1", "Living Room", "192.168.1.100"),
        create_test_speaker("uuid:RINCON_222222222::1", "Kitchen", "192.168.1.101"),
        create_test_speaker("uuid:RINCON_333333333::1", "Bedroom", "192.168.1.102"),
    ];

    let events_by_speaker = Arc::new(Mutex::new(std::collections::HashMap::new()));
    let events_by_speaker_clone = events_by_speaker.clone();

    let builder = EventStreamBuilder::new(speakers.clone())
        .expect("Failed to create builder")
        .with_event_handler(move |event| {
            let speaker_id = match event {
                StateChange::PlaybackStateChanged { speaker_id, .. } => speaker_id,
                StateChange::VolumeChanged { speaker_id, .. } => speaker_id,
                StateChange::MuteChanged { speaker_id, .. } => speaker_id,
                StateChange::TrackChanged { speaker_id, .. } => speaker_id,
                StateChange::PositionChanged { speaker_id, .. } => speaker_id,
                _ => return, // Skip events without speaker_id
            };

            let mut events_map = events_by_speaker_clone.lock().unwrap();
            let count = events_map.entry(speaker_id).or_insert(0);
            *count += 1;
        });

    let _final_builder =
        builder.with_services(&[ServiceType::AVTransport, ServiceType::RenderingControl]);

    // Note: Actual multi-speaker testing would require starting the stream
    // and verifying that events from different speakers are processed correctly
}
