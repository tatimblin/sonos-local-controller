use super::subscription::{ServiceSubscription, SubscriptionError, SubscriptionResult};
use super::types::{ServiceType, SubscriptionConfig, SubscriptionId, SubscriptionScope};
use crate::models::{Group, GroupId, Speaker, SpeakerId, StateChange};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::SystemTime;

/// ZoneGroupTopology service subscription implementation
///
/// This struct handles UPnP subscriptions to the ZoneGroupTopology service on Sonos devices,
/// which provides events for zone group changes, speaker grouping/ungrouping, and topology
/// updates across all Sonos devices in the network. Unlike per-speaker services, this is a
/// network-wide service that only requires one subscription per network.
pub struct ZoneGroupTopologySubscription {
    /// Representative speaker for this network (used for subscription endpoint)
    representative_speaker: Speaker,
    /// All speakers in the network (for event distribution)
    network_speakers: Vec<Speaker>,
    /// Current subscription ID (None if not subscribed)
    subscription_id: Option<SubscriptionId>,
    /// UPnP SID (Subscription ID) returned by the device
    upnp_sid: Option<String>,
    /// URL where the device should send event notifications
    callback_url: String,
    /// Configuration for this subscription
    config: SubscriptionConfig,
    /// Whether the subscription is currently active
    active: bool,
    /// Timestamp of the last successful renewal
    last_renewal: Option<SystemTime>,
    /// Last known zone group state for change detection (using Mutex for thread-safe interior mutability)
    last_zone_groups: Mutex<Option<Vec<Group>>>,
}

impl ZoneGroupTopologySubscription {
    /// Create a new ZoneGroupTopology subscription
    ///
    /// # Arguments
    /// * `representative_speaker` - The speaker to use for the subscription endpoint
    /// * `network_speakers` - All speakers in the network for event distribution
    /// * `callback_url` - URL where the device should send event notifications
    /// * `config` - Configuration for this subscription
    pub fn new(
        representative_speaker: Speaker,
        network_speakers: Vec<Speaker>,
        callback_url: String,
        config: SubscriptionConfig,
    ) -> SubscriptionResult<Self> {
        Ok(Self {
            representative_speaker,
            network_speakers,
            subscription_id: None,
            upnp_sid: None,
            callback_url,
            config,
            active: false,
            last_renewal: None,
            last_zone_groups: Mutex::new(None),
        })
    }

    /// Update the list of network speakers
    pub fn update_network_speakers(&mut self, speakers: Vec<Speaker>) {
        self.network_speakers = speakers;
    }

    /// Get the device URL for the representative speaker
    fn device_url(&self) -> String {
        format!(
            "http://{}:{}",
            self.representative_speaker.ip_address, self.representative_speaker.port
        )
    }

    /// Send a UPnP SUBSCRIBE request to establish the subscription
    fn send_subscribe_request(&self) -> SubscriptionResult<String> {
        let device_url = self.device_url();
        let event_sub_url = ServiceType::ZoneGroupTopology.event_sub_url();
        let full_url = format!("{}{}", device_url, event_sub_url);

        println!(
            "📡 Sending ZoneGroupTopology SUBSCRIBE request to: {}",
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
                format!(
                    "{}:{}",
                    self.representative_speaker.ip_address, self.representative_speaker.port
                ),
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
        let event_sub_url = ServiceType::ZoneGroupTopology.event_sub_url();
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
                format!(
                    "{}:{}",
                    self.representative_speaker.ip_address, self.representative_speaker.port
                ),
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
        let event_sub_url = ServiceType::ZoneGroupTopology.event_sub_url();
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
                format!(
                    "{}:{}",
                    self.representative_speaker.ip_address, self.representative_speaker.port
                ),
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

    /// Parse ZoneGroupTopology XML and extract group information
    pub fn parse_zone_group_state(&self, xml: &str) -> SubscriptionResult<Vec<Group>> {
        println!("🔍 Parsing ZoneGroupTopology XML...");
        println!("   XML length: {} bytes", xml.len());
        
        if xml.is_empty() {
            return Err(SubscriptionError::EventParseError(
                "Empty XML content".to_string(),
            ));
        }

        println!(
            "   XML preview: {}",
            xml.chars().take(200).collect::<String>()
        );

        let mut groups = Vec::new();

        // Look for ZoneGroupState in the event XML
        match self.extract_property_value(xml, "ZoneGroupState") {
            Some(zone_group_state_xml) => {
                println!("✅ Found ZoneGroupState content");

                if zone_group_state_xml.trim().is_empty() {
                    println!("⚠️ ZoneGroupState content is empty");
                    return Ok(groups);
                }

                // Decode XML entities in the ZoneGroupState content
                let decoded_xml = self.decode_xml_entities(&zone_group_state_xml);
                println!("🔍 Decoded ZoneGroupState:");
                println!("   {}", decoded_xml.chars().take(300).collect::<String>());

                // Parse each ZoneGroup element with error handling
                groups = self.parse_zone_groups(&decoded_xml).map_err(|e| {
                    SubscriptionError::EventParseError(format!(
                        "Failed to parse ZoneGroups: {}",
                        e
                    ))
                })?;
            }
            None => {
                println!("❌ No ZoneGroupState found in XML");
                // This might be a different type of event or malformed XML
                // Check if this is a valid UPnP event structure
                if !xml.contains("<property>") && !xml.contains("<e:property>") {
                    return Err(SubscriptionError::EventParseError(
                        "Invalid UPnP event structure - no property elements found".to_string(),
                    ));
                }
                // If it's a valid UPnP event but no ZoneGroupState, return empty groups
                println!("ℹ️ Valid UPnP event but no ZoneGroupState property");
            }
        }

        println!("✅ Successfully parsed {} groups", groups.len());
        Ok(groups)
    }

    /// Parse ZoneGroup elements from the decoded XML
    fn parse_zone_groups(&self, xml: &str) -> SubscriptionResult<Vec<Group>> {
        let mut groups = Vec::new();
        let mut search_pos = 0;
        let mut group_count = 0;

        // Handle empty ZoneGroups case
        if !xml.contains("<ZoneGroup") {
            println!("ℹ️ No ZoneGroup elements found in XML");
            return Ok(groups);
        }

        // Find all ZoneGroup elements
        while let Some(zone_group_start) = xml[search_pos..].find("<ZoneGroup") {
            let zone_group_start_abs = search_pos + zone_group_start;
            group_count += 1;

            // Prevent infinite loops with a reasonable limit
            if group_count > 100 {
                return Err(SubscriptionError::EventParseError(
                    "Too many ZoneGroup elements found (>100), possible malformed XML".to_string(),
                ));
            }

            // Find the end of this ZoneGroup element
            if let Some(zone_group_end) = xml[zone_group_start_abs..].find("</ZoneGroup>") {
                let zone_group_end_abs =
                    zone_group_start_abs + zone_group_end + "</ZoneGroup>".len();
                let zone_group_xml = &xml[zone_group_start_abs..zone_group_end_abs];

                // Parse this individual ZoneGroup with error handling
                match self.parse_single_zone_group(zone_group_xml) {
                    Ok(Some(group)) => {
                        groups.push(group);
                        println!("✅ Successfully parsed group {}", group_count);
                    }
                    Ok(None) => {
                        println!("⚠️ Skipped empty or invalid group {}", group_count);
                    }
                    Err(e) => {
                        println!("❌ Failed to parse group {}: {}", group_count, e);
                        // Continue parsing other groups instead of failing completely
                        // This makes the system more resilient to partial XML corruption
                    }
                }

                search_pos = zone_group_end_abs;
            } else {
                // Check if this is a self-closing ZoneGroup tag
                if let Some(self_close_end) = xml[zone_group_start_abs..].find("/>") {
                    let zone_group_end_abs = zone_group_start_abs + self_close_end + 2;
                    let zone_group_xml = &xml[zone_group_start_abs..zone_group_end_abs];

                    // Parse this self-closing ZoneGroup
                    match self.parse_single_zone_group(zone_group_xml) {
                        Ok(Some(group)) => {
                            groups.push(group);
                            println!("✅ Successfully parsed self-closing group {}", group_count);
                        }
                        Ok(None) => {
                            println!("⚠️ Skipped empty self-closing group {}", group_count);
                        }
                        Err(e) => {
                            println!("❌ Failed to parse self-closing group {}: {}", group_count, e);
                        }
                    }

                    search_pos = zone_group_end_abs;
                } else {
                    // Malformed XML - missing closing tag
                    println!("❌ Malformed ZoneGroup XML - missing closing tag at position {}", zone_group_start_abs);
                    return Err(SubscriptionError::EventParseError(
                        "Malformed ZoneGroup XML - missing closing tag".to_string(),
                    ));
                }
            }
        }

        println!("✅ Parsed {} zone groups from {} found", groups.len(), group_count);
        Ok(groups)
    }

    /// Parse a single ZoneGroup element
    fn parse_single_zone_group(&self, zone_group_xml: &str) -> SubscriptionResult<Option<Group>> {
        if zone_group_xml.trim().is_empty() {
            return Ok(None);
        }

        // Extract coordinator from ZoneGroup attributes
        let coordinator_uuid = self.extract_attribute(zone_group_xml, "Coordinator")
            .map_err(|e| {
                SubscriptionError::EventParseError(format!(
                    "Failed to extract Coordinator attribute: {}",
                    e
                ))
            })?;

        if coordinator_uuid.trim().is_empty() {
            return Err(SubscriptionError::EventParseError(
                "Empty Coordinator UUID found".to_string(),
            ));
        }

        let coordinator_id = SpeakerId::from_udn(&format!("uuid:{}", coordinator_uuid));

        println!(
            "🔍 Parsing ZoneGroup with coordinator: {}",
            coordinator_uuid
        );

        let mut group = Group::new(coordinator_id);
        let mut search_pos = 0;
        let mut member_count = 0;

        // Find all ZoneGroupMember elements
        while let Some(member_start) = zone_group_xml[search_pos..].find("<ZoneGroupMember") {
            let member_start_abs = search_pos + member_start;
            member_count += 1;

            // Prevent infinite loops
            if member_count > 50 {
                return Err(SubscriptionError::EventParseError(
                    "Too many ZoneGroupMember elements (>50), possible malformed XML".to_string(),
                ));
            }

            // Find the end of this member element (self-closing tag)
            if let Some(member_end) = zone_group_xml[member_start_abs..].find("/>") {
                let member_end_abs = member_start_abs + member_end + 2;
                let member_xml = &zone_group_xml[member_start_abs..member_end_abs];

                // Parse this member with error handling
                match self.parse_zone_group_member(member_xml) {
                    Ok(Some((speaker_id, satellites))) => {
                        group.add_member_with_satellites(speaker_id, satellites);
                        println!("✅ Added member {:?} to group", speaker_id);
                    }
                    Ok(None) => {
                        println!("⚠️ Skipped invalid member {}", member_count);
                    }
                    Err(e) => {
                        println!("❌ Failed to parse member {}: {}", member_count, e);
                        // Continue with other members instead of failing completely
                    }
                }

                search_pos = member_end_abs;
            } else {
                // Look for non-self-closing member tags
                if let Some(member_close) = zone_group_xml[member_start_abs..].find("</ZoneGroupMember>") {
                    let member_end_abs = member_start_abs + member_close + "</ZoneGroupMember>".len();
                    let member_xml = &zone_group_xml[member_start_abs..member_end_abs];

                    // Parse this member
                    match self.parse_zone_group_member(member_xml) {
                        Ok(Some((speaker_id, satellites))) => {
                            group.add_member_with_satellites(speaker_id, satellites);
                            println!("✅ Added member {:?} to group", speaker_id);
                        }
                        Ok(None) => {
                            println!("⚠️ Skipped invalid member {}", member_count);
                        }
                        Err(e) => {
                            println!("❌ Failed to parse member {}: {}", member_count, e);
                        }
                    }

                    search_pos = member_end_abs;
                } else {
                    println!("❌ Malformed ZoneGroupMember XML at position {}", member_start_abs);
                    break;
                }
            }
        }

        if group.member_count() == 0 {
            println!("⚠️ ZoneGroup has no valid members, skipping");
            return Ok(None);
        }

        println!("✅ Successfully parsed ZoneGroup with {} members", group.member_count());
        Ok(Some(group))
    }

    /// Parse a single ZoneGroupMember element
    fn parse_zone_group_member(
        &self,
        member_xml: &str,
    ) -> SubscriptionResult<Option<(SpeakerId, Vec<SpeakerId>)>> {
        if member_xml.trim().is_empty() {
            return Ok(None);
        }

        // Extract UUID from member attributes
        let uuid = self.extract_attribute(member_xml, "UUID")
            .map_err(|e| {
                SubscriptionError::EventParseError(format!(
                    "Failed to extract UUID from ZoneGroupMember: {}",
                    e
                ))
            })?;

        if uuid.trim().is_empty() {
            return Err(SubscriptionError::EventParseError(
                "Empty UUID found in ZoneGroupMember".to_string(),
            ));
        }

        let speaker_id = SpeakerId::from_udn(&format!("uuid:{}", uuid));

        // Parse satellites if present in the member XML
        // Satellites might be encoded in various ways in the ZoneGroupTopology format
        let satellites = self.parse_member_satellites(member_xml)?;

        if satellites.is_empty() {
            println!("🔍 Parsed member: {} (no satellites)", uuid);
        } else {
            println!("🔍 Parsed member: {} with {} satellites", uuid, satellites.len());
        }

        Ok(Some((speaker_id, satellites)))
    }

    /// Parse satellite speakers from a ZoneGroupMember element
    pub fn parse_member_satellites(&self, member_xml: &str) -> SubscriptionResult<Vec<SpeakerId>> {
        let mut satellites = Vec::new();

        // Look for satellite information in various possible formats
        // Format 1: Satellites attribute (comma-separated UUIDs)
        if let Ok(satellites_attr) = self.extract_attribute(member_xml, "Satellites") {
            if !satellites_attr.trim().is_empty() {
                for satellite_uuid in satellites_attr.split(',') {
                    let uuid = satellite_uuid.trim();
                    if !uuid.is_empty() {
                        let satellite_id = SpeakerId::from_udn(&format!("uuid:{}", uuid));
                        satellites.push(satellite_id);
                        println!("🔍 Found satellite: {}", uuid);
                    }
                }
            }
        }

        // Format 2: Look for nested satellite elements
        let mut search_pos = 0;
        while let Some(satellite_start) = member_xml[search_pos..].find("<Satellite") {
            let satellite_start_abs = search_pos + satellite_start;
            
            // Check for self-closing satellite tags
            if let Some(satellite_end) = member_xml[satellite_start_abs..].find("/>") {
                let satellite_end_abs = satellite_start_abs + satellite_end + 2;
                let satellite_xml = &member_xml[satellite_start_abs..satellite_end_abs];
                
                if let Ok(satellite_uuid) = self.extract_attribute(satellite_xml, "UUID") {
                    if !satellite_uuid.trim().is_empty() {
                        let satellite_id = SpeakerId::from_udn(&format!("uuid:{}", satellite_uuid));
                        satellites.push(satellite_id);
                        println!("🔍 Found nested satellite: {}", satellite_uuid);
                    }
                }
                
                search_pos = satellite_end_abs;
            } else if let Some(satellite_close) = member_xml[satellite_start_abs..].find("</Satellite>") {
                // Handle non-self-closing satellite tags
                let satellite_end_abs = satellite_start_abs + satellite_close + "</Satellite>".len();
                let satellite_xml = &member_xml[satellite_start_abs..satellite_end_abs];
                
                if let Ok(satellite_uuid) = self.extract_attribute(satellite_xml, "UUID") {
                    if !satellite_uuid.trim().is_empty() {
                        let satellite_id = SpeakerId::from_udn(&format!("uuid:{}", satellite_uuid));
                        satellites.push(satellite_id);
                        println!("🔍 Found nested satellite: {}", satellite_uuid);
                    }
                }
                
                search_pos = satellite_end_abs;
            } else {
                break;
            }
        }

        Ok(satellites)
    }

    /// Extract an attribute value from an XML element
    pub fn extract_attribute(&self, xml: &str, attr_name: &str) -> SubscriptionResult<String> {
        if xml.trim().is_empty() {
            return Err(SubscriptionError::EventParseError(
                "Empty XML provided for attribute extraction".to_string(),
            ));
        }

        if attr_name.trim().is_empty() {
            return Err(SubscriptionError::EventParseError(
                "Empty attribute name provided".to_string(),
            ));
        }

        // Try different quote styles (double and single quotes)
        let patterns = [
            format!("{}=\"", attr_name),
            format!("{}='", attr_name),
            format!("{} = \"", attr_name),
            format!("{} = '", attr_name),
        ];

        for pattern in &patterns {
            if let Some(attr_start) = xml.find(pattern) {
                let value_start = attr_start + pattern.len();
                let quote_char = if pattern.contains('"') { '"' } else { '\'' };
                
                if let Some(value_end) = xml[value_start..].find(quote_char) {
                    let value = &xml[value_start..value_start + value_end];
                    
                    // Decode basic XML entities in attribute values
                    let decoded_value = self.decode_xml_entities(value);
                    return Ok(decoded_value);
                }
            }
        }

        Err(SubscriptionError::EventParseError(format!(
            "Missing required attribute '{}' in XML: {}",
            attr_name,
            xml.chars().take(100).collect::<String>()
        )))
    }

    /// Extract a property value from UPnP event XML
    pub fn extract_property_value(&self, xml: &str, property_name: &str) -> Option<String> {
        if xml.trim().is_empty() || property_name.trim().is_empty() {
            return None;
        }

        // UPnP events use a specific XML structure with <property> elements
        let property_patterns = [
            ("<property>", "</property>"),
            ("<e:property>", "</e:property>"),
            ("<s:property>", "</s:property>"), // Some devices use 's:' namespace
        ];

        let var_patterns = [
            (format!("<{}>", property_name), format!("</{}>", property_name)),
        ];

        // Try each property pattern
        for (property_start, property_end) in &property_patterns {
            let mut search_pos = 0;
            while let Some(prop_start) = xml[search_pos..].find(property_start) {
                let prop_start_abs = search_pos + prop_start;
                if let Some(prop_end) = xml[prop_start_abs..].find(property_end) {
                    let prop_end_abs = prop_start_abs + prop_end + property_end.len();
                    let property_xml = &xml[prop_start_abs..prop_end_abs];

                    // Try each variable pattern within this property block
                    for (var_start, var_end) in &var_patterns {
                        if let Some(var_start_pos) = property_xml.find(var_start) {
                            if let Some(var_end_pos) = property_xml[var_start_pos..].find(var_end) {
                                let content_start = var_start_pos + var_start.len();
                                let content_end = var_start_pos + var_end_pos;
                                
                                if content_end > content_start {
                                    let content = &property_xml[content_start..content_end];
                                    
                                    // Decode XML entities and return
                                    let decoded = self.decode_xml_entities(content);
                                    if !decoded.trim().is_empty() {
                                        return Some(decoded);
                                    }
                                }
                            }
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

    /// Decode XML entities and handle CDATA sections
    pub fn decode_xml_entities(&self, text: &str) -> String {
        let mut result = text.to_string();
        
        // Handle CDATA sections
        while let Some(cdata_start) = result.find("<![CDATA[") {
            if let Some(cdata_end) = result[cdata_start..].find("]]>") {
                let cdata_end_abs = cdata_start + cdata_end;
                let cdata_content = &result[cdata_start + 9..cdata_end_abs];
                let before = &result[..cdata_start];
                let after = &result[cdata_end_abs + 3..];
                result = format!("{}{}{}", before, cdata_content, after);
            } else {
                break;
            }
        }
        
        // Decode standard XML entities
        result = result
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&#39;", "'")
            .replace("&#34;", "\"")
            .replace("&#60;", "<")
            .replace("&#62;", ">")
            .replace("&#38;", "&");
            
        result
    }

    /// Detect changes between old and new zone group states
    fn detect_topology_changes(&self, new_groups: &[Group]) -> Vec<StateChange> {
        let mut changes = Vec::new();

        match &*self.last_zone_groups.lock().unwrap() {
            None => {
                // First time receiving topology - generate initial events
                println!("🔍 Initial topology received with {} groups", new_groups.len());
                
                // Generate GroupFormed events for all initial groups
                for group in new_groups {
                    changes.push(StateChange::GroupFormed {
                        group_id: group.id,
                        coordinator_id: group.coordinator,
                        initial_members: group.all_speaker_ids(),
                    });
                }
            }
            Some(old_groups) => {
                // Compare old and new states to detect specific changes
                let (speakers_joined, speakers_left, coordinator_changes) = 
                    self.analyze_topology_changes(old_groups, new_groups);

                // Generate specific change events
                for (speaker_id, group_id) in &speakers_joined {
                    let coordinator_id = new_groups
                        .iter()
                        .find(|g| g.id == *group_id)
                        .map(|g| g.coordinator)
                        .unwrap_or(*speaker_id);
                    
                    changes.push(StateChange::SpeakerJoinedGroup {
                        speaker_id: *speaker_id,
                        group_id: *group_id,
                        coordinator_id,
                    });
                }

                for (speaker_id, former_group_id) in &speakers_left {
                    if let Some(group_id) = former_group_id {
                        changes.push(StateChange::SpeakerLeftGroup {
                            speaker_id: *speaker_id,
                            former_group_id: *group_id,
                        });
                    }
                }

                for (group_id, old_coordinator, new_coordinator) in &coordinator_changes {
                    changes.push(StateChange::CoordinatorChanged {
                        group_id: *group_id,
                        old_coordinator: *old_coordinator,
                        new_coordinator: *new_coordinator,
                    });
                }

                // Detect newly formed groups
                for group in new_groups {
                    if !old_groups.iter().any(|old_group| old_group.id == group.id) {
                        changes.push(StateChange::GroupFormed {
                            group_id: group.id,
                            coordinator_id: group.coordinator,
                            initial_members: group.all_speaker_ids(),
                        });
                    }
                }

                // Detect dissolved groups
                for old_group in old_groups {
                    if !new_groups.iter().any(|group| group.id == old_group.id) {
                        changes.push(StateChange::GroupDissolved {
                            group_id: old_group.id,
                            former_coordinator: old_group.coordinator,
                            former_members: old_group.all_speaker_ids(),
                        });
                    }
                }

                // Always include the comprehensive topology change event
                changes.push(StateChange::GroupTopologyChanged {
                    groups: new_groups.to_vec(),
                    speakers_joined: speakers_joined.clone(),
                    speakers_left: speakers_left.clone(),
                    coordinator_changes: coordinator_changes.clone(),
                });
            }
        }

        // Update the stored state for next comparison
        *self.last_zone_groups.lock().unwrap() = Some(new_groups.to_vec());

        println!("✅ Generated {} topology change events", changes.len());
        changes
    }

    /// Analyze detailed changes between old and new topology states
    fn analyze_topology_changes(
        &self,
        old_groups: &[Group],
        new_groups: &[Group],
    ) -> (
        Vec<(SpeakerId, GroupId)>, // speakers_joined
        Vec<(SpeakerId, Option<GroupId>)>, // speakers_left  
        Vec<(GroupId, SpeakerId, SpeakerId)>, // coordinator_changes
    ) {
        let mut speakers_joined = Vec::new();
        let mut speakers_left = Vec::new();
        let mut coordinator_changes = Vec::new();

        // Build maps for easier lookup
        let old_speaker_to_group: HashMap<SpeakerId, GroupId> = old_groups
            .iter()
            .flat_map(|group| {
                group.all_speaker_ids().into_iter().map(move |speaker_id| (speaker_id, group.id))
            })
            .collect();

        let new_speaker_to_group: HashMap<SpeakerId, GroupId> = new_groups
            .iter()
            .flat_map(|group| {
                group.all_speaker_ids().into_iter().map(move |speaker_id| (speaker_id, group.id))
            })
            .collect();

        // Find speakers that joined groups
        for (speaker_id, new_group_id) in &new_speaker_to_group {
            match old_speaker_to_group.get(speaker_id) {
                None => {
                    // Speaker wasn't in any group before, now it is
                    speakers_joined.push((*speaker_id, *new_group_id));
                }
                Some(old_group_id) if old_group_id != new_group_id => {
                    // Speaker moved from one group to another
                    speakers_left.push((*speaker_id, Some(*old_group_id)));
                    speakers_joined.push((*speaker_id, *new_group_id));
                }
                _ => {
                    // Speaker remained in the same group
                }
            }
        }

        // Find speakers that left groups (and didn't join another)
        for (speaker_id, old_group_id) in &old_speaker_to_group {
            if !new_speaker_to_group.contains_key(speaker_id) {
                speakers_left.push((*speaker_id, Some(*old_group_id)));
            }
        }

        // Find coordinator changes within existing groups
        for new_group in new_groups {
            if let Some(old_group) = old_groups.iter().find(|g| g.id == new_group.id) {
                if old_group.coordinator != new_group.coordinator {
                    coordinator_changes.push((
                        new_group.id,
                        old_group.coordinator,
                        new_group.coordinator,
                    ));
                }
            }
        }

        (speakers_joined, speakers_left, coordinator_changes)
    }
}

impl ServiceSubscription for ZoneGroupTopologySubscription {
    fn service_type(&self) -> ServiceType {
        ServiceType::ZoneGroupTopology
    }

    fn subscription_scope(&self) -> SubscriptionScope {
        SubscriptionScope::NetworkWide
    }

    fn speaker_id(&self) -> SpeakerId {
        // Return the representative speaker's ID
        self.representative_speaker.id
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

        println!(
            "✅ ZoneGroupTopology subscription established with ID: {}",
            subscription_id
        );
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
        *self.last_zone_groups.lock().unwrap() = None;

        println!("✅ ZoneGroupTopology subscription terminated");
        Ok(())
    }

    fn renew(&mut self) -> SubscriptionResult<()> {
        if !self.active {
            return Err(SubscriptionError::SubscriptionExpired);
        }

        if let Some(upnp_sid) = &self.upnp_sid {
            self.send_renewal_request(upnp_sid)?;
            self.last_renewal = Some(SystemTime::now());
            println!("✅ ZoneGroupTopology subscription renewed");
            Ok(())
        } else {
            Err(SubscriptionError::SubscriptionExpired)
        }
    }

    fn parse_event(&self, event_xml: &str) -> SubscriptionResult<Vec<StateChange>> {
        println!("🔍 Parsing ZoneGroupTopology event...");

        // Parse the zone group state from the event with comprehensive error handling
        let new_groups = match self.parse_zone_group_state(event_xml) {
            Ok(groups) => groups,
            Err(e) => {
                println!("❌ Failed to parse ZoneGroupTopology XML: {}", e);
                // Log the error but don't fail completely - return empty changes
                // This allows the system to continue processing other events
                return Ok(vec![StateChange::SubscriptionError {
                    speaker_id: self.representative_speaker.id,
                    service: ServiceType::ZoneGroupTopology,
                    error: format!("XML parsing failed: {}", e),
                }]);
            }
        };

        // Detect changes and generate appropriate StateChange events
        let changes = self.detect_topology_changes(&new_groups);

        println!(
            "✅ Generated {} state changes from ZoneGroupTopology event",
            changes.len()
        );
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
            *self.last_zone_groups.lock().unwrap() = None;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Speaker;

    fn create_test_speaker(id_suffix: &str, ip: &str) -> Speaker {
        Speaker {
            id: SpeakerId::from_udn(&format!("uuid:RINCON_{}::1", id_suffix)),
            udn: format!("uuid:RINCON_{}::1", id_suffix),
            name: format!("Test Speaker {}", id_suffix),
            room_name: format!("Test Room {}", id_suffix),
            ip_address: ip.to_string(),
            port: 1400,
            model_name: "Test Model".to_string(),
            satellites: vec![],
        }
    }

    #[test]
    fn test_zone_group_topology_subscription_creation() {
        let representative_speaker = create_test_speaker("123456789", "192.168.1.100");
        let network_speakers = vec![
            representative_speaker.clone(),
            create_test_speaker("987654321", "192.168.1.101"),
        ];
        let callback_url = "http://localhost:8080/callback/test".to_string();
        let config = SubscriptionConfig::default();

        let subscription = ZoneGroupTopologySubscription::new(
            representative_speaker.clone(),
            network_speakers,
            callback_url.clone(),
            config,
        );

        assert!(subscription.is_ok());

        let sub = subscription.unwrap();
        assert_eq!(sub.service_type(), ServiceType::ZoneGroupTopology);
        assert_eq!(sub.subscription_scope(), SubscriptionScope::NetworkWide);
        assert_eq!(sub.speaker_id(), representative_speaker.id);
        assert_eq!(sub.callback_url(), &callback_url);
        assert!(!sub.is_active());
        assert!(sub.subscription_id().is_none());
    }

    #[test]
    fn test_extract_attribute() {
        let representative_speaker = create_test_speaker("123456789", "192.168.1.100");
        let subscription = ZoneGroupTopologySubscription::new(
            representative_speaker,
            vec![],
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        let xml = r#"<ZoneGroup Coordinator="RINCON_123456789" ID="RINCON_123456789:1">"#;

        let coordinator = subscription.extract_attribute(xml, "Coordinator").unwrap();
        assert_eq!(coordinator, "RINCON_123456789");

        let id = subscription.extract_attribute(xml, "ID").unwrap();
        assert_eq!(id, "RINCON_123456789:1");

        // Test missing attribute
        let result = subscription.extract_attribute(xml, "NonExistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_zone_group_member() {
        let representative_speaker = create_test_speaker("123456789", "192.168.1.100");
        let subscription = ZoneGroupTopologySubscription::new(
            representative_speaker,
            vec![],
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        let member_xml = r#"<ZoneGroupMember UUID="RINCON_123456789" Location="http://192.168.1.100:1400/xml/device_description.xml" ZoneName="Living Room" />"#;

        let result = subscription.parse_zone_group_member(member_xml).unwrap();
        assert!(result.is_some());

        let (speaker_id, satellites) = result.unwrap();
        assert_eq!(speaker_id, SpeakerId::from_udn("uuid:RINCON_123456789"));
        assert_eq!(satellites.len(), 0); // No satellites parsed yet
    }

    #[test]
    fn test_decode_xml_entities() {
        let representative_speaker = create_test_speaker("123456789", "192.168.1.100");
        let subscription = ZoneGroupTopologySubscription::new(
            representative_speaker,
            vec![],
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        let encoded = "&lt;ZoneGroupState&gt;&lt;ZoneGroups&gt;&amp;test&amp;&lt;/ZoneGroups&gt;&lt;/ZoneGroupState&gt;";
        let decoded = subscription.decode_xml_entities(encoded);
        assert_eq!(
            decoded,
            "<ZoneGroupState><ZoneGroups>&test&</ZoneGroups></ZoneGroupState>"
        );
    }

    #[test]
    fn test_extract_property_value() {
        let representative_speaker = create_test_speaker("123456789", "192.168.1.100");
        let subscription = ZoneGroupTopologySubscription::new(
            representative_speaker,
            vec![],
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        let xml = r#"
            <property>
                <ZoneGroupState>&lt;ZoneGroupState&gt;test&lt;/ZoneGroupState&gt;</ZoneGroupState>
            </property>
        "#;

        let result = subscription.extract_property_value(xml, "ZoneGroupState");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "<ZoneGroupState>test</ZoneGroupState>");

        // Test missing property
        let result = subscription.extract_property_value(xml, "NonExistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_zone_group_state_empty() {
        let representative_speaker = create_test_speaker("123456789", "192.168.1.100");
        let subscription = ZoneGroupTopologySubscription::new(
            representative_speaker,
            vec![],
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        let xml = r#"
            <property>
                <SomeOtherProperty>value</SomeOtherProperty>
            </property>
        "#;

        let groups = subscription.parse_zone_group_state(xml).unwrap();
        assert_eq!(groups.len(), 0);
    }

    #[test]
    fn test_detect_topology_changes_initial() {
        let representative_speaker = create_test_speaker("123456789", "192.168.1.100");
        let subscription = ZoneGroupTopologySubscription::new(
            representative_speaker,
            vec![],
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        let coordinator_id = SpeakerId::from_udn("uuid:RINCON_123456789");
        let groups = vec![Group::new(coordinator_id)];

        let changes = subscription.detect_topology_changes(&groups);
        assert_eq!(changes.len(), 1);

        // Should generate GroupFormed event for initial topology
        if let StateChange::GroupFormed {
            group_id: _,
            coordinator_id: coord_id,
            initial_members,
        } = &changes[0]
        {
            assert_eq!(*coord_id, coordinator_id);
            assert_eq!(initial_members.len(), 1);
            assert!(initial_members.contains(&coordinator_id));
        } else {
            panic!("Expected GroupFormed for initial topology, got: {:?}", changes[0]);
        }
    }

    #[test]
    fn test_detect_topology_changes_subsequent() {
        let representative_speaker = create_test_speaker("123456789", "192.168.1.100");
        let subscription = ZoneGroupTopologySubscription::new(
            representative_speaker,
            vec![],
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        let coordinator_id = SpeakerId::from_udn("uuid:RINCON_123456789");
        let initial_groups = vec![Group::new(coordinator_id)];

        // First call - initial topology
        let _initial_changes = subscription.detect_topology_changes(&initial_groups);

        // Second call - same topology (should generate GroupTopologyChanged with no specific changes)
        let changes = subscription.detect_topology_changes(&initial_groups);
        assert_eq!(changes.len(), 1);

        // Should generate GroupTopologyChanged event
        if let StateChange::GroupTopologyChanged {
            groups: changed_groups,
            speakers_joined,
            speakers_left,
            coordinator_changes,
        } = &changes[0]
        {
            assert_eq!(changed_groups.len(), 1);
            assert_eq!(changed_groups[0].coordinator, coordinator_id);
            assert_eq!(speakers_joined.len(), 0);
            assert_eq!(speakers_left.len(), 0);
            assert_eq!(coordinator_changes.len(), 0);
        } else {
            panic!("Expected GroupTopologyChanged");
        }
    }

    #[test]
    fn test_update_network_speakers() {
        let representative_speaker = create_test_speaker("123456789", "192.168.1.100");
        let initial_speakers = vec![representative_speaker.clone()];

        let mut subscription = ZoneGroupTopologySubscription::new(
            representative_speaker.clone(),
            initial_speakers,
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        assert_eq!(subscription.network_speakers.len(), 1);

        let new_speakers = vec![
            representative_speaker,
            create_test_speaker("987654321", "192.168.1.101"),
            create_test_speaker("111222333", "192.168.1.102"),
        ];

        subscription.update_network_speakers(new_speakers);
        assert_eq!(subscription.network_speakers.len(), 3);
    }

    #[test]
    fn test_parse_zone_group_state_with_valid_xml() {
        let representative_speaker = create_test_speaker("123456789", "192.168.1.100");
        let subscription = ZoneGroupTopologySubscription::new(
            representative_speaker,
            vec![],
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        let xml = r#"
            <property>
                <ZoneGroupState>&lt;ZoneGroupState&gt;&lt;ZoneGroups&gt;&lt;ZoneGroup Coordinator="RINCON_123456789" ID="RINCON_123456789:1"&gt;&lt;ZoneGroupMember UUID="RINCON_123456789" Location="http://192.168.1.100:1400/xml/device_description.xml" ZoneName="Living Room" /&gt;&lt;/ZoneGroup&gt;&lt;/ZoneGroups&gt;&lt;/ZoneGroupState&gt;</ZoneGroupState>
            </property>
        "#;

        let groups = subscription.parse_zone_group_state(xml).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].coordinator, SpeakerId::from_udn("uuid:RINCON_123456789"));
        assert_eq!(groups[0].member_count(), 1);
    }

    #[test]
    fn test_parse_zone_group_state_with_empty_xml() {
        let representative_speaker = create_test_speaker("123456789", "192.168.1.100");
        let subscription = ZoneGroupTopologySubscription::new(
            representative_speaker,
            vec![],
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        let result = subscription.parse_zone_group_state("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Empty XML content"));
    }

    #[test]
    fn test_parse_zone_group_state_with_malformed_xml() {
        let representative_speaker = create_test_speaker("123456789", "192.168.1.100");
        let subscription = ZoneGroupTopologySubscription::new(
            representative_speaker,
            vec![],
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        let xml = r#"<invalid>not a upnp event</invalid>"#;

        let result = subscription.parse_zone_group_state(xml);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid UPnP event structure"));
    }

    #[test]
    fn test_parse_member_satellites() {
        let representative_speaker = create_test_speaker("123456789", "192.168.1.100");
        let subscription = ZoneGroupTopologySubscription::new(
            representative_speaker,
            vec![],
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        // Test with Satellites attribute
        let member_xml = r#"<ZoneGroupMember UUID="RINCON_123456789" Satellites="RINCON_SAT001,RINCON_SAT002" />"#;
        let satellites = subscription.parse_member_satellites(member_xml).unwrap();
        assert_eq!(satellites.len(), 2);

        // Test with no satellites
        let member_xml_no_sats = r#"<ZoneGroupMember UUID="RINCON_123456789" />"#;
        let satellites_empty = subscription.parse_member_satellites(member_xml_no_sats).unwrap();
        assert_eq!(satellites_empty.len(), 0);
    }

    #[test]
    fn test_decode_xml_entities_with_cdata() {
        let representative_speaker = create_test_speaker("123456789", "192.168.1.100");
        let subscription = ZoneGroupTopologySubscription::new(
            representative_speaker,
            vec![],
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        let encoded = "<![CDATA[<ZoneGroupState><ZoneGroups></ZoneGroups></ZoneGroupState>]]>";
        let decoded = subscription.decode_xml_entities(encoded);
        assert_eq!(decoded, "<ZoneGroupState><ZoneGroups></ZoneGroups></ZoneGroupState>");

        let encoded_with_entities = "&lt;test&gt;&amp;data&amp;";
        let decoded_entities = subscription.decode_xml_entities(encoded_with_entities);
        assert_eq!(decoded_entities, "<test>&data&");
    }

    #[test]
    fn test_extract_attribute_with_different_quote_styles() {
        let representative_speaker = create_test_speaker("123456789", "192.168.1.100");
        let subscription = ZoneGroupTopologySubscription::new(
            representative_speaker,
            vec![],
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        // Test double quotes
        let xml1 = r#"<ZoneGroup Coordinator="RINCON_123456789" />"#;
        let result1 = subscription.extract_attribute(xml1, "Coordinator").unwrap();
        assert_eq!(result1, "RINCON_123456789");

        // Test single quotes
        let xml2 = r#"<ZoneGroup Coordinator='RINCON_123456789' />"#;
        let result2 = subscription.extract_attribute(xml2, "Coordinator").unwrap();
        assert_eq!(result2, "RINCON_123456789");

        // Test with spaces
        let xml3 = r#"<ZoneGroup Coordinator = "RINCON_123456789" />"#;
        let result3 = subscription.extract_attribute(xml3, "Coordinator").unwrap();
        assert_eq!(result3, "RINCON_123456789");
    }
}
