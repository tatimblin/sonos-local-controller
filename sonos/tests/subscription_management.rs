use sonos::streaming::{ServiceType, StreamConfig, SubscriptionId};
use std::time::Duration;

/// Test 1: Basic configuration validation
/// Simple test that doesn't create any managers or network connections
#[test]
fn test_basic_configuration() {
    // Test valid configuration creation
    let config = StreamConfig::default();
    assert_eq!(config.enabled_services, vec![ServiceType::AVTransport]);
    assert_eq!(config.buffer_size, 1000);

    // Test configuration validation
    assert!(config.validate().is_ok());

    // Test invalid configurations
    assert!(StreamConfig::default().with_buffer_size(0).is_err());
    assert!(StreamConfig::default().with_retry_attempts(20).is_err());
    assert!(StreamConfig::default()
        .with_callback_port_range(8090, 8080)
        .is_err());
}

/// Test 2: Subscription ID management
/// Tests ID generation and conversion without network calls
#[test]
fn test_subscription_id_operations() {
    // Test ID creation and uniqueness
    let id1 = SubscriptionId::new();
    let id2 = SubscriptionId::new();
    assert_ne!(id1, id2);

    // Test string conversion
    let id_str = id1.as_string();
    let id3 = SubscriptionId::from_string(&id_str).unwrap();
    assert_eq!(id1, id3);

    // Test display formatting
    let display_str = format!("{}", id1);
    assert_eq!(display_str, id_str);
}

/// Test 3: Service type functionality
/// Tests service type enum operations
#[test]
fn test_service_types() {
    // Test service type URNs
    assert_eq!(
        ServiceType::AVTransport.service_type_urn(),
        "urn:schemas-upnp-org:service:AVTransport:1"
    );
    assert_eq!(
        ServiceType::RenderingControl.service_type_urn(),
        "urn:schemas-upnp-org:service:RenderingControl:1"
    );

    // Test control URLs
    assert_eq!(
        ServiceType::AVTransport.control_url(),
        "/MediaRenderer/AVTransport/Control"
    );

    // Test event subscription URLs
    assert_eq!(
        ServiceType::AVTransport.event_sub_url(),
        "/MediaRenderer/AVTransport/Event"
    );
}

/// Test 4: Configuration builder pattern
/// Tests the configuration builder methods
#[test]
fn test_configuration_builder() {
    let config = StreamConfig::default()
        .with_buffer_size(500)
        .unwrap()
        .with_subscription_timeout(Duration::from_secs(900))
        .unwrap()
        .with_retry_attempts(5)
        .unwrap()
        .with_retry_backoff(Duration::from_millis(500))
        .with_enabled_services(vec![
            ServiceType::AVTransport,
            ServiceType::RenderingControl,
        ])
        .with_callback_port_range(9000, 9010)
        .unwrap();

    assert_eq!(config.buffer_size, 500);
    assert_eq!(config.subscription_timeout, Duration::from_secs(900));
    assert_eq!(config.retry_attempts, 5);
    assert_eq!(config.retry_backoff, Duration::from_millis(500));
    assert_eq!(config.enabled_services.len(), 2);
    assert_eq!(config.callback_port_range, (9000, 9010));

    assert!(config.validate().is_ok());
}

/// Test 5: Error conditions
/// Tests various error conditions in configuration
#[test]
fn test_error_conditions() {
    // Test buffer size errors
    assert!(StreamConfig::default().with_buffer_size(0).is_err());
    assert!(StreamConfig::default().with_buffer_size(200_000).is_err());

    // Test timeout errors
    assert!(StreamConfig::default()
        .with_subscription_timeout(Duration::from_secs(30))
        .is_err());
    assert!(StreamConfig::default()
        .with_subscription_timeout(Duration::from_secs(100_000))
        .is_err());

    // Test retry attempt errors
    assert!(StreamConfig::default().with_retry_attempts(15).is_err());

    // Test port range errors
    assert!(StreamConfig::default()
        .with_callback_port_range(8080, 8080)
        .is_err());
    assert!(StreamConfig::default()
        .with_callback_port_range(500, 600)
        .is_err());

    // Test empty services validation
    let empty_config = StreamConfig::default().with_enabled_services(vec![]);
    assert!(empty_config.validate().is_err());
}

/// Test 6: Subscription manager shutdown and cleanup
/// Tests that subscriptions are properly created and then all closed during shutdown
#[test]
fn test_subscription_shutdown_cleanup() {
    use sonos::streaming::SubscriptionManager;
    use sonos::{discover_speakers_with_timeout, SonosError};
    use std::sync::mpsc;
    use std::time::Duration;

    // Discover real speakers from the network
    let speakers = match discover_speakers_with_timeout(Duration::from_secs(5)) {
        Ok(speakers) => speakers,
        Err(SonosError::DiscoveryFailed(_)) => {
            println!("No Sonos speakers found on network - skipping test");
            return;
        }
        Err(e) => panic!("Discovery failed with unexpected error: {:?}", e),
    };

    if speakers.is_empty() {
        println!("No Sonos speakers discovered - skipping test");
        return;
    }

    println!("Discovered {} Sonos speakers for shutdown test:", speakers.len());
    for speaker in &speakers {
        println!(
            "  - {} ({}) at {}",
            speaker.name, speaker.model_name, speaker.ip_address
        );
    }

    let config = StreamConfig::default();
    let (event_sender, _event_receiver) = mpsc::channel();
    let mut manager = SubscriptionManager::new(config, event_sender).unwrap();

    // Add all discovered speakers to create subscriptions
    for speaker in &speakers {
        let result = manager.add_speaker(speaker.clone());
        println!("Added speaker {}: {:?}", speaker.name, result.is_ok());
    }

    // Verify speakers are tracked
    assert_eq!(manager.speaker_count(), speakers.len());

    // Get initial subscription count (may be 0 if network connections fail, but that's ok)
    let initial_subscription_count = manager.subscription_count();
    println!("Initial subscription count: {}", initial_subscription_count);

    // Get subscription info before shutdown
    let subscription_info_before = manager.get_subscription_info();
    println!(
        "Subscriptions before shutdown: {}",
        subscription_info_before.len()
    );

    // Verify callback server is running
    let callback_port = manager.callback_server_port();
    assert!(callback_port.is_some());
    println!("Callback server running on port: {:?}", callback_port);

    // Perform shutdown
    let shutdown_result = manager.shutdown();
    assert!(shutdown_result.is_ok(), "Shutdown should succeed");

    // Verify all subscriptions are cleaned up
    assert_eq!(
        manager.subscription_count(),
        0,
        "All subscriptions should be removed"
    );
    assert_eq!(manager.speaker_count(), 0, "All speakers should be removed");

    // Verify callback server is shut down
    assert!(
        manager.callback_server_port().is_none(),
        "Callback server should be shut down"
    );

    // Get subscription info after shutdown (should be empty)
    let subscription_info_after = manager.get_subscription_info();
    assert_eq!(
        subscription_info_after.len(),
        0,
        "No subscriptions should remain after shutdown"
    );

    println!("Shutdown cleanup test completed successfully with {} real speakers", speakers.len());
}
