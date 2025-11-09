use super::subscription::{ServiceSubscription, SubscriptionError, SubscriptionResult};
use super::types::{ServiceType, SubscriptionConfig, SubscriptionId, SubscriptionScope};
use crate::models::{Speaker, SpeakerId, StateChange};
use crate::service::rendering_control;
use crate::transport::soap::SoapClient;
use std::time::SystemTime;

/// RenderingControl service subscription implementation
///
/// This struct handles UPnP subscriptions to the RenderingControl service on Sonos devices,
/// which provides events for volume changes, mute state changes, and other audio rendering properties.
pub struct RenderingControlSubscription {
    /// The speaker this subscription is associated with
    speaker: Speaker,
    /// Current subscription ID (None if not subscribed)
    subscription_id: Option<SubscriptionId>,
    /// UPnP SID (Subscription ID) returned by the device
    upnp_sid: Option<String>,
    /// URL where the device should send event notifications
    callback_url: String,
    /// SOAP client for making UPnP requests (kept for future use)
    #[allow(dead_code)]
    soap_client: SoapClient,
    /// Timestamp of the last successful renewal
    last_renewal: Option<SystemTime>,
    /// Configuration for this subscription
    config: SubscriptionConfig,
    /// Whether the subscription is currently active
    active: bool,
}

impl RenderingControlSubscription {
    /// Create a new RenderingControl subscription
    pub fn new(
        speaker: Speaker,
        callback_url: String,
        config: SubscriptionConfig,
    ) -> SubscriptionResult<Self> {
        let soap_client = SoapClient::new(std::time::Duration::from_secs(30))
            .map_err(|e| SubscriptionError::NetworkError(e.to_string()))?;

        Ok(Self {
            speaker,
            subscription_id: None,
            upnp_sid: None,
            callback_url,
            soap_client,
            last_renewal: None,
            config,
            active: false,
        })
    }

    /// Get the device URL for this speaker
    fn device_url(&self) -> String {
        format!("http://{}:{}", self.speaker.ip_address, self.speaker.port)
    }

    /// Send a UPnP SUBSCRIBE request to establish the subscription
    fn send_subscribe_request(&self) -> SubscriptionResult<String> {
        let device_url = self.device_url();
        let event_sub_url = ServiceType::RenderingControl.event_sub_url();
        let full_url = format!("{}{}", device_url, event_sub_url);

        println!(
            "📡 Sending RenderingControl SUBSCRIBE request to: {}",
            full_url
        );
        println!("   Callback URL: {}", self.callback_url);

        // Create HTTP client for subscription requests with timeout
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| SubscriptionError::NetworkError(e.to_string()))?;

        println!("🔄 Making HTTP SUBSCRIBE request...");
        let response = client
            .request(
                reqwest::Method::from_bytes(b"SUBSCRIBE").unwrap(),
                &full_url,
            )
            .header(
                "HOST",
                format!("{}:{}", self.speaker.ip_address, self.speaker.port),
            )
            .header("CALLBACK", format!("<{}>", self.callback_url))
            .header("NT", "upnp:event")
            .header("TIMEOUT", format!("Second-{}", self.config.timeout_seconds))
            .send()
            .map_err(|e| {
                println!("❌ HTTP request failed: {}", e);
                SubscriptionError::NetworkError(e.to_string())
            })?;

        if !response.status().is_success() {
            return match response.status().as_u16() {
                503 => {
                    // Don't print error message here - let the caller handle satellite speaker detection
                    Err(SubscriptionError::SatelliteSpeaker)
                }
                _ => {
                    let error_msg = format!(
                        "HTTP {} - {}",
                        response.status(),
                        response.status().canonical_reason().unwrap_or("Unknown")
                    );
                    Err(SubscriptionError::SubscriptionFailed(error_msg))
                }
            };
        }

        // Extract SID from response headers
        let sid = response
            .headers()
            .get("SID")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                SubscriptionError::SubscriptionFailed("No SID in response".to_string())
            })?;

        Ok(sid.to_string())
    }

    /// Send a UPnP UNSUBSCRIBE request to terminate the subscription
    fn send_unsubscribe_request(&self, sid: &str) -> SubscriptionResult<()> {
        let device_url = self.device_url();
        let event_sub_url = ServiceType::RenderingControl.event_sub_url();
        let full_url = format!("{}{}", device_url, event_sub_url);

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| SubscriptionError::NetworkError(e.to_string()))?;

        let response = client
            .request(
                reqwest::Method::from_bytes(b"UNSUBSCRIBE").unwrap(),
                &full_url,
            )
            .header(
                "HOST",
                format!("{}:{}", self.speaker.ip_address, self.speaker.port),
            )
            .header("SID", sid)
            .send()
            .map_err(|e| SubscriptionError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(SubscriptionError::SubscriptionFailed(format!(
                "UNSUBSCRIBE failed: HTTP {}",
                response.status()
            )));
        }

        Ok(())
    }

    /// Send a subscription renewal request
    fn send_renewal_request(&self, sid: &str) -> SubscriptionResult<()> {
        let device_url = self.device_url();
        let event_sub_url = ServiceType::RenderingControl.event_sub_url();
        let full_url = format!("{}{}", device_url, event_sub_url);

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| SubscriptionError::NetworkError(e.to_string()))?;

        let response = client
            .request(
                reqwest::Method::from_bytes(b"SUBSCRIBE").unwrap(),
                &full_url,
            )
            .header(
                "HOST",
                format!("{}:{}", self.speaker.ip_address, self.speaker.port),
            )
            .header("SID", sid)
            .header("TIMEOUT", format!("Second-{}", self.config.timeout_seconds))
            .send()
            .map_err(|e| SubscriptionError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(SubscriptionError::SubscriptionFailed(format!(
                "Renewal failed: HTTP {}",
                response.status()
            )));
        }

        Ok(())
    }
}

impl ServiceSubscription for RenderingControlSubscription {
    fn service_type(&self) -> ServiceType {
        ServiceType::RenderingControl
    }

    fn subscription_scope(&self) -> SubscriptionScope {
        SubscriptionScope::PerSpeaker
    }

    fn speaker_id(&self) -> &SpeakerId {
        &self.speaker.id
    }

    fn subscribe(&mut self) -> SubscriptionResult<SubscriptionId> {
        // Send SUBSCRIBE request
        let upnp_sid = self.send_subscribe_request()?;

        // Create subscription ID and update state
        let subscription_id = SubscriptionId::new();
        self.subscription_id = Some(subscription_id);
        self.upnp_sid = Some(upnp_sid);
        self.active = true;
        self.last_renewal = Some(SystemTime::now());

        Ok(subscription_id)
    }

    fn unsubscribe(&mut self) -> SubscriptionResult<()> {
        if let Some(upnp_sid) = &self.upnp_sid {
            self.send_unsubscribe_request(upnp_sid)?;
        }

        self.subscription_id = None;
        self.upnp_sid = None;
        self.active = false;
        self.last_renewal = None;
        Ok(())
    }

    fn renew(&mut self) -> SubscriptionResult<()> {
        if !self.active {
            return Err(SubscriptionError::SubscriptionExpired);
        }

        if let Some(upnp_sid) = &self.upnp_sid {
            self.send_renewal_request(upnp_sid)?;
            self.last_renewal = Some(SystemTime::now());
            Ok(())
        } else {
            Err(SubscriptionError::SubscriptionExpired)
        }
    }

    fn parse_event(&self, event_xml: &str) -> SubscriptionResult<Vec<StateChange>> {
        let mut changes = Vec::new();
        
        // Validate input
        if event_xml.is_empty() {
            println!("⚠️  Received empty event XML, returning no changes");
            return Ok(Vec::new());
        }

        match rendering_control::parser::RenderingControlParser::from_xml(event_xml) {
          Ok(parser) => {
            match parser.get_volume() {
              Some(volume) => changes.push(StateChange::VolumeChanged {
                speaker_id: self.speaker_id().clone(),
                volume
              }),
              None => {}
            }

            match parser.get_mute() {
              Some(mute) => changes.push(StateChange::MuteChanged {
                speaker_id: self.speaker_id().clone(),
                muted: mute
              }),
              None => {}
            }
          }
          Err(e) => {
            println!("no parser, {:?}", e);
          }
        }

        Ok(changes)
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn last_renewal(&self) -> Option<SystemTime> {
        self.last_renewal
    }

    fn subscription_id(&self) -> Option<SubscriptionId> {
        self.subscription_id
    }

    fn get_config(&self) -> &SubscriptionConfig {
        &self.config
    }

    fn callback_url(&self) -> &str {
        &self.callback_url
    }

    fn on_subscription_state_changed(&mut self, active: bool) -> SubscriptionResult<()> {
        self.active = active;
        if !active {
            self.subscription_id = None;
            self.upnp_sid = None;
            self.last_renewal = None;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Speaker;

    fn create_test_speaker() -> Speaker {
        Speaker {
            id: SpeakerId::new("uuid:RINCON_123456789::1"),
            name: "Test Speaker".to_string(),
            room_name: "Test Room".to_string(),
            ip_address: "192.168.1.100".to_string(),
            port: 1400,
            model_name: "Test Model".to_string(),
            satellites: vec![],
        }
    }

    #[test]
    fn test_rendering_control_subscription_creation() {
        let speaker = create_test_speaker();
        let callback_url = "http://localhost:8080/callback/test".to_string();
        let config = SubscriptionConfig::default();

        let subscription =
            RenderingControlSubscription::new(speaker.clone(), callback_url.clone(), config);
        assert!(subscription.is_ok());

        let sub = subscription.unwrap();
        assert_eq!(sub.service_type(), ServiceType::RenderingControl);
        assert_eq!(sub.speaker_id(), speaker.get_id());
        assert_eq!(sub.callback_url(), &callback_url);
        assert!(!sub.is_active());
        assert!(sub.subscription_id().is_none());
    }

    #[test]
    fn test_parse_event_with_volume_and_mute() {
        let speaker = create_test_speaker();
        let subscription = RenderingControlSubscription::new(
            speaker.clone(),
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        let event_xml = r#"
            <property>
                <Volume>75</Volume>
                <Mute>1</Mute>
            </property>
        "#;

        let changes = subscription.parse_event(event_xml).unwrap();
        assert_eq!(changes.len(), 2);

        // Check volume change
        if let StateChange::VolumeChanged { speaker_id, volume } = &changes[0] {
            assert_eq!(*speaker_id, speaker.id);
            assert_eq!(*volume, 75);
        } else {
            panic!("Expected VolumeChanged");
        }

        // Check mute change
        if let StateChange::MuteChanged { speaker_id, muted } = &changes[1] {
            assert_eq!(*speaker_id, speaker.id);
            assert_eq!(*muted, true);
        } else {
            panic!("Expected MuteChanged");
        }
    }

    #[test]
    fn test_parse_event_with_lastchange() {
        let speaker = create_test_speaker();
        let subscription = RenderingControlSubscription::new(
            speaker.clone(),
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        let event_xml = r#"
            <property>
                <LastChange>&lt;Event xmlns="urn:schemas-upnp-org:metadata-1-0/RCS/"&gt;&lt;InstanceID val="0"&gt;&lt;Volume channel="Master" val="25"/&gt;&lt;Mute channel="Master" val="0"/&gt;&lt;/InstanceID&gt;&lt;/Event&gt;</LastChange>
            </property>
        "#;

        let changes = subscription.parse_event(event_xml).unwrap();
        assert_eq!(changes.len(), 2);

        // Check volume change from LastChange
        if let StateChange::VolumeChanged { speaker_id, volume } = &changes[0] {
            assert_eq!(*speaker_id, speaker.id);
            assert_eq!(*volume, 25);
        } else {
            panic!("Expected VolumeChanged");
        }

        // Check mute change from LastChange
        if let StateChange::MuteChanged { speaker_id, muted } = &changes[1] {
            assert_eq!(*speaker_id, speaker.id);
            assert_eq!(*muted, false);
        } else {
            panic!("Expected MuteChanged");
        }
    }

    #[test]
    fn test_parse_event_no_changes() {
        let speaker = create_test_speaker();
        let subscription = RenderingControlSubscription::new(
            speaker,
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        let event_xml = r#"
            <property>
                <TransportState>PLAYING</TransportState>
                <CurrentTrackURI>x-sonos-spotify:spotify%3atrack%3a123</CurrentTrackURI>
            </property>
        "#;

        let changes = subscription.parse_event(event_xml).unwrap();
        assert_eq!(changes.len(), 0); // No RenderingControl-related changes
    }

    #[test]
    fn test_service_subscription_trait_implementation() {
        let speaker = create_test_speaker();
        let callback_url = "http://localhost:8080/callback/test".to_string();
        let config = SubscriptionConfig::default();

        let mut subscription = RenderingControlSubscription::new(
            speaker.clone(),
            callback_url.clone(),
            config.clone(),
        )
        .unwrap();

        // Test trait methods before subscription
        assert_eq!(subscription.service_type(), ServiceType::RenderingControl);
        assert_eq!(subscription.speaker_id(), speaker.get_id());
        assert_eq!(subscription.callback_url(), &callback_url);
        assert!(!subscription.is_active());
        assert!(subscription.subscription_id().is_none());
        assert!(subscription.last_renewal().is_none());
        assert_eq!(
            subscription.get_config().timeout_seconds,
            config.timeout_seconds
        );

        // Test state change handler
        assert!(subscription.on_subscription_state_changed(false).is_ok());
        assert!(!subscription.is_active());
    }

    #[test]
    fn test_parsing_error_handling() {
        let speaker = create_test_speaker();
        let subscription = RenderingControlSubscription::new(
            speaker,
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        // Test malformed XML handling
        let malformed_xml = "<property><Volume>50</Volum"; // Missing closing tag
        let changes = subscription.parse_event(malformed_xml).unwrap();
        assert_eq!(changes.len(), 0); // Should handle gracefully

        // Test empty XML
        let empty_xml = "";
        let changes = subscription.parse_event(empty_xml).unwrap();
        assert_eq!(changes.len(), 0);

        // Test XML with invalid characters
        let invalid_xml = "<property><Volume>\x00\x01\x02</Volume></property>";
        let changes = subscription.parse_event(invalid_xml).unwrap();
        assert_eq!(changes.len(), 0); // Should handle gracefully

        // Test extremely large XML (potential DoS)
        let large_xml = format!("<property><Volume>{}</Volume></property>", "x".repeat(10000));
        let changes = subscription.parse_event(&large_xml).unwrap();
        assert_eq!(changes.len(), 0); // Should handle gracefully
    }

    #[test]
    fn test_subscription_lifecycle_state_management() {
        let speaker = create_test_speaker();
        let callback_url = "http://localhost:8080/callback/test".to_string();
        let config = SubscriptionConfig::default();

        let mut subscription = RenderingControlSubscription::new(
            speaker.clone(),
            callback_url.clone(),
            config.clone(),
        )
        .unwrap();

        // Initial state
        assert!(!subscription.is_active());
        assert!(subscription.subscription_id().is_none());
        assert!(subscription.last_renewal().is_none());
        assert!(subscription.upnp_sid.is_none());

        // Test state change to active (simulating successful subscription)
        subscription.active = true;
        subscription.subscription_id = Some(SubscriptionId::new());
        subscription.upnp_sid = Some("uuid:test-sid".to_string());
        subscription.last_renewal = Some(SystemTime::now());

        assert!(subscription.is_active());
        assert!(subscription.subscription_id().is_some());
        assert!(subscription.last_renewal().is_some());

        // Test unsubscribe state changes
        let result = subscription.on_subscription_state_changed(false);
        assert!(result.is_ok());
        assert!(!subscription.is_active());
        assert!(subscription.subscription_id().is_none());
        assert!(subscription.last_renewal().is_none());
    }

    #[test]
    fn test_device_url_generation() {
        let speaker = create_test_speaker();
        let subscription = RenderingControlSubscription::new(
            speaker.clone(),
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        let expected_url = format!("http://{}:{}", speaker.ip_address, speaker.port);
        assert_eq!(subscription.device_url(), expected_url);
        assert_eq!(subscription.device_url(), "http://192.168.1.100:1400");
    }

    #[test]
    fn test_subscription_error_handling() {
        let speaker = create_test_speaker();
        let mut subscription = RenderingControlSubscription::new(
            speaker,
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        // Test renew when not active
        let result = subscription.renew();
        assert!(result.is_err());
        match result.unwrap_err() {
            SubscriptionError::SubscriptionExpired => {}, // Expected
            _ => panic!("Expected SubscriptionExpired error"),
        }

        // Test renew when active but no SID
        subscription.active = true;
        subscription.upnp_sid = None;
        let result = subscription.renew();
        assert!(result.is_err());
        match result.unwrap_err() {
            SubscriptionError::SubscriptionExpired => {}, // Expected
            _ => panic!("Expected SubscriptionExpired error"),
        }
    }

    #[test]
    fn test_lastchange_parsing_comprehensive() {
        let speaker = create_test_speaker();
        let subscription = RenderingControlSubscription::new(
            speaker.clone(),
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        // Test complex LastChange with multiple properties
        let event_xml = r#"
            <e:propertyset xmlns:e="urn:schemas-upnp-org:event-1-0">
                <e:property>
                    <LastChange>&lt;Event xmlns="urn:schemas-upnp-org:metadata-1-0/RCS/"&gt;&lt;InstanceID val="0"&gt;&lt;Volume channel="Master" val="85"/&gt;&lt;Mute channel="Master" val="1"/&gt;&lt;Bass val="0"/&gt;&lt;Treble val="0"/&gt;&lt;/InstanceID&gt;&lt;/Event&gt;</LastChange>
                </e:property>
            </e:propertyset>
        "#;

        let changes = subscription.parse_event(event_xml).unwrap();
        assert_eq!(changes.len(), 2);

        // Verify volume change
        let volume_change = changes.iter().find(|c| matches!(c, StateChange::VolumeChanged { .. }));
        assert!(volume_change.is_some());
        if let StateChange::VolumeChanged { speaker_id, volume } = volume_change.unwrap() {
            assert_eq!(*speaker_id, speaker.id);
            assert_eq!(*volume, 85);
        }

        // Verify mute change
        let mute_change = changes.iter().find(|c| matches!(c, StateChange::MuteChanged { .. }));
        assert!(mute_change.is_some());
        if let StateChange::MuteChanged { speaker_id, muted } = mute_change.unwrap() {
            assert_eq!(*speaker_id, speaker.id);
            assert_eq!(*muted, true);
        }
    }

    #[test]
    fn test_lastchange_parsing_malformed() {
        let speaker = create_test_speaker();
        let subscription = RenderingControlSubscription::new(
            speaker,
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        // Test LastChange with malformed escaped XML
        let event_xml = r#"
            <property>
                <LastChange>&lt;Event&gt;&lt;InstanceID val="0"&gt;&lt;Volume channel="Master" val="50"&gt;&lt;/Event&gt;</LastChange>
            </property>
        "#;

        let changes = subscription.parse_event(event_xml).unwrap();
        // Should handle gracefully, may or may not extract volume depending on parsing robustness
        assert!(changes.len() <= 1);
    }

    #[test]
    fn test_error_handling_comprehensive() {
        let speaker = create_test_speaker();
        let subscription = RenderingControlSubscription::new(
            speaker,
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        // Test various malformed XML scenarios
        let malformed_cases = vec![
            "", // Empty
            "<", // Incomplete tag
            "<property>", // Unclosed tag
            "<property><Volume>50</property>", // Mismatched tags
            "<property><Volume>50</Volume><property>", // Malformed structure
            "Not XML at all", // Not XML
            "<property><Volume>abc</Volume></property>", // Invalid volume value
            "<property><Mute>maybe</Mute></property>", // Invalid mute value
        ];

        for malformed_xml in malformed_cases {
            let result = subscription.parse_event(malformed_xml);
            assert!(result.is_ok(), "Should handle malformed XML gracefully: {}", malformed_xml);
            let changes = result.unwrap();
            // Should either parse successfully or return empty changes, but not crash
            assert!(changes.len() <= 2, "Should not generate invalid changes");
        }
    }
}
