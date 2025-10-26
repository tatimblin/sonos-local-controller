use super::types::XmlZoneGroupData;
use crate::models::{Group, GroupId, Speaker, SpeakerId, StateChange};
use crate::streaming::subscription::{ServiceSubscription, SubscriptionError, SubscriptionResult};
use crate::streaming::{ServiceType, SubscriptionConfig, SubscriptionId, SubscriptionScope};
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
    /// * `callback_url` - URL where the device should send event notifications
    /// * `config` - Configuration for this subscription
    pub fn new(
        representative_speaker: Speaker,
        callback_url: String,
        config: SubscriptionConfig,
    ) -> SubscriptionResult<Self> {
        Ok(Self {
            representative_speaker,
            subscription_id: None,
            upnp_sid: None,
            callback_url,
            config,
            active: false,
            last_renewal: None,
            last_zone_groups: Mutex::new(None),
        })
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

        // Use the new ZoneGroupTopologyParser to parse zone group state from UPnP event
        match crate::services::zone_group_topology::parser::ZoneGroupTopologyParser::from_xml(xml) {
            Ok(parser) => {
                match parser.zone_groups() {
                    Some(zone_groups) => {
                        if zone_groups.is_empty() {
                            println!("ℹ️ No zone groups found in UPnP event");
                        } else {
                            println!("✅ Found {} zone groups in UPnP event", zone_groups.len());
                        }

                        // Convert new parser data structures to domain models
                        groups = self.convert_new_parser_groups_to_domain_models(zone_groups)?;
                    }
                    None => {
                        println!("ℹ️ Parser returned None for zone_groups()");
                        // Return empty groups list
                        groups = Vec::new();
                    }
                }
            }
            Err(e) => {
                return Err(SubscriptionError::EventParseError(format!(
                    "Failed to parse ZoneGroupState from UPnP event: {}",
                    e
                )));
            }
        }

        println!("✅ Successfully parsed {} groups", groups.len());
        Ok(groups)
    }

    /// Convert XML zone group data structures to domain models
    fn convert_xml_groups_to_domain_models(
        &self,
        xml_groups: Vec<XmlZoneGroupData>,
    ) -> SubscriptionResult<Vec<Group>> {
        let mut groups = Vec::new();

        for xml_group in xml_groups {
            if xml_group.coordinator.trim().is_empty() {
                println!("⚠️ Skipping group with empty coordinator");
                continue;
            }

            let coordinator_id = SpeakerId::from_udn(&format!("uuid:{}", xml_group.coordinator));
            let mut group = Group::new(coordinator_id);

            println!(
                "🔍 Converting XML group with coordinator: {}",
                xml_group.coordinator
            );

            for xml_member in xml_group.members {
                if xml_member.uuid.trim().is_empty() {
                    println!("⚠️ Skipping member with empty UUID");
                    continue;
                }

                let speaker_id = SpeakerId::from_udn(&format!("uuid:{}", xml_member.uuid));

                // Convert satellite UUIDs to SpeakerIds
                let satellites: Vec<SpeakerId> = xml_member
                    .satellites()
                    .iter()
                    .filter(|uuid| !uuid.trim().is_empty())
                    .map(|uuid| SpeakerId::from_udn(&format!("uuid:{}", uuid)))
                    .collect();

                let satellite_count = satellites.len();
                group.add_member_with_satellites(speaker_id, satellites);

                if satellite_count == 0 {
                    println!("✅ Added member {:?} to group", speaker_id);
                } else {
                    println!(
                        "✅ Added member {:?} with {} satellites to group",
                        speaker_id, satellite_count
                    );
                }
            }

            if group.member_count() == 0 {
                println!("⚠️ ZoneGroup has no valid members, skipping");
                continue;
            }

            println!(
                "✅ Successfully converted ZoneGroup with {} members",
                group.member_count()
            );
            groups.push(group);
        }

        Ok(groups)
    }

    /// Convert new parser zone group data structures to domain models
    fn convert_new_parser_groups_to_domain_models(
        &self,
        zone_groups: Vec<crate::services::zone_group_topology::parser::ZoneGroupInfo>,
    ) -> SubscriptionResult<Vec<Group>> {
        let mut groups = Vec::new();

        for zone_group in zone_groups {
            if zone_group.coordinator.trim().is_empty() {
                println!("⚠️ Skipping group with empty coordinator");
                continue;
            }

            let coordinator_id = SpeakerId::from_udn(&format!("uuid:{}", zone_group.coordinator));
            let mut group = Group::new(coordinator_id);

            println!(
                "🔍 Converting new parser group with coordinator: {}",
                zone_group.coordinator
            );

            for member in zone_group.members {
                if member.uuid.trim().is_empty() {
                    println!("⚠️ Skipping member with empty UUID");
                    continue;
                }

                let speaker_id = SpeakerId::from_udn(&format!("uuid:{}", member.uuid));

                // Convert satellite UUIDs to SpeakerIds
                let satellites: Vec<SpeakerId> = member
                    .satellites
                    .iter()
                    .filter(|uuid| !uuid.trim().is_empty())
                    .map(|uuid| SpeakerId::from_udn(&format!("uuid:{}", uuid)))
                    .collect();

                let satellite_count = satellites.len();
                group.add_member_with_satellites(speaker_id, satellites);

                if satellite_count == 0 {
                    println!("✅ Added member {:?} to group", speaker_id);
                } else {
                    println!(
                        "✅ Added member {:?} with {} satellites to group",
                        speaker_id, satellite_count
                    );
                }
            }

            if group.member_count() == 0 {
                println!("⚠️ ZoneGroup has no valid members, skipping");
                continue;
            }

            println!(
                "✅ Successfully converted ZoneGroup with {} members",
                group.member_count()
            );
            groups.push(group);
        }

        Ok(groups)
    }

    /// Detect changes between old and new zone group states
    fn detect_topology_changes(&self, new_groups: &[Group]) -> Vec<StateChange> {
        let mut changes = Vec::new();

        match &*self.last_zone_groups.lock().unwrap() {
            None => {
                // First time receiving topology - store it without generating events
                // Groups should already be initialized via state_cache.initialize() during discovery
                // Skip initial GroupFormed events to prevent conflicts with initialization
                println!("🔍 Initial topology received with {} groups (storing for future change detection)", new_groups.len());

                // Store the initial topology for future change detection without generating events
                // This satisfies requirements 6.1 and 6.2 by ensuring ZoneGroupTopology events
                // only process actual topology changes, not initial state
            }
            Some(old_groups) => {
                // Compare old and new states to detect specific changes
                let (speakers_joined, speakers_left, coordinator_changes) =
                    self.analyze_topology_changes(old_groups, new_groups);

                // Generate events in the correct order: structural changes first, then membership changes

                // 1. First, detect newly formed groups (must come before SpeakerJoinedGroup)
                for group in new_groups {
                    if !old_groups.iter().any(|old_group| old_group.id == group.id) {
                        changes.push(StateChange::GroupFormed {
                            group_id: group.id,
                            coordinator_id: group.coordinator,
                            initial_members: group.all_speaker_ids(),
                        });
                    }
                }

                // 2. Then, detect dissolved groups (must come before SpeakerLeftGroup)
                for old_group in old_groups {
                    if !new_groups.iter().any(|group| group.id == old_group.id) {
                        changes.push(StateChange::GroupDissolved {
                            group_id: old_group.id,
                            former_coordinator: old_group.coordinator,
                            former_members: old_group.all_speaker_ids(),
                        });
                    }
                }

                // 3. Then, process speaker membership changes (groups now exist)
                for (speaker_id, former_group_id) in &speakers_left {
                    if let Some(group_id) = former_group_id {
                        changes.push(StateChange::SpeakerLeftGroup {
                            speaker_id: *speaker_id,
                            former_group_id: *group_id,
                        });
                    }
                }

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

                // 4. Finally, process coordinator changes within existing groups
                for (group_id, old_coordinator, new_coordinator) in &coordinator_changes {
                    changes.push(StateChange::CoordinatorChanged {
                        group_id: *group_id,
                        old_coordinator: *old_coordinator,
                        new_coordinator: *new_coordinator,
                    });
                }
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
        Vec<(SpeakerId, GroupId)>,            // speakers_joined
        Vec<(SpeakerId, Option<GroupId>)>,    // speakers_left
        Vec<(GroupId, SpeakerId, SpeakerId)>, // coordinator_changes
    ) {
        let mut speakers_joined = Vec::new();
        let mut speakers_left = Vec::new();
        let mut coordinator_changes = Vec::new();

        // Build maps for easier lookup
        let old_speaker_to_group: HashMap<SpeakerId, GroupId> = old_groups
            .iter()
            .flat_map(|group| {
                group
                    .all_speaker_ids()
                    .into_iter()
                    .map(move |speaker_id| (speaker_id, group.id))
            })
            .collect();

        let new_speaker_to_group: HashMap<SpeakerId, GroupId> = new_groups
            .iter()
            .flat_map(|group| {
                group
                    .all_speaker_ids()
                    .into_iter()
                    .map(move |speaker_id| (speaker_id, group.id))
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
        // These speakers became solo, they didn't vanish from the network
        for (speaker_id, old_group_id) in &old_speaker_to_group {
            if !new_speaker_to_group.contains_key(speaker_id) {
                // Speaker left group and became solo (still exists in network)
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
        let callback_url = "http://localhost:8080/callback/test".to_string();
        let config = SubscriptionConfig::default();

        let subscription = ZoneGroupTopologySubscription::new(
            representative_speaker.clone(),
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
    fn test_parse_zone_group_member() {
        let member_xml = r#"<ZoneGroupMember UUID="RINCON_123456789" Location="http://192.168.1.100:1400/xml/device_description.xml" ZoneName="Living Room" />"#;

        // Use XML parser directly
        let result =
            crate::services::zone_group_topology::parser::parse_zone_group_member(member_xml)
                .unwrap();

        assert_eq!(result.uuid, "RINCON_123456789");
        assert_eq!(result.satellites().len(), 0); // No satellites in this test
    }

    #[test]
    fn test_decode_xml_entities() {
        let encoded = "&lt;ZoneGroupState&gt;&lt;ZoneGroups&gt;&amp;test&amp;&lt;/ZoneGroups&gt;&lt;/ZoneGroupState&gt;";
        let decoded = crate::xml::XmlParser::decode_entities_with_cdata(encoded);
        assert_eq!(
            decoded,
            "<ZoneGroupState><ZoneGroups>&test&</ZoneGroups></ZoneGroupState>"
        );
    }

    #[test]
    fn test_parse_zone_group_state_empty() {
        let representative_speaker = create_test_speaker("123456789", "192.168.1.100");
        let subscription = ZoneGroupTopologySubscription::new(
            representative_speaker,
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
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        let coordinator_id = SpeakerId::from_udn("uuid:RINCON_123456789");
        let groups = vec![Group::new(coordinator_id)];

        let changes = subscription.detect_topology_changes(&groups);

        // Should NOT generate any events for initial topology to avoid conflicts
        // with groups already established during discovery (requirements 6.1, 6.2)
        assert_eq!(changes.len(), 0);

        // Verify that the topology was stored for future change detection
        let stored_groups = subscription.last_zone_groups.lock().unwrap();
        assert!(stored_groups.is_some());
        assert_eq!(stored_groups.as_ref().unwrap().len(), 1);
        assert_eq!(
            stored_groups.as_ref().unwrap()[0].coordinator,
            coordinator_id
        );
    }

    #[test]
    fn test_detect_topology_changes_subsequent() {
        let representative_speaker = create_test_speaker("123456789", "192.168.1.100");
        let subscription = ZoneGroupTopologySubscription::new(
            representative_speaker,
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        let coordinator_id = SpeakerId::from_udn("uuid:RINCON_123456789");
        let initial_groups = vec![Group::new(coordinator_id)];

        // First call - initial topology (should not generate events)
        let changes = subscription.detect_topology_changes(&initial_groups);
        assert_eq!(changes.len(), 0);

        // Second call - topology change (should generate events)
        let new_coordinator_id = SpeakerId::from_udn("uuid:RINCON_987654321");
        let mut new_group = Group::new(new_coordinator_id);
        new_group.add_member(coordinator_id); // Add the original speaker as a member
        let new_groups = vec![new_group];

        let changes = subscription.detect_topology_changes(&new_groups);

        // Should generate events for actual topology changes
        assert!(changes.len() > 0);

        // Should include GroupFormed for the new group and GroupDissolved for the old one
        let has_group_formed = changes
            .iter()
            .any(|change| matches!(change, StateChange::GroupFormed { .. }));
        let has_group_dissolved = changes
            .iter()
            .any(|change| matches!(change, StateChange::GroupDissolved { .. }));

        assert!(
            has_group_formed,
            "Should generate GroupFormed event for new group"
        );
        assert!(
            has_group_dissolved,
            "Should generate GroupDissolved event for old group"
        );
    }

    #[test]
    fn test_parse_zone_group_state_with_valid_xml() {
        let representative_speaker = create_test_speaker("123456789", "192.168.1.100");
        let subscription = ZoneGroupTopologySubscription::new(
            representative_speaker,
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
        assert_eq!(
            groups[0].coordinator,
            SpeakerId::from_udn("uuid:RINCON_123456789")
        );
        assert_eq!(groups[0].member_count(), 1);
    }

    #[test]
    fn test_parse_zone_group_state_with_empty_xml() {
        let representative_speaker = create_test_speaker("123456789", "192.168.1.100");
        let subscription = ZoneGroupTopologySubscription::new(
            representative_speaker,
            "http://localhost:8080/callback".to_string(),
            SubscriptionConfig::default(),
        )
        .unwrap();

        let result = subscription.parse_zone_group_state("");
        assert!(result.is_err());

        if let Err(SubscriptionError::EventParseError(msg)) = result {
            assert_eq!(msg, "Empty XML content");
        } else {
            panic!("Expected EventParseError for empty XML");
        }
    }
}
