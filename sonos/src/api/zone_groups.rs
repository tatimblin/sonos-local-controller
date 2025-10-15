use crate::error::Result;
use crate::models::{Group, GroupId, SpeakerId};
use crate::transport::soap::{SoapClient, SoapRequest};

pub struct ZoneGroupsService;

impl ZoneGroupsService {
    const SERVICE_TYPE: &'static str = "urn:schemas-upnp-org:service:ZoneGroupTopology:1";
    const SERVICE_PATH: &'static str = "/ZoneGroupTopology/Control";

    pub fn get_zone_group_state(soap: &SoapClient, device_url: &str) -> Result<Vec<Group>> {
        let request = SoapRequest {
            service_type: Self::SERVICE_TYPE.to_string(),
            action: "GetZoneGroupState".to_string(),
            params: vec![],
        };

        let response = soap.call(device_url, Self::SERVICE_PATH, request)?;

        // Extract the ZoneGroupState from the SOAP response
        let zone_group_state_xml =
            crate::transport::soap::SoapClient::extract_xml_value(&response.body, "ZoneGroupState")
                .ok_or_else(|| {
                    crate::error::SonosError::InvalidState(
                        "No ZoneGroupState found in SOAP response".to_string(),
                    )
                })?;

        // Decode HTML entities in the XML
        let decoded_xml = Self::decode_html_entities(&zone_group_state_xml);

        Self::parse_zone_group_state(&decoded_xml)
    }

    fn parse_zone_group_state(xml: &str) -> Result<Vec<Group>> {
        let mut groups = Vec::new();

        let mut start = 0;
        while let Some(zone_start) = xml[start..].find("<ZoneGroup") {
            let absolute_start = start + zone_start;
            if let Some(zone_end) = xml[absolute_start..].find("</ZoneGroup>") {
                let absolute_end = absolute_start + zone_end + "</ZoneGroup>".len();
                let zone_xml = &xml[absolute_start..absolute_end];

                if let Ok(group) = Self::parse_single_zone_group(zone_xml) {
                    groups.push(group);
                }

                start = absolute_end;
            } else {
                break;
            }
        }

        Ok(groups)
    }

    fn parse_single_zone_group(zone_xml: &str) -> Result<Group> {
        let coordinator_udn =
            Self::extract_attribute(zone_xml, "Coordinator").ok_or_else(|| {
                crate::error::SonosError::InvalidState("No coordinator in zone group".to_string())
            })?;

        let coordinator_id = SpeakerId::from_udn(&Self::uuid_to_udn(&coordinator_udn));
        let mut group = Group::new(coordinator_id);

        let mut start = 0;
        while let Some(member_start) = zone_xml[start..].find("<ZoneGroupMember") {
            let absolute_start = start + member_start;
            
            // Find the end of this ZoneGroupMember (could be self-closing or have content)
            let member_end_pos = if zone_xml[absolute_start..].contains("/>") && 
                                   zone_xml[absolute_start..].find("/>").unwrap() < zone_xml[absolute_start..].find(">").unwrap_or(usize::MAX) {
                // Self-closing tag
                let self_close = zone_xml[absolute_start..].find("/>").unwrap();
                absolute_start + self_close + "/>".len()
            } else if let Some(close_start) = zone_xml[absolute_start..].find(">") {
                // Has content, find the closing tag
                if let Some(close_tag_start) = zone_xml[absolute_start..].find("</ZoneGroupMember>") {
                    absolute_start + close_tag_start + "</ZoneGroupMember>".len()
                } else {
                    // Malformed, skip
                    absolute_start + close_start + ">".len()
                }
            } else {
                break;
            };

            let member_xml = &zone_xml[absolute_start..member_end_pos];

            if let Some(uuid) = Self::extract_attribute(member_xml, "UUID") {
                let member_id = SpeakerId::from_udn(&Self::uuid_to_udn(&uuid));
                
                // Parse satellites for this member
                let satellites = Self::parse_satellites(member_xml);
                
                if member_id == coordinator_id {
                    // Update coordinator with satellites
                    if let Some(coordinator_member) = group.members.iter_mut().find(|m| m.speaker_id == coordinator_id) {
                        coordinator_member.satellites = satellites;
                    }
                } else {
                    // Add as regular member with satellites
                    group.add_member_with_satellites(member_id, satellites);
                }
            }

            start = member_end_pos;
        }

        Ok(group)
    }

    fn parse_satellites(member_xml: &str) -> Vec<SpeakerId> {
        let mut satellites = Vec::new();
        let mut start = 0;
        
        while let Some(satellite_start) = member_xml[start..].find("<Satellite") {
            let absolute_start = start + satellite_start;
            if let Some(satellite_end) = member_xml[absolute_start..].find("/>") {
                let absolute_end = absolute_start + satellite_end + "/>".len();
                let satellite_xml = &member_xml[absolute_start..absolute_end];

                if let Some(uuid) = Self::extract_attribute(satellite_xml, "UUID") {
                    let satellite_id = SpeakerId::from_udn(&Self::uuid_to_udn(&uuid));
                    satellites.push(satellite_id);
                }

                start = absolute_end;
            } else {
                break;
            }
        }
        
        satellites
    }

    fn extract_attribute(xml: &str, attr_name: &str) -> Option<String> {
        let pattern = format!("{}=\"", attr_name);
        xml.find(&pattern).and_then(|start| {
            let content_start = start + pattern.len();
            xml[content_start..]
                .find('"')
                .map(|end| xml[content_start..content_start + end].to_string())
        })
    }

    fn decode_html_entities(xml: &str) -> String {
        xml.replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&amp;", "&") // This should be last to avoid double-decoding
    }

    /// Convert a UUID from zone group topology to full UDN format
    /// Converts "RINCON_C43875CA135801400" to "uuid:RINCON_C43875CA135801400::1"
    fn uuid_to_udn(uuid: &str) -> String {
        if uuid.starts_with("RINCON_") {
            format!("uuid:{}::1", uuid)
        } else {
            // If it's already in UDN format or some other format, return as-is
            uuid.to_string()
        }
    }
}

/// Convenience function for fetching zone groups from a speaker with default timeout
pub fn get_zone_groups_from_speaker(speaker: &crate::models::Speaker) -> Result<Vec<Group>> {
    use crate::transport::soap::SoapClient;
    use std::time::Duration;
    
    let soap_client = SoapClient::new(Duration::from_secs(5))?;
    let device_url = format!("http://{}:{}", speaker.ip_address, speaker.port);
    ZoneGroupsService::get_zone_group_state(&soap_client, &device_url)
}

/// Convenience function for fetching zone groups from a speaker with custom timeout
pub fn get_zone_groups_from_speaker_with_timeout(
    speaker: &crate::models::Speaker,
    timeout: std::time::Duration,
) -> Result<Vec<Group>> {
    use crate::transport::soap::SoapClient;
    
    let soap_client = SoapClient::new(timeout)?;
    let device_url = format!("http://{}:{}", speaker.ip_address, speaker.port);
    ZoneGroupsService::get_zone_group_state(&soap_client, &device_url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{GroupId, SpeakerId};

    fn load_sample_topology() -> String {
        std::fs::read_to_string("tests/fixtures/topology.xml")
            .expect("Failed to load topology fixture")
    }

    #[test]
    fn test_extract_attribute() {
        let xml = r#"<ZoneGroup Coordinator="RINCON_C43875CA135801400" ID="RINCON_C43875CA135801400:2858411400">"#;

        assert_eq!(
            ZoneGroupsService::extract_attribute(xml, "Coordinator"),
            Some("RINCON_C43875CA135801400".to_string())
        );

        assert_eq!(
            ZoneGroupsService::extract_attribute(xml, "ID"),
            Some("RINCON_C43875CA135801400:2858411400".to_string())
        );

        assert_eq!(
            ZoneGroupsService::extract_attribute(xml, "NonExistent"),
            None
        );
    }

    #[test]
    fn test_uuid_to_udn() {
        // Test RINCON UUID conversion
        assert_eq!(
            ZoneGroupsService::uuid_to_udn("RINCON_C43875CA135801400"),
            "uuid:RINCON_C43875CA135801400::1"
        );

        // Test already formatted UDN (should return as-is)
        assert_eq!(
            ZoneGroupsService::uuid_to_udn("uuid:RINCON_C43875CA135801400::1"),
            "uuid:RINCON_C43875CA135801400::1"
        );

        // Test non-RINCON UUID (should return as-is)
        assert_eq!(
            ZoneGroupsService::uuid_to_udn("some-other-uuid"),
            "some-other-uuid"
        );
    }

    #[test]
    fn test_extract_attribute_with_member() {
        let xml = r#"<ZoneGroupMember UUID="RINCON_C43875CA135801400" Location="http://192.168.4.65:1400/xml/device_description.xml" ZoneName="Roam 2"/>"#;

        assert_eq!(
            ZoneGroupsService::extract_attribute(xml, "UUID"),
            Some("RINCON_C43875CA135801400".to_string())
        );

        assert_eq!(
            ZoneGroupsService::extract_attribute(xml, "ZoneName"),
            Some("Roam 2".to_string())
        );

        assert_eq!(
            ZoneGroupsService::extract_attribute(xml, "Location"),
            Some("http://192.168.4.65:1400/xml/device_description.xml".to_string())
        );
    }

    #[test]
    fn test_parse_single_zone_group_simple() {
        let zone_xml = r#"<ZoneGroup Coordinator="RINCON_C43875CA135801400" ID="RINCON_C43875CA135801400:2858411400"><ZoneGroupMember UUID="RINCON_C43875CA135801400" Location="http://192.168.4.65:1400/xml/device_description.xml" ZoneName="Roam 2"/></ZoneGroup>"#;

        let result = ZoneGroupsService::parse_single_zone_group(zone_xml);
        assert!(result.is_ok());

        let group = result.unwrap();
        let expected_coordinator = SpeakerId::from_udn("uuid:RINCON_C43875CA135801400::1");

        assert_eq!(group.coordinator, expected_coordinator);
        assert_eq!(group.id, GroupId::from_coordinator(expected_coordinator));
        assert_eq!(group.members.len(), 1);
        assert!(group.is_member(expected_coordinator));
        
        // Should have no satellites
        let coordinator_member = group.members.iter().find(|m| m.speaker_id == expected_coordinator).unwrap();
        assert_eq!(coordinator_member.satellites.len(), 0);
    }

    #[test]
    fn test_parse_single_zone_group_with_satellites() {
        let zone_xml = r#"<ZoneGroup Coordinator="RINCON_5CAAFDAE58BD01400" ID="RINCON_804AF2AA2FA201400:1331296849"><ZoneGroupMember UUID="RINCON_5CAAFDAE58BD01400" Location="http://192.168.4.94:1400/xml/device_description.xml" ZoneName="Basement"><Satellite UUID="RINCON_7828CA128F0001400" Location="http://192.168.4.93:1400/xml/device_description.xml" ZoneName="Basement"/><Satellite UUID="RINCON_7828CAFB9D9C01400" Location="http://192.168.4.92:1400/xml/device_description.xml" ZoneName="Basement"/></ZoneGroupMember></ZoneGroup>"#;

        let result = ZoneGroupsService::parse_single_zone_group(zone_xml);
        assert!(result.is_ok());

        let group = result.unwrap();
        let expected_coordinator = SpeakerId::from_udn("uuid:RINCON_5CAAFDAE58BD01400::1");
        let expected_satellite1 = SpeakerId::from_udn("uuid:RINCON_7828CA128F0001400::1");
        let expected_satellite2 = SpeakerId::from_udn("uuid:RINCON_7828CAFB9D9C01400::1");

        assert_eq!(group.coordinator, expected_coordinator);
        assert_eq!(group.id, GroupId::from_coordinator(expected_coordinator));
        
        // Should have the coordinator as member with satellites nested under it
        assert_eq!(group.members.len(), 1);
        assert!(group.is_member(expected_coordinator));
        
        // Check that satellites are properly nested under the coordinator
        let coordinator_member = group.members.iter().find(|m| m.speaker_id == expected_coordinator).unwrap();
        assert_eq!(coordinator_member.satellites.len(), 2);
        assert!(coordinator_member.satellites.contains(&expected_satellite1));
        assert!(coordinator_member.satellites.contains(&expected_satellite2));
        
        // Check that all_speaker_ids includes satellites
        let all_ids = group.all_speaker_ids();
        assert_eq!(all_ids.len(), 3);
        assert!(all_ids.contains(&expected_coordinator));
        assert!(all_ids.contains(&expected_satellite1));
        assert!(all_ids.contains(&expected_satellite2));
    }

    #[test]
    fn test_parse_single_zone_group_missing_coordinator() {
        let zone_xml = r#"<ZoneGroup ID="RINCON_C43875CA135801400:2858411400"><ZoneGroupMember UUID="RINCON_C43875CA135801400"/></ZoneGroup>"#;

        let result = ZoneGroupsService::parse_single_zone_group(zone_xml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_zone_group_state_multiple_groups() {
        let sample_xml = load_sample_topology();
        let result = ZoneGroupsService::parse_zone_group_state(&sample_xml);
        assert!(result.is_ok());

        let groups = result.unwrap();
        assert_eq!(groups.len(), 3); // Should parse 3 zone groups from the sample

        // Verify first group (Roam 2)
        let roam_coordinator = SpeakerId::from_udn("uuid:RINCON_C43875CA135801400::1");
        let roam_group = groups.iter().find(|g| g.coordinator == roam_coordinator);
        assert!(roam_group.is_some());
        let roam_group = roam_group.unwrap();
        assert_eq!(roam_group.members.len(), 1);

        // Verify second group (Living Room)
        let living_room_coordinator = SpeakerId::from_udn("uuid:RINCON_804AF2AA2FA201400::1");
        let living_room_group = groups
            .iter()
            .find(|g| g.coordinator == living_room_coordinator);
        assert!(living_room_group.is_some());
        let living_room_group = living_room_group.unwrap();
        assert_eq!(living_room_group.members.len(), 1);

        // Verify third group (Basement with satellites)
        let basement_coordinator = SpeakerId::from_udn("uuid:RINCON_5CAAFDAE58BD01400::1");
        let basement_group = groups
            .iter()
            .find(|g| g.coordinator == basement_coordinator);
        assert!(basement_group.is_some());
        let basement_group = basement_group.unwrap();
        assert_eq!(basement_group.members.len(), 1); // Only coordinator as member
        
        // But satellites should be nested under the coordinator
        let coordinator_member = basement_group.members.iter().find(|m| m.speaker_id == basement_coordinator).unwrap();
        assert_eq!(coordinator_member.satellites.len(), 2); // Should have 2 satellites
    }

    #[test]
    fn test_parse_zone_group_state_empty() {
        let empty_xml = r#"<ZoneGroupState><ZoneGroups></ZoneGroups></ZoneGroupState>"#;

        let result = ZoneGroupsService::parse_zone_group_state(empty_xml);
        assert!(result.is_ok());

        let groups = result.unwrap();
        assert_eq!(groups.len(), 0);
    }

    #[test]
    fn test_parse_zone_group_state_malformed() {
        let malformed_xml =
            r#"<ZoneGroupState><ZoneGroups><ZoneGroup Coordinator="RINCON_C43875CA135801400">"#;

        let result = ZoneGroupsService::parse_zone_group_state(malformed_xml);
        assert!(result.is_ok());

        let groups = result.unwrap();
        assert_eq!(groups.len(), 0); // Should handle malformed XML gracefully
    }

    #[test]
    fn test_parse_zone_group_state_with_multiple_members() {
        let xml_with_members = r#"<ZoneGroupState><ZoneGroups><ZoneGroup Coordinator="RINCON_COORDINATOR01400" ID="RINCON_COORDINATOR01400:123"><ZoneGroupMember UUID="RINCON_COORDINATOR01400" ZoneName="Coordinator"/><ZoneGroupMember UUID="RINCON_MEMBER001400" ZoneName="Member1"/><ZoneGroupMember UUID="RINCON_MEMBER002400" ZoneName="Member2"/></ZoneGroup></ZoneGroups></ZoneGroupState>"#;

        let result = ZoneGroupsService::parse_zone_group_state(xml_with_members);
        assert!(result.is_ok());

        let groups = result.unwrap();
        assert_eq!(groups.len(), 1);

        let group = &groups[0];
        let coordinator_id = SpeakerId::from_udn("uuid:RINCON_COORDINATOR01400::1");
        let member1_id = SpeakerId::from_udn("uuid:RINCON_MEMBER001400::1");
        let member2_id = SpeakerId::from_udn("uuid:RINCON_MEMBER002400::1");

        assert_eq!(group.coordinator, coordinator_id);
        assert_eq!(group.members.len(), 3);
        assert!(group.is_member(coordinator_id));
        assert!(group.is_member(member1_id));
        assert!(group.is_member(member2_id));
        
        // Check that all_speaker_ids works correctly
        let all_ids = group.all_speaker_ids();
        assert_eq!(all_ids.len(), 3);
        assert!(all_ids.contains(&coordinator_id));
        assert!(all_ids.contains(&member1_id));
        assert!(all_ids.contains(&member2_id));
    }
}
