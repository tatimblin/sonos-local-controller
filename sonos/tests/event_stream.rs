use sonos::models::{PlaybackState, Speaker, SpeakerId, StateChange};
use sonos::streaming::{EventStream, ServiceType, StreamConfig};

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
    }
}

/// Helper function to create test events
fn create_test_events(speaker_id: SpeakerId) -> Vec<StateChange> {
    vec![
        StateChange::PlaybackStateChanged {
            speaker_id,
            state: PlaybackState::Playing,
        },
        StateChange::VolumeChanged {
            speaker_id,
            volume: 50,
        },
        StateChange::MuteChanged {
            speaker_id,
            muted: false,
        },
        StateChange::PositionChanged {
            speaker_id,
            position_ms: 30000,
        },
    ]
}

#[test]
fn test_event_stream_basic_creation_and_speaker_management() {
    // Create test speakers
    let speaker1 = create_test_speaker("uuid:RINCON_123456789::1", "Living Room", "192.168.1.100");
    let speaker2 = create_test_speaker("uuid:RINCON_987654321::1", "Kitchen", "192.168.1.101");

    let speakers = vec![speaker1.clone(), speaker2.clone()];

    // Create stream configuration
    let config = StreamConfig::default()
        .with_buffer_size(100)
        .unwrap()
        .with_enabled_services(vec![ServiceType::AVTransport]);

    // Create EventStream
    let event_stream = EventStream::new(speakers, config).expect("Failed to create EventStream");

    // Verify the stream is active
    assert!(event_stream.is_active());

    // Test adding a new speaker
    let speaker3 = create_test_speaker("uuid:RINCON_555666777::1", "Bedroom", "192.168.1.102");
    event_stream
        .add_speaker(speaker3.clone())
        .expect("Failed to add speaker");

    // Test removing a speaker
    event_stream
        .remove_speaker(speaker2.id)
        .expect("Failed to remove speaker");
}

#[test]
fn test_event_stream_non_blocking_operations() {
    // Create a simple test setup
    let speaker = create_test_speaker("uuid:RINCON_123456789::1", "Test Room", "192.168.1.100");
    let config = StreamConfig::default();

    let event_stream =
        EventStream::new(vec![speaker], config).expect("Failed to create EventStream");

    // Test non-blocking receive on empty stream
    let result = event_stream.try_recv();
    assert!(result.is_none(), "Expected no events in empty stream");

    // Test timeout receive
    let timeout_result = event_stream.recv_timeout(Duration::from_millis(10));
    assert!(timeout_result.is_none(), "Expected timeout on empty stream");
}

#[test]
fn test_event_stream_configuration_validation() {
    let speaker = create_test_speaker("uuid:RINCON_123456789::1", "Test Room", "192.168.1.100");

    // Test with valid configuration
    let valid_config = StreamConfig::default()
        .with_buffer_size(500)
        .unwrap()
        .with_enabled_services(vec![
            ServiceType::AVTransport,
            ServiceType::RenderingControl,
        ]);

    let result = EventStream::new(vec![speaker.clone()], valid_config);
    assert!(result.is_ok(), "Valid configuration should succeed");

    // Test with invalid buffer size
    let invalid_config = StreamConfig::default().with_buffer_size(0);
    assert!(invalid_config.is_err(), "Invalid buffer size should fail");
}

#[test]
fn test_event_stream_multiple_speakers() {
    // Create multiple test speakers
    let speakers = vec![
        create_test_speaker("uuid:RINCON_111111111::1", "Living Room", "192.168.1.100"),
        create_test_speaker("uuid:RINCON_222222222::1", "Kitchen", "192.168.1.101"),
        create_test_speaker("uuid:RINCON_333333333::1", "Bedroom", "192.168.1.102"),
    ];

    let config = StreamConfig::default().with_enabled_services(vec![
        ServiceType::AVTransport,
        ServiceType::RenderingControl,
    ]);

    let event_stream = EventStream::new(speakers.clone(), config)
        .expect("Failed to create EventStream with multiple speakers");

    assert!(event_stream.is_active());

    // Test that we can add and remove speakers dynamically
    let new_speaker = create_test_speaker("uuid:RINCON_444444444::1", "Office", "192.168.1.103");
    event_stream
        .add_speaker(new_speaker.clone())
        .expect("Failed to add new speaker");

    event_stream
        .remove_speaker(speakers[0].id)
        .expect("Failed to remove speaker");
}

/// Integration test that simulates the full event flow
/// Note: This test demonstrates the intended behavior once the full implementation is complete
#[test]
fn test_event_stream_mock_event_flow() {
    // This test demonstrates how the EventStream should work with real events
    // Currently it tests the interface, but once the full implementation is done,
    // it will test actual event flow

    let speaker = create_test_speaker("uuid:RINCON_123456789::1", "Test Room", "192.168.1.100");
    let speaker_id = speaker.id;

    let config = StreamConfig::default().with_buffer_size(10).unwrap();

    let event_stream =
        EventStream::new(vec![speaker], config).expect("Failed to create EventStream");

    // Create some test events that we would expect to receive
    let expected_events = create_test_events(speaker_id);

    // In the current implementation, we can't actually send events through the stream
    // because the SubscriptionManager is a placeholder. This test documents the
    // expected behavior for when the implementation is complete.

    // Test the iterator interface (even though it won't receive events yet)
    let _iter = event_stream.iter();

    // Test that the stream is still active
    assert!(event_stream.is_active());

    // Verify we can create the expected event types
    for event in expected_events {
        match event {
            StateChange::PlaybackStateChanged {
                speaker_id: sid,
                state,
            } => {
                assert_eq!(sid, speaker_id);
                assert_eq!(state, PlaybackState::Playing);
            }
            StateChange::VolumeChanged {
                speaker_id: sid,
                volume,
            } => {
                assert_eq!(sid, speaker_id);
                assert_eq!(volume, 50);
            }
            StateChange::MuteChanged {
                speaker_id: sid,
                muted,
            } => {
                assert_eq!(sid, speaker_id);
                assert_eq!(muted, false);
            }
            StateChange::PositionChanged {
                speaker_id: sid,
                position_ms,
            } => {
                assert_eq!(sid, speaker_id);
                assert_eq!(position_ms, 30000);
            }
            _ => panic!("Unexpected event type"),
        }
    }
}

/// Test demonstrating EventStream operations
/// Note: Full concurrent access will be possible once we implement proper thread-safe event handling
#[test]
fn test_event_stream_operations() {
    let speaker = create_test_speaker("uuid:RINCON_123456789::1", "Test Room", "192.168.1.100");
    let config = StreamConfig::default();

    let event_stream =
        EventStream::new(vec![speaker], config).expect("Failed to create EventStream");

    // Test adding and removing speakers in sequence
    let new_speaker = create_test_speaker("uuid:RINCON_999999999::1", "New Room", "192.168.1.199");
    event_stream
        .add_speaker(new_speaker.clone())
        .expect("Failed to add speaker");

    // Simulate some time passing
    thread::sleep(Duration::from_millis(5));

    event_stream
        .remove_speaker(new_speaker.id)
        .expect("Failed to remove speaker");

    assert!(event_stream.is_active());

    // Test multiple non-blocking operations
    for _ in 0..10 {
        let _result = event_stream.try_recv();
        let _timeout_result = event_stream.recv_timeout(Duration::from_millis(1));
    }
}

/// Test the EventStream with different service configurations
#[test]
fn test_event_stream_service_configurations() {
    let speaker = create_test_speaker("uuid:RINCON_123456789::1", "Test Room", "192.168.1.100");

    // Test with only AVTransport
    let av_config = StreamConfig::default().with_enabled_services(vec![ServiceType::AVTransport]);

    let av_stream = EventStream::new(vec![speaker.clone()], av_config)
        .expect("Failed to create AVTransport stream");
    assert!(av_stream.is_active());

    // Test with multiple services
    let multi_config = StreamConfig::default().with_enabled_services(vec![
        ServiceType::AVTransport,
        ServiceType::RenderingControl,
        ServiceType::ContentDirectory,
    ]);

    let multi_stream = EventStream::new(vec![speaker.clone()], multi_config)
        .expect("Failed to create multi-service stream");
    assert!(multi_stream.is_active());

    // Test with empty services (should fail validation)
    let empty_config = StreamConfig::default().with_enabled_services(vec![]);

    let empty_result = EventStream::new(vec![speaker], empty_config);
    assert!(
        empty_result.is_err(),
        "Empty services should fail validation"
    );
}
