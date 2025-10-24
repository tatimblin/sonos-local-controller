use super::types::{XmlZoneGroupData, XmlZoneGroupMember, XmlZoneGroups};
use crate::xml::{
    error::{XmlParseError, XmlParseResult},
    parser::XmlParser,
    types::XmlProperty,
};

/// ZoneGroupTopology service-specific parsing functions
pub struct ZoneGroupTopologyParser;

impl ZoneGroupTopologyParser {
    /// Parse zone groups using serde
    pub fn parse_zone_groups_serde(xml: &str) -> XmlParseResult<Vec<XmlZoneGroupData>> {
        let decoded_xml = XmlParser::decode_entities(xml);

        match serde_xml_rs::from_str::<XmlZoneGroups>(&decoded_xml) {
            Ok(zone_groups) => Ok(zone_groups.zone_groups),
            Err(e) => Err(XmlParseError::SyntaxError(format!(
                "Serde XML error: {}",
                e
            ))),
        }
    }

    /// Parse zone group state from a UPnP event XML using serde
    pub fn parse_zone_group_state_from_upnp_event(xml_content: &str) -> XmlParseResult<Vec<XmlZoneGroupData>> {
        // Try to parse as property first
        if let Ok(property) = XmlParser::parse_property_serde(xml_content) {
            if let Some(zone_group_state_xml) = property.zone_group_state {
                if zone_group_state_xml.trim().is_empty() {
                    return Ok(Vec::new());
                }
                
                // Decode XML entities and parse zone groups
                return Self::parse_zone_groups_serde(&zone_group_state_xml);
            }
        }
        
        // Fallback: try to parse directly as zone groups
        Self::parse_zone_groups_serde(xml_content)
    }

    /// Parse zone group state using serde (simplified)
    pub fn parse_zone_group_state(xml_content: &str) -> XmlParseResult<Vec<XmlZoneGroupData>> {
        Self::parse_zone_groups_serde(xml_content)
    }

    /// Parse a single zone group member using serde
    pub fn parse_zone_group_member(member_xml: &str) -> XmlParseResult<XmlZoneGroupMember> {
        match serde_xml_rs::from_str::<XmlZoneGroupMember>(member_xml) {
            Ok(member) => Ok(member),
            Err(e) => Err(XmlParseError::SyntaxError(format!("Serde XML error: {}", e))),
        }
    }
}

/// ZoneGroupTopology service-specific extensions for the XML parser
impl<'a> XmlParser<'a> {
    /// Parse zone group state from a UPnP event XML using serde
    pub fn parse_zone_group_state_from_upnp_event(&mut self) -> XmlParseResult<Vec<XmlZoneGroupData>> {
        let xml_content = std::str::from_utf8(self.reader.get_ref())
            .map_err(|e| XmlParseError::SyntaxError(format!("Invalid UTF-8: {}", e)))?;
        
        ZoneGroupTopologyParser::parse_zone_group_state_from_upnp_event(xml_content)
    }

    /// Parse zone group state using serde (simplified)
    pub fn parse_zone_group_state(&mut self) -> XmlParseResult<Vec<XmlZoneGroupData>> {
        let xml_content = std::str::from_utf8(self.reader.get_ref())
            .map_err(|e| XmlParseError::SyntaxError(format!("Invalid UTF-8: {}", e)))?;
        
        ZoneGroupTopologyParser::parse_zone_group_state(xml_content)
    }

    /// Parse a single zone group member using serde
    pub fn parse_zone_group_member(&mut self, member_xml: &str) -> XmlParseResult<XmlZoneGroupMember> {
        ZoneGroupTopologyParser::parse_zone_group_member(member_xml)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_zone_group_state_single_group() {
        let xml = r#"
            <ZoneGroups>
                <ZoneGroup Coordinator="RINCON_123456789" ID="RINCON_123456789:1">
                    <ZoneGroupMember UUID="RINCON_123456789" Location="http://192.168.1.100:1400/xml/device_description.xml" ZoneName="Living Room" />
                </ZoneGroup>
            </ZoneGroups>
        "#;
        let result = ZoneGroupTopologyParser::parse_zone_group_state(xml).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].coordinator, "RINCON_123456789");
        assert_eq!(result[0].members.len(), 1);
        assert_eq!(result[0].members[0].uuid, "RINCON_123456789");
    }

    #[test]
    fn test_parse_zone_group_state_multiple_groups() {
        let xml = r#"
            <ZoneGroups>
                <ZoneGroup Coordinator="RINCON_123456789" ID="RINCON_123456789:1">
                    <ZoneGroupMember UUID="RINCON_123456789" Location="http://192.168.1.100:1400/xml/device_description.xml" ZoneName="Living Room" />
                </ZoneGroup>
                <ZoneGroup Coordinator="RINCON_987654321" ID="RINCON_987654321:1">
                    <ZoneGroupMember UUID="RINCON_987654321" Location="http://192.168.1.101:1400/xml/device_description.xml" ZoneName="Kitchen" />
                </ZoneGroup>
            </ZoneGroups>
        "#;
        let result = ZoneGroupTopologyParser::parse_zone_group_state(xml).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].coordinator, "RINCON_123456789");
        assert_eq!(result[1].coordinator, "RINCON_987654321");
    }

    #[test]
    fn test_parse_zone_group_state_with_satellites() {
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
        let result = ZoneGroupTopologyParser::parse_zone_group_state(xml).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].coordinator, "RINCON_123456789");
        assert_eq!(result[0].members.len(), 1);
        assert_eq!(result[0].members[0].uuid, "RINCON_123456789");
        assert_eq!(result[0].members[0].satellites().len(), 2);
        assert_eq!(result[0].members[0].satellites()[0], "RINCON_111111111");
        assert_eq!(result[0].members[0].satellites()[1], "RINCON_222222222");
    }

    #[test]
    fn test_parse_zone_group_member() {
        let member_xml = r#"<ZoneGroupMember UUID="RINCON_123456789" Location="http://192.168.1.100:1400/xml/device_description.xml" ZoneName="Living Room" />"#;
        let result = ZoneGroupTopologyParser::parse_zone_group_member(member_xml).unwrap();
        assert_eq!(result.uuid, "RINCON_123456789");
        assert_eq!(result.satellites().len(), 0);
    }

    #[test]
    fn test_parse_zone_group_state_empty() {
        let xml = r#"<ZoneGroups></ZoneGroups>"#;
        let result = ZoneGroupTopologyParser::parse_zone_group_state(xml).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_zone_group_state_multiple_members() {
        let xml = r#"
            <ZoneGroups>
                <ZoneGroup Coordinator="RINCON_123456789" ID="RINCON_123456789:1">
                    <ZoneGroupMember UUID="RINCON_123456789" Location="http://192.168.1.100:1400/xml/device_description.xml" ZoneName="Living Room" />
                    <ZoneGroupMember UUID="RINCON_987654321" Location="http://192.168.1.101:1400/xml/device_description.xml" ZoneName="Kitchen" />
                </ZoneGroup>
            </ZoneGroups>
        "#;
        let result = ZoneGroupTopologyParser::parse_zone_group_state(xml).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].coordinator, "RINCON_123456789");
        assert_eq!(result[0].members.len(), 2);
        assert_eq!(result[0].members[0].uuid, "RINCON_123456789");
        assert_eq!(result[0].members[1].uuid, "RINCON_987654321");
    }
}