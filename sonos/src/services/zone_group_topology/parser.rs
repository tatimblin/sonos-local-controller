use crate::error::{Result, SonosError};
use serde::Deserialize;

/// Zone Group Topology parser using serde
pub struct ZoneGroupTopologyParser {
    data: ZoneGroupTopologyData,
}

/// Zone group information
#[derive(Debug, Clone)]
pub struct ZoneGroupInfo {
    pub coordinator: String,
    pub id: String,
    pub members: Vec<ZoneGroupMemberInfo>,
}

/// Zone group member information
#[derive(Debug, Clone)]
pub struct ZoneGroupMemberInfo {
    pub uuid: String,
    pub location: String,
    pub zone_name: String,
    pub satellites: Vec<String>,
}

/// Main data structure for ZoneGroupTopology events
#[derive(Debug, Deserialize)]
struct ZoneGroupTopologyData {
    #[serde(rename = "property")]
    property: ZoneGroupTopologyProperty,
}

/// Property containing ZoneGroupState
#[derive(Debug, Deserialize)]
struct ZoneGroupTopologyProperty {
    #[serde(rename = "ZoneGroupState", default)]
    zone_group_state: Option<String>,
}

/// Zone group state wrapper
#[derive(Debug, Deserialize)]
struct ZoneGroupStateWrapper {
    #[serde(rename = "ZoneGroups")]
    zone_groups: ZoneGroupsWrapper,
}

/// Zone groups container
#[derive(Debug, Deserialize)]
struct ZoneGroupsWrapper {
    #[serde(rename = "ZoneGroup", default)]
    zone_groups: Vec<ZoneGroupSerde>,
}

/// Individual zone group
#[derive(Debug, Deserialize)]
struct ZoneGroupSerde {
    #[serde(rename = "@Coordinator")]
    coordinator: Option<String>,
    #[serde(rename = "@ID")]
    id: Option<String>,
    #[serde(rename = "ZoneGroupMember", default)]
    members: Vec<ZoneGroupMemberSerde>,
}

/// Zone group member
#[derive(Debug, Deserialize)]
struct ZoneGroupMemberSerde {
    #[serde(rename = "@UUID", default)]
    uuid: Option<String>,
    #[serde(rename = "@Location", default)]
    location: Option<String>,
    #[serde(rename = "@ZoneName", default)]
    zone_name: Option<String>,
    #[serde(rename = "@Satellites", default)]
    satellites_attr: Option<String>,
    #[serde(rename = "Satellite", default)]
    satellite_elements: Vec<SatelliteSerde>,
}

/// Satellite speaker
#[derive(Debug, Deserialize)]
struct SatelliteSerde {
    #[serde(rename = "@UUID", default)]
    uuid: Option<String>,
}

impl ZoneGroupTopologyParser {
    /// Create a new parser from XML string
    pub fn from_xml(xml: &str) -> Result<Self> {
        let cleaned_xml = xml
            .replace("e:propertyset", "propertyset")
            .replace("e:property", "property");

        let data = serde_xml_rs::from_str(&cleaned_xml)
            .map_err(|e| SonosError::ParseError(format!("PropertySet parse error: {}", e)))?;

        Ok(Self { data })
    }

    /// Get the zone groups from the parsed XML
    pub fn zone_groups(&self) -> Option<Vec<ZoneGroupInfo>> {
        // Check ZoneGroupState property
        let zone_group_state_xml = self.data.property.zone_group_state.as_ref()?;
        
        if zone_group_state_xml.trim().is_empty() {
            return Some(Vec::new());
        }

        // Decode XML entities
        let decoded_xml = decode_html_entities(zone_group_state_xml);
        
        // Try to parse as ZoneGroupState (with wrapper) - this is the most common format
        if let Ok(zone_group_state) = serde_xml_rs::from_str::<ZoneGroupStateWrapper>(&decoded_xml) {
            return Some(convert_to_zone_group_info(zone_group_state.zone_groups.zone_groups));
        }
        
        // Fallback: try to parse directly as ZoneGroups
        if let Ok(zone_groups) = serde_xml_rs::from_str::<ZoneGroupsWrapper>(&decoded_xml) {
            return Some(convert_to_zone_group_info(zone_groups.zone_groups));
        }

        // If parsing fails, return empty list
        Some(Vec::new())
    }
}

/// Convert serde structures to domain models
fn convert_to_zone_group_info(zone_groups: Vec<ZoneGroupSerde>) -> Vec<ZoneGroupInfo> {
    zone_groups
        .into_iter()
        .filter_map(|group| {
            let coordinator = group.coordinator?;
            let coordinator_clone = coordinator.clone();
            Some(ZoneGroupInfo {
                coordinator,
                id: group.id.unwrap_or_else(|| format!("{}:1", coordinator_clone)),
                members: group
                    .members
                    .into_iter()
                    .map(|member| {
                        let mut satellites = Vec::new();
                        
                        // Add satellites from attribute (comma-separated)
                        if let Some(ref attr) = member.satellites_attr {
                            for uuid in attr.split(',') {
                                let uuid = uuid.trim();
                                if !uuid.is_empty() {
                                    satellites.push(uuid.to_string());
                                }
                            }
                        }
                        
                        // Add satellites from nested elements
                        for satellite in member.satellite_elements {
                            if let Some(uuid) = satellite.uuid {
                                satellites.push(uuid);
                            }
                        }
                        
                        ZoneGroupMemberInfo {
                            uuid: member.uuid.unwrap_or_default(),
                            location: member.location.unwrap_or_default(),
                            zone_name: member.zone_name.unwrap_or_default(),
                            satellites,
                        }
                    })
                    .collect(),
            })
        })
        .collect()
}

/// Decode HTML entities in XML content
fn decode_html_entities(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

// Legacy functions for backward compatibility
/// Parse zone group state from a UPnP event XML (legacy function for backward compatibility)
pub fn parse_zone_group_state_from_upnp_event(xml_content: &str) -> Result<Vec<super::types::XmlZoneGroupData>> {
    let parser = ZoneGroupTopologyParser::from_xml(xml_content)?;
    
    if let Some(zone_groups) = parser.zone_groups() {
        // Convert to legacy format
        let legacy_groups = zone_groups
            .into_iter()
            .map(|group| super::types::XmlZoneGroupData {
                coordinator: group.coordinator,
                members: group
                    .members
                    .into_iter()
                    .map(|member| super::types::XmlZoneGroupMember {
                        uuid: member.uuid,
                        satellites_attr: if member.satellites.is_empty() {
                            None
                        } else {
                            Some(member.satellites.join(","))
                        },
                        satellite_elements: member
                            .satellites
                            .into_iter()
                            .map(|uuid| super::types::XmlSatellite { uuid })
                            .collect(),
                    })
                    .collect(),
            })
            .collect();
        
        Ok(legacy_groups)
    } else {
        Ok(Vec::new())
    }
}

/// Parse a single zone group member (legacy function for backward compatibility)
pub fn parse_zone_group_member(member_xml: &str) -> Result<super::types::XmlZoneGroupMember> {
    match serde_xml_rs::from_str::<super::types::XmlZoneGroupMember>(member_xml) {
        Ok(member) => Ok(member),
        Err(e) => Err(SonosError::ParseError(format!("Serde XML error: {}", e))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_from_xml_with_zone_group_state() {
        let xml = r#"
            <propertyset>
                <property>
                    <ZoneGroupState>&lt;ZoneGroups&gt;&lt;ZoneGroup Coordinator="RINCON_123456789" ID="RINCON_123456789:1"&gt;&lt;ZoneGroupMember UUID="RINCON_123456789" Location="http://192.168.1.100:1400/xml/device_description.xml" ZoneName="Living Room" /&gt;&lt;/ZoneGroup&gt;&lt;/ZoneGroups&gt;</ZoneGroupState>
                </property>
            </propertyset>
        "#;

        let parser = ZoneGroupTopologyParser::from_xml(xml).unwrap();
        let zone_groups = parser.zone_groups().unwrap();

        assert_eq!(zone_groups.len(), 1);
        assert_eq!(zone_groups[0].coordinator, "RINCON_123456789");
        assert_eq!(zone_groups[0].id, "RINCON_123456789:1");
        assert_eq!(zone_groups[0].members.len(), 1);
        assert_eq!(zone_groups[0].members[0].uuid, "RINCON_123456789");
        assert_eq!(zone_groups[0].members[0].satellites.len(), 0);
    }

    #[test]
    fn test_parser_from_xml_with_multiple_groups() {
        let xml = r#"
            <propertyset>
                <property>
                    <ZoneGroupState>&lt;ZoneGroups&gt;&lt;ZoneGroup Coordinator="RINCON_123456789" ID="RINCON_123456789:1"&gt;&lt;ZoneGroupMember UUID="RINCON_123456789" Location="http://192.168.1.100:1400/xml/device_description.xml" ZoneName="Living Room" /&gt;&lt;/ZoneGroup&gt;&lt;ZoneGroup Coordinator="RINCON_987654321" ID="RINCON_987654321:1"&gt;&lt;ZoneGroupMember UUID="RINCON_987654321" Location="http://192.168.1.101:1400/xml/device_description.xml" ZoneName="Kitchen" /&gt;&lt;/ZoneGroup&gt;&lt;/ZoneGroups&gt;</ZoneGroupState>
                </property>
            </propertyset>
        "#;

        let parser = ZoneGroupTopologyParser::from_xml(xml).unwrap();
        let zone_groups = parser.zone_groups().unwrap();

        assert_eq!(zone_groups.len(), 2);
        assert_eq!(zone_groups[0].coordinator, "RINCON_123456789");
        assert_eq!(zone_groups[1].coordinator, "RINCON_987654321");
        assert_eq!(zone_groups[0].members[0].zone_name, "Living Room");
        assert_eq!(zone_groups[1].members[0].zone_name, "Kitchen");
    }

    #[test]
    fn test_parser_from_xml_with_satellites() {
        let xml = r#"
            <propertyset>
                <property>
                    <ZoneGroupState>&lt;ZoneGroups&gt;&lt;ZoneGroup Coordinator="RINCON_123456789" ID="RINCON_123456789:1"&gt;&lt;ZoneGroupMember UUID="RINCON_123456789" Location="http://192.168.1.100:1400/xml/device_description.xml" ZoneName="Living Room"&gt;&lt;Satellite UUID="RINCON_111111111" /&gt;&lt;Satellite UUID="RINCON_222222222" /&gt;&lt;/ZoneGroupMember&gt;&lt;/ZoneGroup&gt;&lt;/ZoneGroups&gt;</ZoneGroupState>
                </property>
            </propertyset>
        "#;

        let parser = ZoneGroupTopologyParser::from_xml(xml).unwrap();
        let zone_groups = parser.zone_groups().unwrap();

        assert_eq!(zone_groups.len(), 1);
        assert_eq!(zone_groups[0].members.len(), 1);
        assert_eq!(zone_groups[0].members[0].satellites.len(), 2);
        assert_eq!(zone_groups[0].members[0].satellites[0], "RINCON_111111111");
        assert_eq!(zone_groups[0].members[0].satellites[1], "RINCON_222222222");
    }

    #[test]
    fn test_parser_from_xml_with_satellites_attribute() {
        let xml = r#"
            <propertyset>
                <property>
                    <ZoneGroupState>&lt;ZoneGroups&gt;&lt;ZoneGroup Coordinator="RINCON_123456789" ID="RINCON_123456789:1"&gt;&lt;ZoneGroupMember UUID="RINCON_123456789" Location="http://192.168.1.100:1400/xml/device_description.xml" ZoneName="Living Room" Satellites="RINCON_111111111,RINCON_222222222" /&gt;&lt;/ZoneGroup&gt;&lt;/ZoneGroups&gt;</ZoneGroupState>
                </property>
            </propertyset>
        "#;

        let parser = ZoneGroupTopologyParser::from_xml(xml).unwrap();
        let zone_groups = parser.zone_groups().unwrap();

        assert_eq!(zone_groups.len(), 1);
        assert_eq!(zone_groups[0].members.len(), 1);
        assert_eq!(zone_groups[0].members[0].satellites.len(), 2);
        assert_eq!(zone_groups[0].members[0].satellites[0], "RINCON_111111111");
        assert_eq!(zone_groups[0].members[0].satellites[1], "RINCON_222222222");
    }

    #[test]
    fn test_parser_from_xml_with_namespace_prefixes() {
        let xml = r#"<e:propertyset xmlns:e="urn:schemas-upnp-org:event-1-0">
            <e:property>
                <ZoneGroupState>&lt;ZoneGroups&gt;&lt;ZoneGroup Coordinator="RINCON_123456789" ID="RINCON_123456789:1"&gt;&lt;ZoneGroupMember UUID="RINCON_123456789" Location="http://192.168.1.100:1400/xml/device_description.xml" ZoneName="Living Room" /&gt;&lt;/ZoneGroup&gt;&lt;/ZoneGroups&gt;</ZoneGroupState>
            </e:property>
        </e:propertyset>"#;

        let parser = ZoneGroupTopologyParser::from_xml(xml).unwrap();
        let zone_groups = parser.zone_groups().unwrap();

        assert_eq!(zone_groups.len(), 1);
        assert_eq!(zone_groups[0].coordinator, "RINCON_123456789");
    }

    #[test]
    fn test_parser_from_xml_empty_zone_group_state() {
        let xml = r#"
            <propertyset>
                <property>
                    <ZoneGroupState></ZoneGroupState>
                </property>
            </propertyset>
        "#;

        let parser = ZoneGroupTopologyParser::from_xml(xml).unwrap();
        let zone_groups = parser.zone_groups().unwrap();

        assert_eq!(zone_groups.len(), 0);
    }

    #[test]
    fn test_parser_from_xml_multiple_members_in_group() {
        let xml = r#"
            <propertyset>
                <property>
                    <ZoneGroupState>&lt;ZoneGroups&gt;&lt;ZoneGroup Coordinator="RINCON_123456789" ID="RINCON_123456789:1"&gt;&lt;ZoneGroupMember UUID="RINCON_123456789" Location="http://192.168.1.100:1400/xml/device_description.xml" ZoneName="Living Room" /&gt;&lt;ZoneGroupMember UUID="RINCON_987654321" Location="http://192.168.1.101:1400/xml/device_description.xml" ZoneName="Kitchen" /&gt;&lt;/ZoneGroup&gt;&lt;/ZoneGroups&gt;</ZoneGroupState>
                </property>
            </propertyset>
        "#;

        let parser = ZoneGroupTopologyParser::from_xml(xml).unwrap();
        let zone_groups = parser.zone_groups().unwrap();

        assert_eq!(zone_groups.len(), 1);
        assert_eq!(zone_groups[0].coordinator, "RINCON_123456789");
        assert_eq!(zone_groups[0].members.len(), 2);
        assert_eq!(zone_groups[0].members[0].uuid, "RINCON_123456789");
        assert_eq!(zone_groups[0].members[1].uuid, "RINCON_987654321");
        assert_eq!(zone_groups[0].members[0].zone_name, "Living Room");
        assert_eq!(zone_groups[0].members[1].zone_name, "Kitchen");
    }

    #[test]
    fn test_parser_from_xml_real_sonos_xml() {
        // Real XML from Sonos device with complex structure
        let xml = r#"<e:propertyset xmlns:e="urn:schemas-upnp-org:event-1-0"><e:property><ZoneGroupState>&lt;ZoneGroupState&gt;&lt;ZoneGroups&gt;&lt;ZoneGroup Coordinator="RINCON_C43875CA135801400" ID="RINCON_C43875CA135801400:2858411400"&gt;&lt;ZoneGroupMember UUID="RINCON_C43875CA135801400" Location="http://192.168.4.65:1400/xml/device_description.xml" ZoneName="Roam 2" Icon="" Configuration="1" SoftwareVersion="85.0-64200" /&gt;&lt;/ZoneGroup&gt;&lt;ZoneGroup Coordinator="RINCON_804AF2AA2FA201400" ID="RINCON_804AF2AA2FA201400:1331296863"&gt;&lt;ZoneGroupMember UUID="RINCON_804AF2AA2FA201400" Location="http://192.168.4.69:1400/xml/device_description.xml" ZoneName="Living Room" Icon="" Configuration="1" SoftwareVersion="85.0-65020" /&gt;&lt;/ZoneGroup&gt;&lt;/ZoneGroups&gt;&lt;/ZoneGroupState&gt;</ZoneGroupState></e:property></e:propertyset>"#;

        let parser = ZoneGroupTopologyParser::from_xml(xml).unwrap();
        let zone_groups = parser.zone_groups().unwrap();

        assert_eq!(zone_groups.len(), 2);
        assert_eq!(zone_groups[0].coordinator, "RINCON_C43875CA135801400");
        assert_eq!(zone_groups[1].coordinator, "RINCON_804AF2AA2FA201400");
        assert_eq!(zone_groups[0].members[0].zone_name, "Roam 2");
        assert_eq!(zone_groups[1].members[0].zone_name, "Living Room");
    }

    #[test]
    fn test_parser_from_xml_with_zone_group_state_wrapper() {
        let xml = r#"
            <propertyset>
                <property>
                    <ZoneGroupState>&lt;ZoneGroupState&gt;&lt;ZoneGroups&gt;&lt;ZoneGroup Coordinator="RINCON_123456789" ID="RINCON_123456789:1"&gt;&lt;ZoneGroupMember UUID="RINCON_123456789" Location="http://192.168.1.100:1400/xml/device_description.xml" ZoneName="Living Room" /&gt;&lt;/ZoneGroup&gt;&lt;/ZoneGroups&gt;&lt;/ZoneGroupState&gt;</ZoneGroupState>
                </property>
            </propertyset>
        "#;

        let parser = ZoneGroupTopologyParser::from_xml(xml).unwrap();
        let zone_groups = parser.zone_groups().unwrap();

        assert_eq!(zone_groups.len(), 1);
        assert_eq!(zone_groups[0].coordinator, "RINCON_123456789");
        assert_eq!(zone_groups[0].members[0].zone_name, "Living Room");
    }

    #[test]
    fn test_parser_invalid_xml() {
        let invalid_xml = r#"<invalid>xml</invalid>"#;
        let result = ZoneGroupTopologyParser::from_xml(invalid_xml);
        assert!(result.is_err());
    }

    // Legacy function tests for backward compatibility
    #[test]
    fn test_parse_zone_group_state_from_upnp_event_legacy() {
        let xml = r#"
            <propertyset>
                <property>
                    <ZoneGroupState>&lt;ZoneGroups&gt;&lt;ZoneGroup Coordinator="RINCON_123456789" ID="RINCON_123456789:1"&gt;&lt;ZoneGroupMember UUID="RINCON_123456789" Location="http://192.168.1.100:1400/xml/device_description.xml" ZoneName="Living Room" /&gt;&lt;/ZoneGroup&gt;&lt;/ZoneGroups&gt;</ZoneGroupState>
                </property>
            </propertyset>
        "#;



        let result = parse_zone_group_state_from_upnp_event(xml).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].coordinator, "RINCON_123456789");
        assert_eq!(result[0].members.len(), 1);
        assert_eq!(result[0].members[0].uuid, "RINCON_123456789");
    }

    #[test]
    fn test_parse_zone_group_member_legacy() {
        let member_xml = r#"<ZoneGroupMember UUID="RINCON_123456789" Location="http://192.168.1.100:1400/xml/device_description.xml" ZoneName="Living Room" />"#;
        let result = parse_zone_group_member(member_xml).unwrap();
        assert_eq!(result.uuid, "RINCON_123456789");
        assert_eq!(result.satellites().len(), 0);
    }

    #[test]
    fn test_zone_group_topology_parser_legacy_struct() {
        let xml = r#"
            <propertyset>
                <property>
                    <ZoneGroupState>&lt;ZoneGroups&gt;&lt;ZoneGroup Coordinator="RINCON_123456789" ID="RINCON_123456789:1"&gt;&lt;ZoneGroupMember UUID="RINCON_123456789" Location="http://192.168.1.100:1400/xml/device_description.xml" ZoneName="Living Room" /&gt;&lt;/ZoneGroup&gt;&lt;/ZoneGroups&gt;</ZoneGroupState>
                </property>
            </propertyset>
        "#;

        let result = parse_zone_group_state_from_upnp_event(xml).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].coordinator, "RINCON_123456789");
    }

    #[test]
    fn test_parse_zone_group_state_direct_zone_groups() {
        let xml = r#"
            <ZoneGroups>
                <ZoneGroup Coordinator="RINCON_123456789" ID="RINCON_123456789:1">
                    <ZoneGroupMember UUID="RINCON_123456789" Location="http://192.168.1.100:1400/xml/device_description.xml" ZoneName="Living Room" />
                </ZoneGroup>
            </ZoneGroups>
        "#;

        let result = parse_zone_group_state_from_upnp_event(xml).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].coordinator, "RINCON_123456789");
        assert_eq!(result[0].members.len(), 1);
        assert_eq!(result[0].members[0].uuid, "RINCON_123456789");
    }

    #[test]
    fn test_parse_zone_group_state_empty_legacy() {
        let xml = r#"<ZoneGroups></ZoneGroups>"#;
        let result = parse_zone_group_state_from_upnp_event(xml).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_zone_group_state_with_satellites_legacy() {
        let xml = r#"
            <ZoneGroups>
                <ZoneGroup Coordinator="RINCON_123456789" ID="RINCON_123456789:1">
                    <ZoneGroupMember UUID="RINCON_123456789" Location="http://192.168.1.100:1400/xml/device_description.xml" ZoneName="Living Room">
                        <Satellite UUID="RINCON_111111111" Location="http://192.168.1.102:1400/xml/device_description.xml" ZoneName="Living Room Left" />
                        <Satellite UUID="RINCON_222222222" Location="http://192.168.1.103:1400/xml/device_description.xml" ZoneName="Living Room Right" />
                    </ZoneGroupMember>
                </ZoneGroup>
            </ZoneGroups>
        "#;

        let result = parse_zone_group_state_from_upnp_event(xml).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].coordinator, "RINCON_123456789");
        assert_eq!(result[0].members.len(), 1);
        assert_eq!(result[0].members[0].uuid, "RINCON_123456789");
        assert_eq!(result[0].members[0].satellites().len(), 2);
        assert_eq!(result[0].members[0].satellites()[0], "RINCON_111111111");
        assert_eq!(result[0].members[0].satellites()[1], "RINCON_222222222");
    }
}
