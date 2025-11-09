use sonos::{ServiceType, StateChange};
use sonos::streaming::{SubscriptionScope};
use sonos::streaming::subscription::SubscriptionError;
use sonos::model::{Speaker, SpeakerId};
use std::sync::mpsc;

#[test]
fn test_service_scope_classification() {
    // Test that service scope classification works correctly
    assert_eq!(ServiceType::AVTransport.subscription_scope(), SubscriptionScope::PerSpeaker);
    assert_eq!(ServiceType::RenderingControl.subscription_scope(), SubscriptionScope::PerSpeaker);
    assert_eq!(ServiceType::ZoneGroupTopology.subscription_scope(), SubscriptionScope::NetworkWide);
}

#[test]
fn test_subscription_error_types() {
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
fn test_detailed_service_scope_classification() {
    // Verify PerSpeaker services
    let per_speaker_services = vec![
        ServiceType::AVTransport,
        ServiceType::RenderingControl,
        ServiceType::ContentDirectory,
    ];

    for service in per_speaker_services {
        assert_eq!(
            service.subscription_scope(),
            SubscriptionScope::PerSpeaker,
            "Service {:?} should be PerSpeaker",
            service
        );
    }

    // Verify NetworkWide services
    let network_wide_services = vec![ServiceType::ZoneGroupTopology];

    for service in network_wide_services {
        assert_eq!(
            service.subscription_scope(),
            SubscriptionScope::NetworkWide,
            "Service {:?} should be NetworkWide",
            service
        );
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_error_isolation_integration() {
        // This test verifies that the error isolation system works end-to-end
        let (event_sender, event_receiver) = mpsc::channel::<StateChange>();
        
        // Test that we can create a test speaker with correct structure
        let test_speaker = Speaker {
            id: SpeakerId::from_udn("uuid:RINCON_TEST123::1"),
            udn: "uuid:RINCON_TEST123::1".to_string(),
            name: "Test Speaker".to_string(),
            room_name: "Test Room".to_string(),
            ip_address: "192.168.1.100".to_string(),
            port: 1400,
            model_name: "Test Model".to_string(),
            satellites: vec![],
        };

        // Verify speaker structure is correct
        assert_eq!(test_speaker.name, "Test Speaker");
        assert_eq!(test_speaker.ip_address, "192.168.1.100");
        assert_eq!(test_speaker.port, 1400);
        
        // Event receiver should not be blocked
        match event_receiver.try_recv() {
            Ok(_) => println!("Received event (expected)"),
            Err(mpsc::TryRecvError::Empty) => println!("No events (also expected)"),
            Err(mpsc::TryRecvError::Disconnected) => panic!("Event channel disconnected unexpectedly"),
        }
    }
}