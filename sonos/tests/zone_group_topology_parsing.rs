//! Unit tests for ZoneGroupTopology XML parsing functionality
//!
//! This module contains comprehensive tests for the ZoneGroupTopology service's
//! XML parsing capabilities, including valid XML parsing, error handling for
//! malformed XML, and event generation for different topology change scenarios.

use sonos::models::{Speaker, SpeakerId, StateChange};
use sonos::streaming::subscription::ServiceSubscription;
use sonos::streaming::{ServiceType, SubscriptionConfig, ZoneGroupTopologySubscription};
use std::fs;

/// Helper function to create a test speaker
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

/// Helper function to create a test subscription
fn create_test_subscription() -> ZoneGroupTopologySubscription {
    let representative_speaker = create_test_speaker("123456789", "192.168.1.100");
    let callback_url = "http://localhost:8080/callback/test".to_string();
    let config = SubscriptionConfig::default();

    ZoneGroupTopologySubscription::new(
        representative_speaker,
        callback_url,
        config,
    )
    .expect("Failed to create test subscription")
}

/// Helper function to wrap XML in UPnP event structure
fn wrap_in_upnp_event(zone_group_state_xml: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<e:propertyset xmlns:e="urn:schemas-upnp-org:event-1-0">
    <e:property>
        <ZoneGroupState>{}</ZoneGroupState>
    </e:property>
</e:propertyset>"#,
        zone_group_state_xml
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('&', "&amp;")
            .replace('"', "&quot;")
    )
}

/// Helper function to load test fixture
fn load_fixture(filename: &str) -> String {
    fs::read_to_string(format!("tests/fixtures/{}", filename))
        .expect(&format!("Failed to load fixture: {}", filename))
}

#[cfg(test)]
mod xml_parsing_tests {
    use super::*;

    #[test]
    fn test_parse_single_zone_group() {
        let subscription = create_test_subscription();
        let zone_group_xml = load_fixture("zone_group_topology_single.xml");
        let upnp_event = wrap_in_upnp_event(&zone_group_xml);

        let groups = subscription.parse_zone_group_state(&upnp_event).unwrap();

        assert_eq!(groups.len(), 1);
        let group = &groups[0];
        assert_eq!(
            group.coordinator,
            SpeakerId::from_udn("uuid:RINCON_123456789")
        );
        assert_eq!(group.member_count(), 1);
        assert!(group.is_member(SpeakerId::from_udn("uuid:RINCON_123456789")));
    }

    #[test]
    fn test_parse_multiple_zone_groups() {
        let subscription = create_test_subscription();
        let zone_group_xml = load_fixture("zone_group_topology_multiple.xml");
        let upnp_event = wrap_in_upnp_event(&zone_group_xml);

        let groups = subscription.parse_zone_group_state(&upnp_event).unwrap();

        assert_eq!(groups.len(), 2);

        // First group with 2 members
        let group1 = &groups[0];
        assert_eq!(
            group1.coordinator,
            SpeakerId::from_udn("uuid:RINCON_123456789")
        );
        assert_eq!(group1.member_count(), 2);
        assert!(group1.is_member(SpeakerId::from_udn("uuid:RINCON_123456789")));
        assert!(group1.is_member(SpeakerId::from_udn("uuid:RINCON_987654321")));

        // Second group with 1 member
        let group2 = &groups[1];
        assert_eq!(
            group2.coordinator,
            SpeakerId::from_udn("uuid:RINCON_111222333")
        );
        assert_eq!(group2.member_count(), 1);
        assert!(group2.is_member(SpeakerId::from_udn("uuid:RINCON_111222333")));
    }

    #[test]
    fn test_parse_zone_group_with_satellites() {
        let subscription = create_test_subscription();
        let zone_group_xml = load_fixture("zone_group_topology_with_satellites.xml");
        let upnp_event = wrap_in_upnp_event(&zone_group_xml);

        let groups = subscription.parse_zone_group_state(&upnp_event).unwrap();

        assert_eq!(groups.len(), 1);
        let group = &groups[0];
        assert_eq!(
            group.coordinator,
            SpeakerId::from_udn("uuid:RINCON_123456789")
        );
        assert_eq!(group.member_count(), 1);

        // Check that satellites are included in all_speaker_ids
        let all_ids = group.all_speaker_ids();
        assert!(
            all_ids.len() >= 3,
            "Expected at least 3 speakers (main + 2 satellites), got {}",
            all_ids.len()
        ); // Main speaker + 2 satellites
        assert!(all_ids.contains(&SpeakerId::from_udn("uuid:RINCON_123456789")));
        assert!(all_ids.contains(&SpeakerId::from_udn("uuid:RINCON_SAT001")));
        assert!(all_ids.contains(&SpeakerId::from_udn("uuid:RINCON_SAT002")));
    }

    #[test]
    fn test_parse_empty_zone_groups() {
        let subscription = create_test_subscription();
        let zone_group_xml = load_fixture("zone_group_topology_empty.xml");
        let upnp_event = wrap_in_upnp_event(&zone_group_xml);

        let groups = subscription.parse_zone_group_state(&upnp_event).unwrap();

        assert_eq!(groups.len(), 0);
    }

    #[test]
    fn test_parse_real_topology_fixture() {
        let subscription = create_test_subscription();
        let zone_group_xml = load_fixture("topology.xml");
        let upnp_event = wrap_in_upnp_event(&zone_group_xml);

        let groups = subscription.parse_zone_group_state(&upnp_event).unwrap();

        // The real fixture has 3 groups
        assert_eq!(groups.len(), 3);

        // Verify each group has the expected coordinator
        let coordinators: Vec<_> = groups.iter().map(|g| g.coordinator).collect();
        assert!(coordinators.contains(&SpeakerId::from_udn("uuid:RINCON_C43875CA135801400")));
        assert!(coordinators.contains(&SpeakerId::from_udn("uuid:RINCON_804AF2AA2FA201400")));
        assert!(coordinators.contains(&SpeakerId::from_udn("uuid:RINCON_5CAAFDAE58BD01400")));

        // Find the group with satellites (basement group)
        let basement_group = groups
            .iter()
            .find(|g| g.coordinator == SpeakerId::from_udn("uuid:RINCON_5CAAFDAE58BD01400"))
            .expect("Basement group not found");

        // This group should have satellites
        let all_ids = basement_group.all_speaker_ids();
        assert!(all_ids.len() > 1); // Main speaker + satellites
        assert!(all_ids.contains(&SpeakerId::from_udn("uuid:RINCON_5CAAFDAE58BD01400")));
        assert!(all_ids.contains(&SpeakerId::from_udn("uuid:RINCON_7828CA128F0001400")));
        assert!(all_ids.contains(&SpeakerId::from_udn("uuid:RINCON_7828CAFB9D9C01400")));
    }
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[test]
    fn test_parse_empty_xml() {
        let subscription = create_test_subscription();

        let result = subscription.parse_zone_group_state("");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Empty XML content"));
    }

    #[test]
    fn test_parse_malformed_xml() {
        let subscription = create_test_subscription();
        let malformed_xml = load_fixture("zone_group_topology_malformed.xml");
        let upnp_event = wrap_in_upnp_event(&malformed_xml);

        let result = subscription.parse_zone_group_state(&upnp_event);
        // Should handle gracefully and return empty groups due to parsing errors
        assert!(result.is_ok());
        let groups = result.unwrap();
        assert_eq!(groups.len(), 0); // No valid groups parsed due to malformed XML
    }

    #[test]
    fn test_parse_invalid_upnp_structure() {
        let subscription = create_test_subscription();
        let invalid_xml = "<invalid>not a upnp event</invalid>";

        let result = subscription.parse_zone_group_state(invalid_xml);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid UPnP event structure"));
    }

    #[test]
    fn test_parse_missing_coordinator_attribute() {
        let subscription = create_test_subscription();
        let xml_without_coordinator = r#"
            <ZoneGroupState>
                <ZoneGroups>
                    <ZoneGroup ID="RINCON_123456789:1">
                        <ZoneGroupMember UUID="RINCON_123456789" Location="http://192.168.1.100:1400/xml/device_description.xml" ZoneName="Living Room" />
                    </ZoneGroup>
                </ZoneGroups>
            </ZoneGroupState>
        "#;
        let upnp_event = wrap_in_upnp_event(xml_without_coordinator);

        let result = subscription.parse_zone_group_state(&upnp_event);
        // Should handle gracefully and continue parsing other groups
        assert!(result.is_ok());
        let groups = result.unwrap();
        // Group should be created but with no valid members due to missing UUID
        assert_eq!(groups.len(), 0);
    }

    #[test]
    fn test_parse_missing_uuid_attribute() {
        let subscription = create_test_subscription();
        let xml_without_uuid = r#"
            <ZoneGroupState>
                <ZoneGroups>
                    <ZoneGroup Coordinator="RINCON_123456789" ID="RINCON_123456789:1">
                        <ZoneGroupMember Location="http://192.168.1.100:1400/xml/device_description.xml" ZoneName="Living Room" />
                    </ZoneGroup>
                </ZoneGroups>
            </ZoneGroupState>
        "#;
        let upnp_event = wrap_in_upnp_event(xml_without_uuid);

        let result = subscription.parse_zone_group_state(&upnp_event);
        // Should handle gracefully and continue parsing
        assert!(result.is_ok());
        let groups = result.unwrap();
        // Group should be created but with no valid members
        assert_eq!(groups.len(), 0);
    }

    #[test]
    fn test_parse_empty_coordinator_attribute() {
        let subscription = create_test_subscription();
        let xml_empty_coordinator = r#"
            <ZoneGroupState>
                <ZoneGroups>
                    <ZoneGroup Coordinator="" ID="RINCON_123456789:1">
                        <ZoneGroupMember UUID="RINCON_123456789" Location="http://192.168.1.100:1400/xml/device_description.xml" ZoneName="Living Room" />
                    </ZoneGroup>
                </ZoneGroups>
            </ZoneGroupState>
        "#;
        let upnp_event = wrap_in_upnp_event(xml_empty_coordinator);

        let result = subscription.parse_zone_group_state(&upnp_event);
        // Should handle gracefully
        assert!(result.is_ok());
        let groups = result.unwrap();
        assert_eq!(groups.len(), 0); // No valid groups with empty coordinator
    }

    #[test]
    fn test_parse_too_many_groups() {
        let subscription = create_test_subscription();

        // Create XML with many groups to test the limit
        let mut xml = String::from("<ZoneGroupState><ZoneGroups>");
        for i in 0..150 {
            xml.push_str(&format!(
                r#"<ZoneGroup Coordinator="RINCON_{:012}" ID="RINCON_{:012}:1">
                    <ZoneGroupMember UUID="RINCON_{:012}" Location="http://192.168.1.{}:1400/xml/device_description.xml" ZoneName="Room {}" />
                </ZoneGroup>"#,
                i, i, i, i % 255 + 1, i
            ));
        }
        xml.push_str("</ZoneGroups></ZoneGroupState>");

        let upnp_event = wrap_in_upnp_event(&xml);

        let result = subscription.parse_zone_group_state(&upnp_event);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Too many ZoneGroup elements"));
    }

    #[test]
    fn test_parse_too_many_members() {
        let subscription = create_test_subscription();

        // Create XML with many members to test the limit
        let mut xml = String::from(
            r#"<ZoneGroupState><ZoneGroups><ZoneGroup Coordinator="RINCON_123456789" ID="RINCON_123456789:1">"#,
        );
        for i in 0..60 {
            xml.push_str(&format!(
                r#"<ZoneGroupMember UUID="RINCON_{:012}" Location="http://192.168.1.{}:1400/xml/device_description.xml" ZoneName="Room {}" />"#,
                i, i % 255 + 1, i
            ));
        }
        xml.push_str("</ZoneGroup></ZoneGroups></ZoneGroupState>");

        let upnp_event = wrap_in_upnp_event(&xml);

        let result = subscription.parse_zone_group_state(&upnp_event);
        // Should handle gracefully and return empty groups due to too many members
        assert!(result.is_ok());
        let groups = result.unwrap();
        assert_eq!(groups.len(), 0); // No valid groups due to member limit exceeded
    }
}

#[cfg(test)]
mod event_generation_tests {
    use super::*;

    #[test]
    fn test_initial_topology_generates_group_formed_events() {
        let subscription = create_test_subscription();
        let zone_group_xml = load_fixture("zone_group_topology_single.xml");
        let upnp_event = wrap_in_upnp_event(&zone_group_xml);

        let changes = subscription.parse_event(&upnp_event).unwrap();

        assert_eq!(changes.len(), 1);
        match &changes[0] {
            StateChange::GroupFormed {
                group_id: _,
                coordinator_id,
                initial_members,
            } => {
                assert_eq!(
                    coordinator_id,
                    &SpeakerId::from_udn("uuid:RINCON_123456789")
                );
                assert_eq!(initial_members.len(), 1);
                assert!(initial_members.contains(&SpeakerId::from_udn("uuid:RINCON_123456789")));
            }
            _ => panic!("Expected GroupFormed event, got: {:?}", changes[0]),
        }
    }

    #[test]
    fn test_multiple_initial_groups_generate_multiple_events() {
        let subscription = create_test_subscription();
        let zone_group_xml = load_fixture("zone_group_topology_multiple.xml");
        let upnp_event = wrap_in_upnp_event(&zone_group_xml);

        let changes = subscription.parse_event(&upnp_event).unwrap();

        // Should generate 2 GroupFormed events (no comprehensive event for initial topology)
        assert_eq!(changes.len(), 2);

        let group_formed_events: Vec<_> = changes
            .iter()
            .filter(|c| matches!(c, StateChange::GroupFormed { .. }))
            .collect();
        assert_eq!(group_formed_events.len(), 2);
    }

    #[test]
    fn test_parsing_error_generates_subscription_error() {
        let subscription = create_test_subscription();
        let malformed_xml = "<invalid>malformed</invalid>";

        let changes = subscription.parse_event(malformed_xml).unwrap();

        assert_eq!(changes.len(), 1);
        match &changes[0] {
            StateChange::SubscriptionError {
                speaker_id,
                service,
                error,
            } => {
                assert_eq!(speaker_id, &subscription.speaker_id());
                assert_eq!(service, &ServiceType::ZoneGroupTopology);
                assert!(error.contains("XML parsing failed"));
            }
            _ => panic!("Expected SubscriptionError event, got: {:?}", changes[0]),
        }
    }

    #[test]
    fn test_empty_topology_generates_no_events() {
        let subscription = create_test_subscription();
        let zone_group_xml = load_fixture("zone_group_topology_empty.xml");
        let upnp_event = wrap_in_upnp_event(&zone_group_xml);

        let changes = subscription.parse_event(&upnp_event).unwrap();

        // Should generate no events for empty topology (initial state)
        assert_eq!(changes.len(), 0);
    }
}

#[cfg(test)]
mod attribute_extraction_tests {
    use super::*;

    #[test]
    fn test_extract_attribute_with_double_quotes() {
        let subscription = create_test_subscription();
        let xml = r#"<ZoneGroup Coordinator="RINCON_123456789" ID="RINCON_123456789:1">"#;

        let coordinator = subscription.extract_attribute(xml, "Coordinator").unwrap();
        assert_eq!(coordinator, "RINCON_123456789");

        let id = subscription.extract_attribute(xml, "ID").unwrap();
        assert_eq!(id, "RINCON_123456789:1");
    }

    #[test]
    fn test_extract_attribute_with_single_quotes() {
        let subscription = create_test_subscription();
        let xml = r#"<ZoneGroup Coordinator='RINCON_123456789' ID='RINCON_123456789:1'>"#;

        let coordinator = subscription.extract_attribute(xml, "Coordinator").unwrap();
        assert_eq!(coordinator, "RINCON_123456789");

        let id = subscription.extract_attribute(xml, "ID").unwrap();
        assert_eq!(id, "RINCON_123456789:1");
    }

    #[test]
    fn test_extract_attribute_with_spaces() {
        let subscription = create_test_subscription();
        let xml = r#"<ZoneGroup Coordinator = "RINCON_123456789" ID = "RINCON_123456789:1">"#;

        let coordinator = subscription.extract_attribute(xml, "Coordinator").unwrap();
        assert_eq!(coordinator, "RINCON_123456789");

        let id = subscription.extract_attribute(xml, "ID").unwrap();
        assert_eq!(id, "RINCON_123456789:1");
    }

    #[test]
    fn test_extract_attribute_with_xml_entities() {
        let subscription = create_test_subscription();
        let xml = r#"<ZoneGroup ZoneName="Living &amp; Dining Room" Location="http://192.168.1.100:1400/xml/device_description.xml">"#;

        let zone_name = subscription.extract_attribute(xml, "ZoneName").unwrap();
        assert_eq!(zone_name, "Living & Dining Room");

        let location = subscription.extract_attribute(xml, "Location").unwrap();
        assert_eq!(
            location,
            "http://192.168.1.100:1400/xml/device_description.xml"
        );
    }

    #[test]
    fn test_extract_missing_attribute() {
        let subscription = create_test_subscription();
        let xml = r#"<ZoneGroup Coordinator="RINCON_123456789">"#;

        let result = subscription.extract_attribute(xml, "NonExistent");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing required attribute 'NonExistent'"));
    }

    #[test]
    fn test_extract_attribute_from_empty_xml() {
        let subscription = create_test_subscription();

        let result = subscription.extract_attribute("", "Coordinator");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Empty XML provided"));
    }

    #[test]
    fn test_extract_attribute_with_empty_name() {
        let subscription = create_test_subscription();
        let xml = r#"<ZoneGroup Coordinator="RINCON_123456789">"#;

        let result = subscription.extract_attribute(xml, "");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Empty attribute name"));
    }
}

#[cfg(test)]
mod xml_entity_decoding_tests {
    use super::*;

    #[test]
    fn test_decode_standard_xml_entities() {
        let subscription = create_test_subscription();

        let encoded = "&lt;ZoneGroupState&gt;&amp;test&amp;&quot;value&quot;&apos;single&apos;";
        let decoded = subscription.decode_xml_entities(encoded);
        assert_eq!(decoded, "<ZoneGroupState>&test&\"value\"'single'");
    }

    #[test]
    fn test_decode_numeric_xml_entities() {
        let subscription = create_test_subscription();

        let encoded = "&#60;test&#62;&#38;data&#38;&#34;quoted&#34;&#39;single&#39;";
        let decoded = subscription.decode_xml_entities(encoded);
        assert_eq!(decoded, "<test>&data&\"quoted\"'single'");
    }

    #[test]
    fn test_decode_cdata_sections() {
        let subscription = create_test_subscription();

        let encoded = "<![CDATA[<ZoneGroupState><ZoneGroups></ZoneGroups></ZoneGroupState>]]>";
        let decoded = subscription.decode_xml_entities(encoded);
        assert_eq!(
            decoded,
            "<ZoneGroupState><ZoneGroups></ZoneGroups></ZoneGroupState>"
        );
    }

    #[test]
    fn test_decode_mixed_entities_and_cdata() {
        let subscription = create_test_subscription();

        let encoded = "&lt;root&gt;<![CDATA[<inner>content</inner>]]>&lt;/root&gt;";
        let decoded = subscription.decode_xml_entities(encoded);
        assert_eq!(decoded, "<root><inner>content</inner></root>");
    }

    #[test]
    fn test_decode_nested_cdata() {
        let subscription = create_test_subscription();

        let encoded = "<![CDATA[First]]> and <![CDATA[Second]]>";
        let decoded = subscription.decode_xml_entities(encoded);
        assert_eq!(decoded, "First and Second");
    }

    #[test]
    fn test_decode_malformed_cdata() {
        let subscription = create_test_subscription();

        // CDATA without closing tag should be left as-is
        let encoded = "<![CDATA[unclosed cdata";
        let decoded = subscription.decode_xml_entities(encoded);
        assert_eq!(decoded, "<![CDATA[unclosed cdata");
    }
}

#[cfg(test)]
mod property_extraction_tests {
    use super::*;

    #[test]
    fn test_extract_property_value_standard_format() {
        let subscription = create_test_subscription();
        let xml = r#"
            <property>
                <ZoneGroupState>&lt;ZoneGroupState&gt;test&lt;/ZoneGroupState&gt;</ZoneGroupState>
            </property>
        "#;

        let result = subscription.extract_property_value(xml, "ZoneGroupState");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "<ZoneGroupState>test</ZoneGroupState>");
    }

    #[test]
    fn test_extract_property_value_with_namespace() {
        let subscription = create_test_subscription();
        let xml = r#"
            <e:property>
                <ZoneGroupState>&lt;ZoneGroupState&gt;test&lt;/ZoneGroupState&gt;</ZoneGroupState>
            </e:property>
        "#;

        let result = subscription.extract_property_value(xml, "ZoneGroupState");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "<ZoneGroupState>test</ZoneGroupState>");
    }

    #[test]
    fn test_extract_property_value_with_cdata() {
        let subscription = create_test_subscription();
        let xml = r#"
            <property>
                <ZoneGroupState><![CDATA[<ZoneGroupState>test</ZoneGroupState>]]></ZoneGroupState>
            </property>
        "#;

        let result = subscription.extract_property_value(xml, "ZoneGroupState");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "<ZoneGroupState>test</ZoneGroupState>");
    }

    #[test]
    fn test_extract_property_value_missing_property() {
        let subscription = create_test_subscription();
        let xml = r#"
            <property>
                <SomeOtherProperty>value</SomeOtherProperty>
            </property>
        "#;

        let result = subscription.extract_property_value(xml, "ZoneGroupState");
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_property_value_empty_content() {
        let subscription = create_test_subscription();
        let xml = r#"
            <property>
                <ZoneGroupState></ZoneGroupState>
            </property>
        "#;

        let result = subscription.extract_property_value(xml, "ZoneGroupState");
        assert!(result.is_none()); // Empty content should return None
    }

    #[test]
    fn test_extract_property_value_multiple_properties() {
        let subscription = create_test_subscription();
        let xml = r#"
            <property>
                <FirstProperty>first</FirstProperty>
            </property>
            <property>
                <ZoneGroupState>target</ZoneGroupState>
            </property>
            <property>
                <ThirdProperty>third</ThirdProperty>
            </property>
        "#;

        let result = subscription.extract_property_value(xml, "ZoneGroupState");
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "target");
    }
}

#[cfg(test)]
mod satellite_parsing_tests {
    use super::*;

    #[test]
    fn test_parse_member_satellites_with_attribute() {
        let subscription = create_test_subscription();
        let member_xml = r#"<ZoneGroupMember UUID="RINCON_123456789" Satellites="RINCON_SAT001,RINCON_SAT002" />"#;

        let satellites = subscription.parse_member_satellites(member_xml).unwrap();
        assert_eq!(satellites.len(), 2);
        assert!(satellites.contains(&SpeakerId::from_udn("uuid:RINCON_SAT001")));
        assert!(satellites.contains(&SpeakerId::from_udn("uuid:RINCON_SAT002")));
    }

    #[test]
    fn test_parse_member_satellites_with_nested_elements() {
        let subscription = create_test_subscription();
        let member_xml = r#"
            <ZoneGroupMember UUID="RINCON_123456789">
                <Satellite UUID="RINCON_SAT001" />
                <Satellite UUID="RINCON_SAT002" />
            </ZoneGroupMember>
        "#;

        let satellites = subscription.parse_member_satellites(member_xml).unwrap();
        assert_eq!(satellites.len(), 2);
        assert!(satellites.contains(&SpeakerId::from_udn("uuid:RINCON_SAT001")));
        assert!(satellites.contains(&SpeakerId::from_udn("uuid:RINCON_SAT002")));
    }

    #[test]
    fn test_parse_member_satellites_no_satellites() {
        let subscription = create_test_subscription();
        let member_xml = r#"<ZoneGroupMember UUID="RINCON_123456789" />"#;

        let satellites = subscription.parse_member_satellites(member_xml).unwrap();
        assert_eq!(satellites.len(), 0);
    }

    #[test]
    fn test_parse_member_satellites_empty_attribute() {
        let subscription = create_test_subscription();
        let member_xml = r#"<ZoneGroupMember UUID="RINCON_123456789" Satellites="" />"#;

        let satellites = subscription.parse_member_satellites(member_xml).unwrap();
        assert_eq!(satellites.len(), 0);
    }

    #[test]
    fn test_parse_member_satellites_whitespace_in_list() {
        let subscription = create_test_subscription();
        let member_xml = r#"<ZoneGroupMember UUID="RINCON_123456789" Satellites=" RINCON_SAT001 , RINCON_SAT002 " />"#;

        let satellites = subscription.parse_member_satellites(member_xml).unwrap();
        assert_eq!(satellites.len(), 2);
        assert!(satellites.contains(&SpeakerId::from_udn("uuid:RINCON_SAT001")));
        assert!(satellites.contains(&SpeakerId::from_udn("uuid:RINCON_SAT002")));
    }

    #[test]
    fn test_parse_member_satellites_single_satellite() {
        let subscription = create_test_subscription();
        let member_xml =
            r#"<ZoneGroupMember UUID="RINCON_123456789" Satellites="RINCON_SAT001" />"#;

        let satellites = subscription.parse_member_satellites(member_xml).unwrap();
        assert_eq!(satellites.len(), 1);
        assert!(satellites.contains(&SpeakerId::from_udn("uuid:RINCON_SAT001")));
    }
}
