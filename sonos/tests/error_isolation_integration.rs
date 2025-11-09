use sonos::streaming::subscription::SubscriptionError;
use sonos::{ServiceType};
use sonos::streaming::SubscriptionScope;

#[test]
fn test_error_isolation_service_classification() {
    // Test that service scope classification works correctly for error isolation
    assert_eq!(
        ServiceType::AVTransport.subscription_scope(),
        SubscriptionScope::PerSpeaker
    );
    assert_eq!(
        ServiceType::RenderingControl.subscription_scope(),
        SubscriptionScope::PerSpeaker
    );
    assert_eq!(
        ServiceType::ZoneGroupTopology.subscription_scope(),
        SubscriptionScope::NetworkWide
    );
}

#[test]
fn test_error_isolation_error_types() {
    // Test new error types for error isolation
    let service_conflict = SubscriptionError::ServiceConflict {
        service: ServiceType::ZoneGroupTopology,
        message: "Test conflict".to_string(),
    };

    let registry_corruption = SubscriptionError::RegistryCorruption {
        message: "Test corruption".to_string(),
    };

    // Verify error types can be cloned (needed for error isolation)
    let _cloned_conflict = service_conflict.clone();
    let _cloned_corruption = registry_corruption.clone();

    // Verify error messages contain service type information
    assert!(service_conflict.to_string().contains("ZoneGroupTopology"));
    assert!(registry_corruption.to_string().contains("corruption"));
}

#[test]
fn test_error_isolation_service_scope_boundaries() {
    // Verify that PerSpeaker and NetworkWide services are properly classified
    // This is critical for error isolation to work correctly

    let per_speaker_services = vec![
        ServiceType::AVTransport,
        ServiceType::RenderingControl,
        ServiceType::ContentDirectory,
    ];

    let network_wide_services = vec![ServiceType::ZoneGroupTopology];

    // All PerSpeaker services should be classified correctly
    for service in per_speaker_services {
        assert_eq!(
            service.subscription_scope(),
            SubscriptionScope::PerSpeaker,
            "Service {:?} should be PerSpeaker for proper error isolation",
            service
        );
    }

    // All NetworkWide services should be classified correctly
    for service in network_wide_services {
        assert_eq!(
            service.subscription_scope(),
            SubscriptionScope::NetworkWide,
            "Service {:?} should be NetworkWide for proper error isolation",
            service
        );
    }
}

#[test]
fn test_error_isolation_error_message_formatting() {
    // Test that error messages include proper service type identification
    // This is important for debugging and monitoring error isolation

    let av_transport_error = SubscriptionError::ServiceConflict {
        service: ServiceType::AVTransport,
        message: "PerSpeaker conflict".to_string(),
    };

    // Error messages should clearly identify the service type
    let av_error_str = av_transport_error.to_string();

    assert!(
        av_error_str.contains("AVTransport"),
        "AVTransport error should mention service type"
    );
}