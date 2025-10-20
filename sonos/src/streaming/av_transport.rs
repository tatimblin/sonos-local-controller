use super::subscription::{ServiceSubscription, SubscriptionError, SubscriptionResult};
use super::types::{ServiceType, SubscriptionConfig, SubscriptionId, SubscriptionScope};
use crate::models::{PlaybackState, Speaker, SpeakerId, StateChange, TrackInfo};
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

    /// Parse transport state from UPnP event XML
    fn parse_transport_state(&self, xml: &str) -> SubscriptionResult<Option<PlaybackState>> {
        println!("🔍 Parsing transport state from XML...");
        println!("   XML length: {} bytes", xml.len());
        println!("   XML preview: {}", xml.chars().take(200).collect::<String>());
        
        // Look for TransportState in the event XML
        if let Some(state_str) = self.extract_property_value(xml, "TransportState") {
            println!("✅ Found TransportState: {}", state_str);
            match state_str.as_str() {
                "PLAYING" => Ok(Some(PlaybackState::Playing)),
                "PAUSED_PLAYBACK" => Ok(Some(PlaybackState::Paused)),
                "STOPPED" => Ok(Some(PlaybackState::Stopped)),
                "TRANSITIONING" => Ok(Some(PlaybackState::Transitioning)),
                _ => {
                    println!("⚠️  Unknown transport state: {}", state_str);
                    Ok(None) // Unknown state, ignore
                }
            }
        } else {
            println!("❌ No TransportState found in XML");
            
            // Try to extract LastChange content for debugging
            if let Some(last_change) = self.extract_property_value(xml, "LastChange") {
                println!("🔍 Found LastChange content:");
                println!("   {}", last_change.chars().take(300).collect::<String>());
                
                // Try to parse the escaped XML in LastChange
                let decoded = self.decode_xml_entities(&last_change);
                println!("🔍 Decoded LastChange:");
                println!("   {}", decoded.chars().take(300).collect::<String>());
                
                // Look for TransportState in the decoded content
                if decoded.contains("TransportState") {
                    println!("✅ Found TransportState in decoded LastChange!");
                    // Try to extract the val attribute
                    if let Some(start) = decoded.find("TransportState val=\"") {
                        let content_start = start + "TransportState val=\"".len();
                        if let Some(end) = decoded[content_start..].find("\"") {
                            let state_value = &decoded[content_start..content_start + end];
                            println!("✅ Extracted TransportState value: {}", state_value);
                            
                            match state_value {
                                "PLAYING" => return Ok(Some(PlaybackState::Playing)),
                                "PAUSED_PLAYBACK" => return Ok(Some(PlaybackState::Paused)),
                                "STOPPED" => return Ok(Some(PlaybackState::Stopped)),
                                "TRANSITIONING" => return Ok(Some(PlaybackState::Transitioning)),
                                _ => {
                                    println!("⚠️  Unknown transport state in LastChange: {}", state_value);
                                    return Ok(None);
                                }
                            }
                        }
                    }
                }
            }
            
            Ok(None) // No transport state in this event
        }
    }

    /// Parse current track information from UPnP event XML
    fn parse_current_track_info(&self, xml: &str) -> SubscriptionResult<Option<TrackInfo>> {
        // Look for CurrentTrackMetaData in the event XML
        if let Some(metadata_xml) = self.extract_property_value(xml, "CurrentTrackMetaData") {
            if metadata_xml.is_empty() || metadata_xml == "NOT_IMPLEMENTED" {
                return Ok(None);
            }

            // Parse DIDL-Lite metadata
            let title = self.extract_didl_value(&metadata_xml, "dc:title");
            let artist = self.extract_didl_value(&metadata_xml, "dc:creator");
            let album = self.extract_didl_value(&metadata_xml, "upnp:album");

            // Parse duration from CurrentTrackDuration property
            let duration_ms = self
                .extract_property_value(xml, "CurrentTrackDuration")
                .and_then(|duration_str| self.parse_duration(&duration_str));

            // Parse URI from CurrentTrackURI property
            let uri = self.extract_property_value(xml, "CurrentTrackURI");

            // Only return TrackInfo if we have at least some information
            if title.is_some() || artist.is_some() || album.is_some() || uri.is_some() {
                Ok(Some(TrackInfo {
                    title,
                    artist,
                    album,
                    duration_ms,
                    uri,
                }))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Extract a property value from UPnP event XML
    fn extract_property_value(&self, xml: &str, property_name: &str) -> Option<String> {
        // UPnP events use a specific XML structure with <property> elements
        // Handle both namespaced and non-namespaced property elements
        let property_patterns = [
            ("<property>", "</property>"),
            ("<e:property>", "</e:property>"),
        ];
        
        let var_start = format!("<{}>", property_name);
        let var_end = format!("</{}>", property_name);

        // Try each property pattern
        for (property_start, property_end) in &property_patterns {
            let mut search_pos = 0;
            while let Some(prop_start) = xml[search_pos..].find(property_start) {
                let prop_start_abs = search_pos + prop_start;
                if let Some(prop_end) = xml[prop_start_abs..].find(property_end) {
                    let prop_end_abs = prop_start_abs + prop_end + property_end.len();
                    let property_xml = &xml[prop_start_abs..prop_end_abs];

                    // Look for our variable within this property block
                    if let Some(var_start_pos) = property_xml.find(&var_start) {
                        if let Some(var_end_pos) = property_xml[var_start_pos..].find(&var_end) {
                            let content_start = var_start_pos + var_start.len();
                            let content_end = var_start_pos + var_end_pos;
                            let content = &property_xml[content_start..content_end];

                            // Decode XML entities
                            return Some(self.decode_xml_entities(content));
                        }
                    }

                    search_pos = prop_end_abs;
                } else {
                    break;
                }
            }
        }

        None
    }

    /// Extract a value from DIDL-Lite XML metadata
    fn extract_didl_value(&self, didl_xml: &str, tag: &str) -> Option<String> {
        let start_tag = format!("<{}>", tag);
        let end_tag = format!("</{}>", tag);

        if let Some(start_pos) = didl_xml.find(&start_tag) {
            let content_start = start_pos + start_tag.len();
            if let Some(end_pos) = didl_xml[content_start..].find(&end_tag) {
                let content = &didl_xml[content_start..content_start + end_pos];
                return Some(self.decode_xml_entities(content));
            }
        }

        None
    }

    /// Parse duration string (HH:MM:SS or HH:MM:SS.mmm) to milliseconds
    fn parse_duration(&self, duration_str: &str) -> Option<u64> {
        let parts: Vec<&str> = duration_str.split(':').collect();
        if parts.len() >= 3 {
            let hours: u64 = parts[0].parse().ok()?;
            let minutes: u64 = parts[1].parse().ok()?;

            // Handle seconds with optional milliseconds
            let seconds_part = parts[2];
            let seconds: f64 = seconds_part.parse().ok()?;

            let total_ms = (hours * 3600 + minutes * 60) * 1000 + (seconds * 1000.0) as u64;
            Some(total_ms)
        } else {
            None
        }
    }

    /// Decode basic XML entities
    fn decode_xml_entities(&self, text: &str) -> String {
        text.replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
    }
}

impl ServiceSubscription for AVTransportSubscription {
    fn service_type(&self) -> ServiceType {
        ServiceType::AVTransport
    }

    fn subscription_scope(&self) -> SubscriptionScope {
        SubscriptionScope::PerSpeaker
    }

    fn speaker_id(&self) -> SpeakerId {
        self.speaker.id
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

        // Parse transport state changes
        if let Some(playback_state) = self.parse_transport_state(event_xml)? {
            changes.push(StateChange::PlaybackStateChanged {
                speaker_id: self.speaker.id,
                state: playback_state,
            });
        }

        // Parse track information changes
        if let Some(track_info) = self.parse_current_track_info(event_xml)? {
            changes.push(StateChange::TrackChanged {
                speaker_id: self.speaker.id,
                track_info: Some(track_info),
            });
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
            id: SpeakerId::from_udn("uuid:RINCON_123456789::1"),
            udn: "uuid:RINCON_123456789::1".to_string(),
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
        assert_eq!(sub.speaker_id(), speaker.id);
        assert_eq!(sub.callback_url(), &callback_url);
        assert!(!sub.is_active());
        assert!(sub.subscription_id().is_none());
    }

    #[test]
    fn test_parse_transport_state() {
        let speaker = create_test_speaker();
        let subscription = AVTransportSubscription::new(
            speaker,
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        // Test PLAYING state
        let playing_xml = r#"
            <property>
                <TransportState>PLAYING</TransportState>
            </property>
        "#;
        let state = subscription.parse_transport_state(playing_xml).unwrap();
        assert_eq!(state, Some(PlaybackState::Playing));

        // Test PAUSED state
        let paused_xml = r#"
            <property>
                <TransportState>PAUSED_PLAYBACK</TransportState>
            </property>
        "#;
        let state = subscription.parse_transport_state(paused_xml).unwrap();
        assert_eq!(state, Some(PlaybackState::Paused));

        // Test STOPPED state
        let stopped_xml = r#"
            <property>
                <TransportState>STOPPED</TransportState>
            </property>
        "#;
        let state = subscription.parse_transport_state(stopped_xml).unwrap();
        assert_eq!(state, Some(PlaybackState::Stopped));

        // Test no transport state
        let no_state_xml = r#"
            <property>
                <SomeOtherProperty>value</SomeOtherProperty>
            </property>
        "#;
        let state = subscription.parse_transport_state(no_state_xml).unwrap();
        assert_eq!(state, None);
    }

    #[test]
    fn test_parse_duration() {
        let speaker = create_test_speaker();
        let subscription = AVTransportSubscription::new(
            speaker,
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        // Test normal duration
        assert_eq!(subscription.parse_duration("0:03:45"), Some(225000)); // 3:45 = 225 seconds = 225000ms
        assert_eq!(subscription.parse_duration("1:23:45"), Some(5025000)); // 1:23:45 = 5025 seconds

        // Test with milliseconds
        assert_eq!(subscription.parse_duration("0:00:30.500"), Some(30500)); // 30.5 seconds

        // Test invalid format
        assert_eq!(subscription.parse_duration("invalid"), None);
        assert_eq!(subscription.parse_duration("1:23"), None); // Missing seconds
    }

    #[test]
    fn test_extract_property_value() {
        let speaker = create_test_speaker();
        let subscription = AVTransportSubscription::new(
            speaker,
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        let xml = r#"
            <property>
                <TransportState>PLAYING</TransportState>
                <CurrentTrackURI>x-sonos-spotify:spotify%3atrack%3a123</CurrentTrackURI>
            </property>
            <property>
                <Volume>50</Volume>
            </property>
        "#;

        assert_eq!(
            subscription.extract_property_value(xml, "TransportState"),
            Some("PLAYING".to_string())
        );
        assert_eq!(
            subscription.extract_property_value(xml, "Volume"),
            Some("50".to_string())
        );
        assert_eq!(
            subscription.extract_property_value(xml, "NonExistent"),
            None
        );
    }

    #[test]
    fn test_decode_xml_entities() {
        let speaker = create_test_speaker();
        let subscription = AVTransportSubscription::new(
            speaker,
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        let encoded = "Artist &amp; Band &lt;Live&gt; &quot;Greatest Hits&quot;";
        let decoded = subscription.decode_xml_entities(encoded);
        assert_eq!(decoded, "Artist & Band <Live> \"Greatest Hits\"");
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
    fn test_extract_didl_value() {
        let speaker = create_test_speaker();
        let subscription = AVTransportSubscription::new(
            speaker,
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        let didl_xml = r#"
            <DIDL-Lite>
                <item>
                    <dc:title>Test Title</dc:title>
                    <dc:creator>Test Creator</dc:creator>
                    <upnp:album>Test Album</upnp:album>
                </item>
            </DIDL-Lite>
        "#;

        assert_eq!(
            subscription.extract_didl_value(didl_xml, "dc:title"),
            Some("Test Title".to_string())
        );
        assert_eq!(
            subscription.extract_didl_value(didl_xml, "dc:creator"),
            Some("Test Creator".to_string())
        );
        assert_eq!(
            subscription.extract_didl_value(didl_xml, "upnp:album"),
            Some("Test Album".to_string())
        );
        assert_eq!(
            subscription.extract_didl_value(didl_xml, "nonexistent"),
            None
        );
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
        assert_eq!(subscription.speaker_id(), speaker.id);
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
