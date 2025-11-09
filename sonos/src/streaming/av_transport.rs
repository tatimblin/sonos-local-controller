use super::subscription::{ServiceSubscription, SubscriptionError, SubscriptionResult};
use super::types::{ServiceType, SubscriptionConfig, SubscriptionId, SubscriptionScope};
use crate::models::{Speaker, SpeakerId, StateChange};
use crate::service::av_transport;
use crate::transport::soap::SoapClient;
use std::time::SystemTime;

/// AVTransport service subscription implementation
///
/// This struct handles UPnP subscriptions to the AVTransport service on Sonos devices,
/// which provides events for playback state changes, track information, and transport status.
pub struct AVTransportSubscription {
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

impl AVTransportSubscription {
    /// Create a new AVTransport subscription
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
        let event_sub_url = ServiceType::AVTransport.event_sub_url();
        let full_url = format!("{}{}", device_url, event_sub_url);

        println!("📡 Sending SUBSCRIBE request to: {}", full_url);
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
        let event_sub_url = ServiceType::AVTransport.event_sub_url();
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
        let event_sub_url = ServiceType::AVTransport.event_sub_url();
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

impl ServiceSubscription for AVTransportSubscription {
    fn service_type(&self) -> ServiceType {
        ServiceType::AVTransport
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

        match av_transport::parser::AVTransportParser::from_xml(event_xml) {
            Ok(parser) => {
                match parser.get_playback_state() {
                    Some(state) => changes.push(StateChange::PlaybackStateChanged {
                        speaker_id: self.speaker_id().clone(),
                        state,
                    }),
                    None => {}
                }

                match parser.get_track_info() {
                    Some(track_info) => changes.push(StateChange::TrackChanged {
                        speaker_id: self.speaker_id().clone(),
                        track_info: Some(track_info),
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
    use crate::{PlaybackState, models::Speaker};

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
    fn test_av_transport_subscription_creation() {
        let speaker = create_test_speaker();
        let callback_url = "http://localhost:8080/callback/test".to_string();
        let config = SubscriptionConfig::default();

        let subscription =
            AVTransportSubscription::new(speaker.clone(), callback_url.clone(), config);
        assert!(subscription.is_ok());

        let sub = subscription.unwrap();
        assert_eq!(sub.service_type(), ServiceType::AVTransport);
        assert_eq!(sub.speaker_id(), speaker.get_id());
        assert_eq!(sub.callback_url(), &callback_url);
        assert!(!sub.is_active());
        assert!(sub.subscription_id().is_none());
    }

    #[test]
    fn test_parse_event_with_state_change() {
        let speaker = create_test_speaker();
        let subscription = AVTransportSubscription::new(
            speaker.clone(),
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        let event_xml = r#"
            <property>
                <TransportState>PLAYING</TransportState>
                <CurrentTrackMetaData>&lt;DIDL-Lite&gt;&lt;item&gt;&lt;dc:title&gt;Test Song&lt;/dc:title&gt;&lt;dc:creator&gt;Test Artist&lt;/dc:creator&gt;&lt;/item&gt;&lt;/DIDL-Lite&gt;</CurrentTrackMetaData>
            </property>
        "#;

        let changes = subscription.parse_event(event_xml).unwrap();
        assert_eq!(changes.len(), 2);

        // Check playback state change
        if let StateChange::PlaybackStateChanged { speaker_id, state } = &changes[0] {
            assert_eq!(*speaker_id, speaker.id);
            assert_eq!(*state, PlaybackState::Playing);
        } else {
            panic!("Expected PlaybackStateChanged");
        }

        // Check track change
        if let StateChange::TrackChanged {
            speaker_id,
            track_info,
        } = &changes[1]
        {
            assert_eq!(*speaker_id, speaker.id);
            assert!(track_info.is_some());
            let track = track_info.as_ref().unwrap();
            assert_eq!(track.title, Some("Test Song".to_string()));
            assert_eq!(track.artist, Some("Test Artist".to_string()));
        } else {
            panic!("Expected TrackChanged");
        }
    }

    #[test]
    fn test_parse_event_with_complete_track_info() {
        let speaker = create_test_speaker();
        let subscription = AVTransportSubscription::new(
            speaker.clone(),
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        let event_xml = r#"
            <property>
                <CurrentTrackMetaData>&lt;DIDL-Lite&gt;&lt;item&gt;&lt;dc:title&gt;Amazing Song&lt;/dc:title&gt;&lt;dc:creator&gt;Great Artist&lt;/dc:creator&gt;&lt;upnp:album&gt;Best Album&lt;/upnp:album&gt;&lt;/item&gt;&lt;/DIDL-Lite&gt;</CurrentTrackMetaData>
                <CurrentTrackDuration>0:04:32</CurrentTrackDuration>
                <CurrentTrackURI>x-sonos-spotify:spotify%3atrack%3a123456</CurrentTrackURI>
            </property>
        "#;

        let changes = subscription.parse_event(event_xml).unwrap();
        assert_eq!(changes.len(), 1);

        if let StateChange::TrackChanged {
            speaker_id,
            track_info,
        } = &changes[0]
        {
            assert_eq!(*speaker_id, speaker.id);
            assert!(track_info.is_some());
            let track = track_info.as_ref().unwrap();
            assert_eq!(track.title, Some("Amazing Song".to_string()));
            assert_eq!(track.artist, Some("Great Artist".to_string()));
            assert_eq!(track.album, Some("Best Album".to_string()));
            assert_eq!(track.duration_ms, Some(272000)); // 4:32 = 272 seconds
            assert_eq!(
                track.uri,
                Some("x-sonos-spotify:spotify%3atrack%3a123456".to_string())
            );
        } else {
            panic!("Expected TrackChanged");
        }
    }

    #[test]
    fn test_parse_event_empty_metadata() {
        let speaker = create_test_speaker();
        let subscription = AVTransportSubscription::new(
            speaker.clone(),
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        let event_xml = r#"
            <property>
                <TransportState>STOPPED</TransportState>
                <CurrentTrackMetaData>NOT_IMPLEMENTED</CurrentTrackMetaData>
            </property>
        "#;

        let changes = subscription.parse_event(event_xml).unwrap();
        assert_eq!(changes.len(), 1);

        // Should only have state change, no track change for empty metadata
        if let StateChange::PlaybackStateChanged { speaker_id, state } = &changes[0] {
            assert_eq!(*speaker_id, speaker.id);
            assert_eq!(*state, PlaybackState::Stopped);
        } else {
            panic!("Expected PlaybackStateChanged");
        }
    }

    #[test]
    fn test_parse_event_no_changes() {
        let speaker = create_test_speaker();
        let subscription = AVTransportSubscription::new(
            speaker,
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        let event_xml = r#"
            <property>
                <Volume>50</Volume>
                <Mute>0</Mute>
            </property>
        "#;

        let changes = subscription.parse_event(event_xml).unwrap();
        assert_eq!(changes.len(), 0); // No AVTransport-related changes
    }

    #[test]
    fn test_service_subscription_trait_implementation() {
        let speaker = create_test_speaker();
        let callback_url = "http://localhost:8080/callback/test".to_string();
        let config = SubscriptionConfig::default();

        let mut subscription =
            AVTransportSubscription::new(speaker.clone(), callback_url.clone(), config.clone())
                .unwrap();

        // Test trait methods before subscription
        assert_eq!(subscription.service_type(), ServiceType::AVTransport);
        assert_eq!(subscription.speaker_id(), speaker.get_id());
        assert_eq!(subscription.callback_url(), &callback_url);
        assert!(!subscription.is_active());
        assert!(subscription.subscription_id().is_none());
        assert!(subscription.last_renewal().is_none());
        assert!(!subscription.needs_renewal());

        // Test configuration access
        let retrieved_config = subscription.get_config();
        assert_eq!(retrieved_config.timeout_seconds, config.timeout_seconds);
        assert_eq!(
            retrieved_config.max_retry_attempts,
            config.max_retry_attempts
        );

        // Test state change handler
        assert!(subscription.on_subscription_state_changed(true).is_ok());
        assert!(subscription.active); // Should be set to true

        assert!(subscription.on_subscription_state_changed(false).is_ok());
        assert!(!subscription.active); // Should be set to false
        assert!(subscription.subscription_id().is_none()); // Should be cleared
        assert!(subscription.last_renewal().is_none()); // Should be cleared
    }

    #[test]
    fn test_subscription_lifecycle_trait_methods() {
        let speaker = create_test_speaker();
        let mut subscription = AVTransportSubscription::new(
            speaker,
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        // Test renewal when not active (should fail)
        let renew_result = subscription.renew();
        assert!(renew_result.is_err());
        assert!(matches!(
            renew_result.unwrap_err(),
            SubscriptionError::SubscriptionExpired
        ));

        // Test unsubscribe when not subscribed (should succeed but do nothing)
        assert!(subscription.unsubscribe().is_ok());
        assert!(!subscription.is_active());
    }
}
