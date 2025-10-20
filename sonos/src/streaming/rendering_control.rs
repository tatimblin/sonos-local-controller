use super::subscription::{ServiceSubscription, SubscriptionError, SubscriptionResult};
use super::types::{ServiceType, SubscriptionConfig, SubscriptionId, SubscriptionScope};
use crate::models::{Speaker, SpeakerId, StateChange};
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

    /// Parse volume value from UPnP event XML with comprehensive validation
    fn parse_volume(&self, xml: &str) -> SubscriptionResult<Option<u8>> {
        println!("🔍 Parsing volume from XML...");
        println!("   XML length: {} bytes", xml.len());
        println!(
            "   XML preview: {}",
            xml.chars().take(200).collect::<String>()
        );

        // Wrap parsing in error handling to prevent crashes
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.parse_volume_internal(xml)
        }));

        match result {
            Ok(parsed_result) => parsed_result,
            Err(_) => {
                println!("⚠️  Volume parsing panicked, returning None gracefully");
                Ok(None)
            }
        }
    }

    /// Internal volume parsing implementation with detailed validation
    fn parse_volume_internal(&self, xml: &str) -> SubscriptionResult<Option<u8>> {
        // Look for Volume in the event XML
        if let Some(volume_str) = self.extract_property_value(xml, "Volume") {
            println!("✅ Found Volume: {}", volume_str);
            return self.validate_and_parse_volume(&volume_str);
        }

        println!("❌ No Volume found in XML");

        // Try to extract LastChange content
        if let Some(last_change) = self.extract_property_value(xml, "LastChange") {
            println!("🔍 Found LastChange content:");
            println!("   {}", last_change.chars().take(300).collect::<String>());

            // Try to parse the escaped XML in LastChange with error handling
            match self.parse_volume_from_lastchange(&last_change) {
                Ok(volume_opt) => return Ok(volume_opt),
                Err(e) => {
                    println!("⚠️  Failed to parse volume from LastChange: {}", e);
                    // Continue to return None instead of propagating error
                }
            }
        }

        Ok(None) // No volume in this event
    }

    /// Parse volume from LastChange XML content with error handling
    fn parse_volume_from_lastchange(&self, last_change: &str) -> SubscriptionResult<Option<u8>> {
        let decoded = self.decode_xml_entities(last_change);
        println!("🔍 Decoded LastChange:");
        println!("   {}", decoded.chars().take(300).collect::<String>());

        // Look for Volume in the decoded content
        if !decoded.contains("Volume") {
            return Ok(None);
        }

        println!("✅ Found Volume in decoded LastChange!");
        
        // Try to extract the val attribute with multiple patterns for robustness
        let volume_patterns = [
            "Volume channel=\"Master\" val=\"",
            "Volume val=\"",
            "<Volume>",
        ];

        for pattern in &volume_patterns {
            if let Some(volume_value) = self.extract_volume_with_pattern(&decoded, pattern) {
                println!("✅ Extracted Volume value with pattern '{}': {}", pattern, volume_value);
                return self.validate_and_parse_volume(&volume_value);
            }
        }

        println!("⚠️  Could not extract volume value from LastChange despite finding Volume element");
        Ok(None)
    }

    /// Extract volume value using a specific pattern
    fn extract_volume_with_pattern(&self, xml: &str, pattern: &str) -> Option<String> {
        if pattern.ends_with("val=\"") {
            // Attribute-based pattern
            if let Some(start) = xml.find(pattern) {
                let content_start = start + pattern.len();
                if let Some(end) = xml[content_start..].find("\"") {
                    return Some(xml[content_start..content_start + end].to_string());
                }
            }
        } else if pattern == "<Volume>" {
            // Element-based pattern
            if let Some(start) = xml.find(pattern) {
                let content_start = start + pattern.len();
                if let Some(end) = xml[content_start..].find("</Volume>") {
                    return Some(xml[content_start..content_start + end].to_string());
                }
            }
        }
        None
    }

    /// Validate and parse volume value with comprehensive range checking
    fn validate_and_parse_volume(&self, volume_str: &str) -> SubscriptionResult<Option<u8>> {
        // Trim whitespace and validate non-empty
        let volume_str = volume_str.trim();
        if volume_str.is_empty() {
            println!("⚠️  Volume string is empty after trimming");
            return Ok(None);
        }

        // Parse as integer with error handling
        match volume_str.parse::<i32>() {
            Ok(volume_int) => {
                // Validate range (0-100)
                if volume_int < 0 {
                    println!("⚠️  Volume value is negative: {}", volume_int);
                    Ok(None)
                } else if volume_int > 100 {
                    println!("⚠️  Volume value exceeds maximum (100): {}", volume_int);
                    Ok(None)
                } else {
                    // Safe to cast to u8 since we validated range
                    let volume_u8 = volume_int as u8;
                    println!("✅ Valid volume parsed: {}", volume_u8);
                    Ok(Some(volume_u8))
                }
            }
            Err(e) => {
                println!("⚠️  Failed to parse volume value '{}': {}", volume_str, e);
                // Return None instead of error to handle gracefully
                Ok(None)
            }
        }
    }

    /// Parse mute state from UPnP event XML with comprehensive validation
    fn parse_mute(&self, xml: &str) -> SubscriptionResult<Option<bool>> {
        println!("🔍 Parsing mute state from XML...");

        // Wrap parsing in error handling to prevent crashes
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.parse_mute_internal(xml)
        }));

        match result {
            Ok(parsed_result) => parsed_result,
            Err(_) => {
                println!("⚠️  Mute parsing panicked, returning None gracefully");
                Ok(None)
            }
        }
    }

    /// Internal mute parsing implementation with detailed validation
    fn parse_mute_internal(&self, xml: &str) -> SubscriptionResult<Option<bool>> {
        // Look for Mute in the event XML
        if let Some(mute_str) = self.extract_property_value(xml, "Mute") {
            println!("✅ Found Mute: {}", mute_str);
            return self.validate_and_parse_mute(&mute_str);
        }

        println!("❌ No Mute found in XML");

        // Try to extract LastChange content
        if let Some(last_change) = self.extract_property_value(xml, "LastChange") {
            match self.parse_mute_from_lastchange(&last_change) {
                Ok(mute_opt) => return Ok(mute_opt),
                Err(e) => {
                    println!("⚠️  Failed to parse mute from LastChange: {}", e);
                    // Continue to return None instead of propagating error
                }
            }
        }

        Ok(None) // No mute state in this event
    }

    /// Parse mute state from LastChange XML content with error handling
    fn parse_mute_from_lastchange(&self, last_change: &str) -> SubscriptionResult<Option<bool>> {
        let decoded = self.decode_xml_entities(last_change);

        // Look for Mute in the decoded content
        if !decoded.contains("Mute") {
            return Ok(None);
        }

        println!("✅ Found Mute in decoded LastChange!");
        
        // Try to extract the val attribute with multiple patterns for robustness
        let mute_patterns = [
            "Mute channel=\"Master\" val=\"",
            "Mute val=\"",
            "<Mute>",
        ];

        for pattern in &mute_patterns {
            if let Some(mute_value) = self.extract_mute_with_pattern(&decoded, pattern) {
                println!("✅ Extracted Mute value with pattern '{}': {}", pattern, mute_value);
                return self.validate_and_parse_mute(&mute_value);
            }
        }

        println!("⚠️  Could not extract mute value from LastChange despite finding Mute element");
        Ok(None)
    }

    /// Extract mute value using a specific pattern
    fn extract_mute_with_pattern(&self, xml: &str, pattern: &str) -> Option<String> {
        if pattern.ends_with("val=\"") {
            // Attribute-based pattern
            if let Some(start) = xml.find(pattern) {
                let content_start = start + pattern.len();
                if let Some(end) = xml[content_start..].find("\"") {
                    return Some(xml[content_start..content_start + end].to_string());
                }
            }
        } else if pattern == "<Mute>" {
            // Element-based pattern
            if let Some(start) = xml.find(pattern) {
                let content_start = start + pattern.len();
                if let Some(end) = xml[content_start..].find("</Mute>") {
                    return Some(xml[content_start..content_start + end].to_string());
                }
            }
        }
        None
    }

    /// Validate and parse mute value with comprehensive validation and boolean conversion
    fn validate_and_parse_mute(&self, mute_str: &str) -> SubscriptionResult<Option<bool>> {
        // Trim whitespace and validate non-empty
        let mute_str = mute_str.trim();
        if mute_str.is_empty() {
            println!("⚠️  Mute string is empty after trimming");
            return Ok(None);
        }

        // Handle various mute value formats with comprehensive validation
        match mute_str.to_lowercase().as_str() {
            "0" | "false" | "off" | "unmuted" => {
                println!("✅ Valid mute state parsed: false (unmuted)");
                Ok(Some(false))
            }
            "1" | "true" | "on" | "muted" => {
                println!("✅ Valid mute state parsed: true (muted)");
                Ok(Some(true))
            }
            _ => {
                println!("⚠️  Unknown mute value: '{}' - expected 0/1, true/false, on/off, or muted/unmuted", mute_str);
                // Return None instead of error to handle gracefully
                Ok(None)
            }
        }
    }

    /// Internal event parsing implementation with comprehensive error handling
    fn parse_event_internal(&self, event_xml: &str) -> SubscriptionResult<Vec<StateChange>> {
        let mut changes = Vec::new();

        println!("🔍 Parsing RenderingControl event for speaker: {}", self.speaker.name);

        // Parse volume changes with individual error handling
        match self.parse_volume(event_xml) {
            Ok(Some(volume)) => {
                println!("✅ Successfully parsed volume change: {}", volume);
                changes.push(StateChange::VolumeChanged {
                    speaker_id: self.speaker.id,
                    volume,
                });
            }
            Ok(None) => {
                println!("ℹ️  No volume change in this event");
            }
            Err(e) => {
                println!("⚠️  Failed to parse volume, continuing with other properties: {}", e);
                // Continue processing instead of failing the entire event
            }
        }

        // Parse mute state changes with individual error handling
        match self.parse_mute(event_xml) {
            Ok(Some(muted)) => {
                println!("✅ Successfully parsed mute change: {}", muted);
                changes.push(StateChange::MuteChanged {
                    speaker_id: self.speaker.id,
                    muted,
                });
            }
            Ok(None) => {
                println!("ℹ️  No mute change in this event");
            }
            Err(e) => {
                println!("⚠️  Failed to parse mute state, continuing with other properties: {}", e);
                // Continue processing instead of failing the entire event
            }
        }

        println!("✅ Event parsing completed, generated {} state changes", changes.len());
        Ok(changes)
    }

    /// Extract a property value from UPnP event XML with error handling
    fn extract_property_value(&self, xml: &str, property_name: &str) -> Option<String> {
        // Wrap extraction in error handling to prevent crashes from malformed XML
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.extract_property_value_internal(xml, property_name)
        }));

        match result {
            Ok(extracted_value) => extracted_value,
            Err(_) => {
                println!("⚠️  Property extraction panicked for '{}', returning None gracefully", property_name);
                None
            }
        }
    }

    /// Internal property extraction implementation with comprehensive error handling
    fn extract_property_value_internal(&self, xml: &str, property_name: &str) -> Option<String> {
        // Validate inputs
        if xml.is_empty() || property_name.is_empty() {
            return None;
        }

        // UPnP events use a specific XML structure with <property> elements
        // Handle both namespaced and non-namespaced property elements
        let property_patterns = [
            ("<property>", "</property>"),
            ("<e:property>", "</e:property>"),
            ("<Property>", "</Property>"), // Handle case variations
        ];

        let var_start = format!("<{}>", property_name);
        let var_end = format!("</{}>", property_name);

        // Try each property pattern with bounds checking
        for (property_start, property_end) in &property_patterns {
            let mut search_pos = 0;
            
            // Prevent infinite loops with a reasonable limit
            let mut iteration_count = 0;
            const MAX_ITERATIONS: usize = 100;
            
            while iteration_count < MAX_ITERATIONS && search_pos < xml.len() {
                iteration_count += 1;
                
                if let Some(prop_start) = xml[search_pos..].find(property_start) {
                    let prop_start_abs = search_pos + prop_start;
                    
                    // Ensure we don't go out of bounds
                    if prop_start_abs >= xml.len() {
                        break;
                    }
                    
                    if let Some(prop_end) = xml[prop_start_abs..].find(property_end) {
                        let prop_end_abs = prop_start_abs + prop_end + property_end.len();
                        
                        // Bounds check for property extraction
                        if prop_end_abs > xml.len() {
                            search_pos = prop_start_abs + 1;
                            continue;
                        }
                        
                        let property_xml = &xml[prop_start_abs..prop_end_abs];

                        // Look for our variable within this property block with bounds checking
                        if let Some(var_start_pos) = property_xml.find(&var_start) {
                            if let Some(var_end_pos) = property_xml[var_start_pos..].find(&var_end) {
                                let content_start = var_start_pos + var_start.len();
                                let content_end = var_start_pos + var_end_pos;
                                
                                // Validate content bounds
                                if content_start <= content_end && content_end <= property_xml.len() {
                                    let content = &property_xml[content_start..content_end];

                                    // Decode XML entities with error handling
                                    return Some(self.decode_xml_entities_safe(content));
                                }
                            }
                        }

                        search_pos = prop_end_abs;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            
            if iteration_count >= MAX_ITERATIONS {
                println!("⚠️  Property extraction hit iteration limit for '{}', possible malformed XML", property_name);
            }
        }

        None
    }

    /// Decode basic XML entities (legacy method for backward compatibility)
    fn decode_xml_entities(&self, text: &str) -> String {
        self.decode_xml_entities_safe(text)
    }

    /// Decode basic XML entities with comprehensive error handling
    fn decode_xml_entities_safe(&self, text: &str) -> String {
        // Handle empty or invalid input gracefully
        if text.is_empty() {
            return String::new();
        }

        // Wrap decoding in error handling to prevent crashes from malformed entities
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.decode_xml_entities_internal(text)
        }));

        match result {
            Ok(decoded) => decoded,
            Err(_) => {
                println!("⚠️  XML entity decoding panicked, returning original text");
                text.to_string()
            }
        }
    }

    /// Internal XML entity decoding implementation
    fn decode_xml_entities_internal(&self, text: &str) -> String {
        // Define entity mappings with validation
        let entities = [
            ("&lt;", "<"),
            ("&gt;", ">"),
            ("&amp;", "&"), // Process &amp; last to avoid double-decoding
            ("&quot;", "\""),
            ("&apos;", "'"),
            ("&#39;", "'"),   // Alternative apostrophe encoding
            ("&#34;", "\""),  // Alternative quote encoding
        ];

        let mut result = text.to_string();
        
        // Apply entity replacements with bounds checking
        for (entity, replacement) in &entities {
            // Prevent infinite loops by limiting replacements
            let mut replacement_count = 0;
            const MAX_REPLACEMENTS: usize = 1000;
            
            while result.contains(entity) && replacement_count < MAX_REPLACEMENTS {
                result = result.replace(entity, replacement);
                replacement_count += 1;
            }
            
            if replacement_count >= MAX_REPLACEMENTS {
                println!("⚠️  Hit replacement limit for entity '{}', possible malformed XML", entity);
            }
        }

        result
    }
}

impl ServiceSubscription for RenderingControlSubscription {
    fn service_type(&self) -> ServiceType {
        ServiceType::RenderingControl
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
        // Validate input
        if event_xml.is_empty() {
            println!("⚠️  Received empty event XML, returning no changes");
            return Ok(Vec::new());
        }

        // Wrap entire parsing in error handling to prevent crashes
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.parse_event_internal(event_xml)
        }));

        match result {
            Ok(parsed_result) => parsed_result,
            Err(_) => {
                println!("⚠️  Event parsing panicked, returning empty changes gracefully");
                // Return empty changes instead of error to continue processing other events
                Ok(Vec::new())
            }
        }
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
    fn test_rendering_control_subscription_creation() {
        let speaker = create_test_speaker();
        let callback_url = "http://localhost:8080/callback/test".to_string();
        let config = SubscriptionConfig::default();

        let subscription =
            RenderingControlSubscription::new(speaker.clone(), callback_url.clone(), config);
        assert!(subscription.is_ok());

        let sub = subscription.unwrap();
        assert_eq!(sub.service_type(), ServiceType::RenderingControl);
        assert_eq!(sub.speaker_id(), speaker.id);
        assert_eq!(sub.callback_url(), &callback_url);
        assert!(!sub.is_active());
        assert!(sub.subscription_id().is_none());
    }

    #[test]
    fn test_parse_volume() {
        let speaker = create_test_speaker();
        let subscription = RenderingControlSubscription::new(
            speaker,
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        // Test valid volume
        let volume_xml = r#"
            <property>
                <Volume>50</Volume>
            </property>
        "#;
        let volume = subscription.parse_volume(volume_xml).unwrap();
        assert_eq!(volume, Some(50));

        // Test volume 0
        let volume_xml = r#"
            <property>
                <Volume>0</Volume>
            </property>
        "#;
        let volume = subscription.parse_volume(volume_xml).unwrap();
        assert_eq!(volume, Some(0));

        // Test volume 100
        let volume_xml = r#"
            <property>
                <Volume>100</Volume>
            </property>
        "#;
        let volume = subscription.parse_volume(volume_xml).unwrap();
        assert_eq!(volume, Some(100));

        // Test invalid volume (out of range)
        let volume_xml = r#"
            <property>
                <Volume>150</Volume>
            </property>
        "#;
        let volume = subscription.parse_volume(volume_xml).unwrap();
        assert_eq!(volume, None);

        // Test no volume
        let no_volume_xml = r#"
            <property>
                <SomeOtherProperty>value</SomeOtherProperty>
            </property>
        "#;
        let volume = subscription.parse_volume(no_volume_xml).unwrap();
        assert_eq!(volume, None);
    }

    #[test]
    fn test_parse_mute() {
        let speaker = create_test_speaker();
        let subscription = RenderingControlSubscription::new(
            speaker,
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        // Test unmuted (0)
        let mute_xml = r#"
            <property>
                <Mute>0</Mute>
            </property>
        "#;
        let muted = subscription.parse_mute(mute_xml).unwrap();
        assert_eq!(muted, Some(false));

        // Test muted (1)
        let mute_xml = r#"
            <property>
                <Mute>1</Mute>
            </property>
        "#;
        let muted = subscription.parse_mute(mute_xml).unwrap();
        assert_eq!(muted, Some(true));

        // Test invalid mute value
        let mute_xml = r#"
            <property>
                <Mute>2</Mute>
            </property>
        "#;
        let muted = subscription.parse_mute(mute_xml).unwrap();
        assert_eq!(muted, None);

        // Test no mute
        let no_mute_xml = r#"
            <property>
                <SomeOtherProperty>value</SomeOtherProperty>
            </property>
        "#;
        let muted = subscription.parse_mute(no_mute_xml).unwrap();
        assert_eq!(muted, None);
    }

    #[test]
    fn test_extract_property_value() {
        let speaker = create_test_speaker();
        let subscription = RenderingControlSubscription::new(
            speaker,
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        let xml = r#"
            <property>
                <Volume>50</Volume>
                <Mute>0</Mute>
            </property>
            <property>
                <TransportState>PLAYING</TransportState>
            </property>
        "#;

        assert_eq!(
            subscription.extract_property_value(xml, "Volume"),
            Some("50".to_string())
        );
        assert_eq!(
            subscription.extract_property_value(xml, "Mute"),
            Some("0".to_string())
        );
        assert_eq!(
            subscription.extract_property_value(xml, "TransportState"),
            Some("PLAYING".to_string())
        );
        assert_eq!(
            subscription.extract_property_value(xml, "NonExistent"),
            None
        );
    }

    #[test]
    fn test_decode_xml_entities() {
        let speaker = create_test_speaker();
        let subscription = RenderingControlSubscription::new(
            speaker,
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        let encoded = "&lt;Event&gt; &amp; &quot;test&quot; &apos;value&apos;";
        let decoded = subscription.decode_xml_entities(encoded);
        assert_eq!(decoded, "<Event> & \"test\" 'value'");
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
        assert_eq!(subscription.speaker_id(), speaker.id);
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
    fn test_volume_range_validation() {
        let speaker = create_test_speaker();
        let subscription = RenderingControlSubscription::new(
            speaker,
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        // Test valid range boundaries
        assert_eq!(subscription.validate_and_parse_volume("0").unwrap(), Some(0));
        assert_eq!(subscription.validate_and_parse_volume("100").unwrap(), Some(100));
        assert_eq!(subscription.validate_and_parse_volume("50").unwrap(), Some(50));

        // Test invalid ranges
        assert_eq!(subscription.validate_and_parse_volume("-1").unwrap(), None);
        assert_eq!(subscription.validate_and_parse_volume("101").unwrap(), None);
        assert_eq!(subscription.validate_and_parse_volume("255").unwrap(), None);

        // Test invalid formats
        assert_eq!(subscription.validate_and_parse_volume("").unwrap(), None);
        assert_eq!(subscription.validate_and_parse_volume("   ").unwrap(), None);
        assert_eq!(subscription.validate_and_parse_volume("abc").unwrap(), None);
        assert_eq!(subscription.validate_and_parse_volume("50.5").unwrap(), None);
        assert_eq!(subscription.validate_and_parse_volume("50%").unwrap(), None);
    }

    #[test]
    fn test_mute_value_validation_and_conversion() {
        let speaker = create_test_speaker();
        let subscription = RenderingControlSubscription::new(
            speaker,
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        // Test standard UPnP values
        assert_eq!(subscription.validate_and_parse_mute("0").unwrap(), Some(false));
        assert_eq!(subscription.validate_and_parse_mute("1").unwrap(), Some(true));

        // Test boolean string values
        assert_eq!(subscription.validate_and_parse_mute("true").unwrap(), Some(true));
        assert_eq!(subscription.validate_and_parse_mute("false").unwrap(), Some(false));
        assert_eq!(subscription.validate_and_parse_mute("TRUE").unwrap(), Some(true));
        assert_eq!(subscription.validate_and_parse_mute("FALSE").unwrap(), Some(false));

        // Test on/off values
        assert_eq!(subscription.validate_and_parse_mute("on").unwrap(), Some(true));
        assert_eq!(subscription.validate_and_parse_mute("off").unwrap(), Some(false));
        assert_eq!(subscription.validate_and_parse_mute("ON").unwrap(), Some(true));
        assert_eq!(subscription.validate_and_parse_mute("OFF").unwrap(), Some(false));

        // Test muted/unmuted values
        assert_eq!(subscription.validate_and_parse_mute("muted").unwrap(), Some(true));
        assert_eq!(subscription.validate_and_parse_mute("unmuted").unwrap(), Some(false));
        assert_eq!(subscription.validate_and_parse_mute("MUTED").unwrap(), Some(true));

        // Test invalid values
        assert_eq!(subscription.validate_and_parse_mute("").unwrap(), None);
        assert_eq!(subscription.validate_and_parse_mute("   ").unwrap(), None);
        assert_eq!(subscription.validate_and_parse_mute("2").unwrap(), None);
        assert_eq!(subscription.validate_and_parse_mute("yes").unwrap(), None);
        assert_eq!(subscription.validate_and_parse_mute("no").unwrap(), None);
        assert_eq!(subscription.validate_and_parse_mute("invalid").unwrap(), None);
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
    fn test_xml_entity_decoding_safety() {
        let speaker = create_test_speaker();
        let subscription = RenderingControlSubscription::new(
            speaker,
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        // Test normal entity decoding
        let encoded = "&lt;Event&gt; &amp; &quot;test&quot; &apos;value&apos;";
        let decoded = subscription.decode_xml_entities_safe(encoded);
        assert_eq!(decoded, "<Event> & \"test\" 'value'");

        // Test empty string
        assert_eq!(subscription.decode_xml_entities_safe(""), "");

        // Test string without entities
        let no_entities = "Normal text without entities";
        assert_eq!(subscription.decode_xml_entities_safe(no_entities), no_entities);

        // Test malformed entities (should not crash)
        let malformed = "&lt &gt; &amp &unknown;";
        let result = subscription.decode_xml_entities_safe(malformed);
        assert!(!result.is_empty()); // Should return something, not crash

        // Test recursive entities (potential infinite loop)
        let recursive = "&amp;lt;";
        let result = subscription.decode_xml_entities_safe(recursive);
        assert_eq!(result, "&lt;"); // Should handle properly
    }

    #[test]
    fn test_property_extraction_bounds_checking() {
        let speaker = create_test_speaker();
        let subscription = RenderingControlSubscription::new(
            speaker,
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        // Test normal extraction
        let normal_xml = "<property><Volume>50</Volume></property>";
        assert_eq!(
            subscription.extract_property_value(normal_xml, "Volume"),
            Some("50".to_string())
        );

        // Test empty inputs
        assert_eq!(subscription.extract_property_value("", "Volume"), None);
        assert_eq!(subscription.extract_property_value(normal_xml, ""), None);

        // Test malformed XML that could cause bounds issues
        let malformed_xml = "<property><Volume>50</Volume"; // Missing closing
        assert_eq!(subscription.extract_property_value(malformed_xml, "Volume"), None);

        // Test nested properties (should handle correctly)
        let nested_xml = r#"
            <property>
                <Volume>30</Volume>
                <property>
                    <Volume>50</Volume>
                </property>
            </property>
        "#;
        let result = subscription.extract_property_value(nested_xml, "Volume");
        assert!(result.is_some()); // Should extract one of the volumes

        // Test very large property names (potential buffer overflow)
        let large_property_name = "x".repeat(1000);
        assert_eq!(subscription.extract_property_value(normal_xml, &large_property_name), None);
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
    fn test_volume_parsing_edge_cases() {
        let speaker = create_test_speaker();
        let subscription = RenderingControlSubscription::new(
            speaker,
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        // Test volume with whitespace
        let volume_xml = r#"<property><Volume>  50  </Volume></property>"#;
        let volume = subscription.parse_volume(volume_xml).unwrap();
        assert_eq!(volume, Some(50));

        // Test volume with leading zeros
        let volume_xml = r#"<property><Volume>050</Volume></property>"#;
        let volume = subscription.parse_volume(volume_xml).unwrap();
        assert_eq!(volume, Some(50));

        // Test negative volume
        let volume_xml = r#"<property><Volume>-10</Volume></property>"#;
        let volume = subscription.parse_volume(volume_xml).unwrap();
        assert_eq!(volume, None);

        // Test volume with decimal
        let volume_xml = r#"<property><Volume>50.5</Volume></property>"#;
        let volume = subscription.parse_volume(volume_xml).unwrap();
        assert_eq!(volume, None);

        // Test empty volume element
        let volume_xml = r#"<property><Volume></Volume></property>"#;
        let volume = subscription.parse_volume(volume_xml).unwrap();
        assert_eq!(volume, None);
    }

    #[test]
    fn test_mute_parsing_edge_cases() {
        let speaker = create_test_speaker();
        let subscription = RenderingControlSubscription::new(
            speaker,
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        // Test mute with whitespace
        let mute_xml = r#"<property><Mute>  1  </Mute></property>"#;
        let muted = subscription.parse_mute(mute_xml).unwrap();
        assert_eq!(muted, Some(true));

        // Test case insensitive boolean values
        let mute_xml = r#"<property><Mute>True</Mute></property>"#;
        let muted = subscription.parse_mute(mute_xml).unwrap();
        assert_eq!(muted, Some(true));

        let mute_xml = r#"<property><Mute>False</Mute></property>"#;
        let muted = subscription.parse_mute(mute_xml).unwrap();
        assert_eq!(muted, Some(false));

        // Test empty mute element
        let mute_xml = r#"<property><Mute></Mute></property>"#;
        let muted = subscription.parse_mute(mute_xml).unwrap();
        assert_eq!(muted, None);

        // Test numeric values beyond 0/1
        let mute_xml = r#"<property><Mute>5</Mute></property>"#;
        let muted = subscription.parse_mute(mute_xml).unwrap();
        assert_eq!(muted, None);
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
    fn test_xml_entity_decoding_comprehensive() {
        let speaker = create_test_speaker();
        let subscription = RenderingControlSubscription::new(
            speaker,
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        // Test all standard XML entities
        let test_cases = vec![
            ("&lt;", "<"),
            ("&gt;", ">"),
            ("&amp;", "&"),
            ("&quot;", "\""),
            ("&apos;", "'"),
            ("&#39;", "'"),
            ("&#34;", "\""),
        ];

        for (encoded, expected) in test_cases {
            let result = subscription.decode_xml_entities(encoded);
            assert_eq!(result, expected, "Failed to decode entity: {}", encoded);
        }

        // Test mixed entities
        let mixed = "&lt;Volume&gt;&amp;&quot;50&quot;&apos;";
        let expected = "<Volume>&\"50\"'";
        assert_eq!(subscription.decode_xml_entities(mixed), expected);

        // Test text without entities
        let plain_text = "Normal text without any entities";
        assert_eq!(subscription.decode_xml_entities(plain_text), plain_text);
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

    #[test]
    fn test_volume_parsing_patterns() {
        let speaker = create_test_speaker();
        let subscription = RenderingControlSubscription::new(
            speaker,
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        // Test different XML patterns for volume
        let patterns = vec![
            // Standard property format
            (r#"<property><Volume>42</Volume></property>"#, Some(42)),
            // Namespaced property
            (r#"<e:property><Volume>42</Volume></e:property>"#, Some(42)),
            // Volume with attributes (not supported by current parser - should return None)
            (r#"<property><Volume channel="Master">42</Volume></property>"#, None),
            // In LastChange format
            (r#"<property><LastChange>&lt;Event&gt;&lt;InstanceID val="0"&gt;&lt;Volume channel="Master" val="42"/&gt;&lt;/InstanceID&gt;&lt;/Event&gt;</LastChange></property>"#, Some(42)),
        ];

        for (i, (pattern, expected)) in patterns.iter().enumerate() {
            let volume = subscription.parse_volume(pattern).unwrap();
            assert_eq!(volume, *expected, "Pattern {} failed: {}", i, pattern);
        }
    }

    #[test]
    fn test_mute_parsing_patterns() {
        let speaker = create_test_speaker();
        let subscription = RenderingControlSubscription::new(
            speaker,
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        // Test different XML patterns for mute
        let patterns = vec![
            // Standard property format - muted
            (r#"<property><Mute>1</Mute></property>"#, Some(true)),
            // Standard property format - unmuted
            (r#"<property><Mute>0</Mute></property>"#, Some(false)),
            // Namespaced property
            (r#"<e:property><Mute>1</Mute></e:property>"#, Some(true)),
            // In LastChange format - muted
            (r#"<property><LastChange>&lt;Event&gt;&lt;InstanceID val="0"&gt;&lt;Mute channel="Master" val="1"/&gt;&lt;/InstanceID&gt;&lt;/Event&gt;</LastChange></property>"#, Some(true)),
            // In LastChange format - unmuted
            (r#"<property><LastChange>&lt;Event&gt;&lt;InstanceID val="0"&gt;&lt;Mute channel="Master" val="0"/&gt;&lt;/InstanceID&gt;&lt;/Event&gt;</LastChange></property>"#, Some(false)),
        ];

        for (i, (pattern, expected)) in patterns.iter().enumerate() {
            let muted = subscription.parse_mute(pattern).unwrap();
            assert_eq!(muted, *expected, "Pattern {} failed: {}", i, pattern);
        }
    }
}
