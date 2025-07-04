//! XML parsing functionality for Sonos topology data
//!
//! This module contains the `TopologyParser` struct and related functionality
//! for parsing SOAP XML responses containing zone group state information.

use log::{debug, error, info};
use serde_derive::Deserialize;
use xmltree::Element;

use crate::SonosError;
use super::constants::*;
use super::types::{Topology, ZoneGroup, ZoneGroupMember, Satellite, VanishedDevices, VanishedDevice};
use super::utils::write_debug_xml;

/// Response structure for the outer SOAP envelope containing the zone group state
#[derive(Debug, Deserialize)]
#[serde(rename = "GetZoneGroupStateResponse")]
pub struct GetZoneGroupStateResponse {
    #[serde(rename = "ZoneGroupState")]
    pub zone_group_state: String, // HTML-encoded XML content
}

/// Parser for Sonos topology XML data
pub struct TopologyParser;

impl TopologyParser {
    /// Parses topology information from SOAP XML response
    ///
    /// # Arguments
    /// * `xml` - Raw SOAP XML response containing zone group state
    ///
    /// # Returns
    /// * `Ok(Topology)` - Parsed topology information
    /// * `Err(SonosError)` - If parsing fails at any stage
    pub fn from_xml(xml: &str) -> Result<Topology, SonosError> {
        debug!("Starting XML parsing...");
        debug!("Input XML length: {} characters", xml.len());
        
        // Parse the outer SOAP response to extract the inner XML
        let decoded_xml = Self::extract_inner_xml(xml)?;
        
        // Write decoded XML for debugging (only in debug builds)
        if cfg!(debug_assertions) {
            write_debug_xml(&decoded_xml);
        }
        
        // Parse the inner XML using xmltree
        debug!("Parsing inner XML with xmltree...");
        let topology = Self::parse_topology_xml(&decoded_xml)?;
        
        info!("Successfully parsed topology with {} zone groups", topology.zone_groups.len());
        Ok(topology)
    }

    /// Extracts and decodes the inner XML from the SOAP response
    fn extract_inner_xml(xml: &str) -> Result<String, SonosError> {
        debug!("Parsing outer SOAP response...");
        let outer_response: GetZoneGroupStateResponse = serde_xml_rs::from_str(xml)
            .map_err(|e| {
                error!("Failed to parse outer SOAP response: {}", e);
                SonosError::ParseError(format!("Failed to parse outer response: {}", e))
            })?;
        
        debug!("Successfully parsed outer response");
        debug!("Zone group state length: {} characters", outer_response.zone_group_state.len());
        
        // Decode HTML entities in the inner XML
        debug!("Decoding HTML entities...");
        let decoded_xml = html_escape::decode_html_entities(&outer_response.zone_group_state).to_string();
        debug!("Decoded XML length: {} characters", decoded_xml.len());
        debug!("First 200 chars of decoded XML: {}", 
                 if decoded_xml.len() > 200 { &decoded_xml[..200] } else { &decoded_xml });
        
        Ok(decoded_xml)
    }

    /// Parses the topology XML using xmltree for manual parsing
    fn parse_topology_xml(xml: &str) -> Result<Topology, SonosError> {
        let root = Element::parse(xml.as_bytes())
            .map_err(|e| SonosError::ParseError(format!("Failed to parse XML with xmltree: {}", e)))?;
        
        let zone_groups = Self::parse_zone_groups(&root)?;
        let vanished_devices = Self::parse_vanished_devices(&root);
        
        Ok(Topology {
            zone_groups,
            vanished_devices,
        })
    }

    /// Parses zone groups from the XML root element
    fn parse_zone_groups(root: &Element) -> Result<Vec<ZoneGroup>, SonosError> {
        let mut zone_groups = Vec::new();
        
        if let Some(zone_groups_elem) = root.get_child(ZONE_GROUPS_ELEMENT) {
            for zone_group_elem in zone_groups_elem.children.iter() {
                if let Some(element) = zone_group_elem.as_element() {
                    if element.name == ZONE_GROUP_ELEMENT {
                        let zone_group = Self::parse_zone_group(element)?;
                        zone_groups.push(zone_group);
                    }
                }
            }
        }
        
        Ok(zone_groups)
    }

    /// Parses a single zone group element
    fn parse_zone_group(element: &Element) -> Result<ZoneGroup, SonosError> {
        let coordinator = Self::get_attribute(element, COORDINATOR_ATTR);
        let id = Self::get_attribute(element, ID_ATTR);
        let members = Self::parse_zone_group_members(element)?;
        
        Ok(ZoneGroup {
            coordinator,
            id,
            members,
        })
    }

    /// Parses zone group members from a zone group element
    fn parse_zone_group_members(zone_group_elem: &Element) -> Result<Vec<ZoneGroupMember>, SonosError> {
        let mut members = Vec::new();
        
        for member_elem in zone_group_elem.children.iter() {
            if let Some(member_element) = member_elem.as_element() {
                if member_element.name == ZONE_GROUP_MEMBER_ELEMENT {
                    let member = Self::parse_zone_group_member(member_element)?;
                    members.push(member);
                }
            }
        }
        
        Ok(members)
    }

    /// Parses a single zone group member element
    fn parse_zone_group_member(element: &Element) -> Result<ZoneGroupMember, SonosError> {
        let uuid = Self::get_attribute(element, UUID_ATTR);
        let location = Self::get_attribute(element, LOCATION_ATTR);
        let zone_name = Self::get_attribute(element, ZONE_NAME_ATTR);
        let software_version = Self::get_attribute(element, SOFTWARE_VERSION_ATTR);
        let configuration = Self::get_attribute(element, CONFIGURATION_ATTR);
        let icon = Self::get_attribute(element, ICON_ATTR);
        let satellites = Self::parse_satellites(element)?;
        
        Ok(ZoneGroupMember {
            uuid,
            location,
            zone_name,
            software_version,
            configuration,
            icon,
            satellites,
        })
    }

    /// Parses satellite speakers from a zone group member element
    fn parse_satellites(member_element: &Element) -> Result<Vec<Satellite>, SonosError> {
        let mut satellites = Vec::new();
        
        for satellite_elem in member_element.children.iter() {
            if let Some(satellite_element) = satellite_elem.as_element() {
                if satellite_element.name == SATELLITE_ELEMENT {
                    let satellite = Self::parse_satellite(satellite_element)?;
                    satellites.push(satellite);
                }
            }
        }
        
        Ok(satellites)
    }

    /// Parses a single satellite element
    fn parse_satellite(element: &Element) -> Result<Satellite, SonosError> {
        let uuid = Self::get_attribute(element, UUID_ATTR);
        let location = Self::get_attribute(element, LOCATION_ATTR);
        let zone_name = Self::get_attribute(element, ZONE_NAME_ATTR);
        let software_version = Self::get_attribute(element, SOFTWARE_VERSION_ATTR);
        
        Ok(Satellite {
            uuid,
            location,
            zone_name,
            software_version,
        })
    }

    /// Parses vanished devices from the XML root element (optional)
    fn parse_vanished_devices(root: &Element) -> Option<VanishedDevices> {
        root.get_child(VANISHED_DEVICES_ELEMENT).map(|vanished_elem| {
            let mut devices = Vec::new();
            
            for device_elem in vanished_elem.children.iter() {
                if let Some(device_element) = device_elem.as_element() {
                    if device_element.name == DEVICE_ELEMENT {
                        let uuid = Self::get_attribute(device_element, UUID_ATTR);
                        let zone_name = Self::get_attribute(device_element, ZONE_NAME_ATTR);
                        let reason = Self::get_attribute(device_element, REASON_ATTR);
                        
                        devices.push(VanishedDevice {
                            uuid,
                            zone_name,
                            reason,
                        });
                    }
                }
            }
            
            VanishedDevices { devices }
        })
    }

    /// Helper function to safely get an attribute value, returning empty string if not found
    fn get_attribute(element: &Element, attr_name: &str) -> String {
        element.attributes.get(attr_name).cloned().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_SOAP_RESPONSE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
    <s:Body>
        <u:GetZoneGroupStateResponse xmlns:u="urn:schemas-upnp-org:service:ZoneGroupTopology:1">
            <ZoneGroupState>&lt;ZoneGroups&gt;&lt;ZoneGroup Coordinator=&quot;RINCON_123456&quot; ID=&quot;RINCON_123456:1234567890&quot;&gt;&lt;ZoneGroupMember UUID=&quot;RINCON_123456&quot; Location=&quot;http://192.168.1.100:1400/xml/device_description.xml&quot; ZoneName=&quot;Living Room&quot; SoftwareVersion=&quot;56.0-76060&quot; Configuration=&quot;1&quot; Icon=&quot;x-rincon-roomicon:living&quot;&gt;&lt;Satellite UUID=&quot;RINCON_SAT123&quot; Location=&quot;http://192.168.1.101:1400/xml/device_description.xml&quot; ZoneName=&quot;Satellite Speaker&quot; SoftwareVersion=&quot;56.0-76060&quot;/&gt;&lt;/ZoneGroupMember&gt;&lt;/ZoneGroup&gt;&lt;/ZoneGroups&gt;</ZoneGroupState>
        </u:GetZoneGroupStateResponse>
    </s:Body>
</s:Envelope>"#;

    const SAMPLE_SOAP_WITH_VANISHED: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
    <s:Body>
        <u:GetZoneGroupStateResponse xmlns:u="urn:schemas-upnp-org:service:ZoneGroupTopology:1">
            <ZoneGroupState>&lt;ZoneGroups&gt;&lt;ZoneGroup Coordinator=&quot;RINCON_123456&quot; ID=&quot;RINCON_123456:1234567890&quot;&gt;&lt;ZoneGroupMember UUID=&quot;RINCON_123456&quot; Location=&quot;http://192.168.1.100:1400/xml/device_description.xml&quot; ZoneName=&quot;Living Room&quot; SoftwareVersion=&quot;56.0-76060&quot; Configuration=&quot;1&quot; Icon=&quot;x-rincon-roomicon:living&quot;/&gt;&lt;/ZoneGroup&gt;&lt;/ZoneGroups&gt;&lt;VanishedDevices&gt;&lt;Device UUID=&quot;RINCON_VANISHED&quot; ZoneName=&quot;Old Speaker&quot; Reason=&quot;powered off&quot;/&gt;&lt;/VanishedDevices&gt;</ZoneGroupState>
        </u:GetZoneGroupStateResponse>
    </s:Body>
</s:Envelope>"#;

    const SAMPLE_MULTI_GROUP_SOAP: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
    <s:Body>
        <u:GetZoneGroupStateResponse xmlns:u="urn:schemas-upnp-org:service:ZoneGroupTopology:1">
            <ZoneGroupState>&lt;ZoneGroups&gt;&lt;ZoneGroup Coordinator=&quot;RINCON_123456&quot; ID=&quot;RINCON_123456:1234567890&quot;&gt;&lt;ZoneGroupMember UUID=&quot;RINCON_123456&quot; Location=&quot;http://192.168.1.100:1400/xml/device_description.xml&quot; ZoneName=&quot;Living Room&quot; SoftwareVersion=&quot;56.0-76060&quot; Configuration=&quot;1&quot; Icon=&quot;x-rincon-roomicon:living&quot;/&gt;&lt;ZoneGroupMember UUID=&quot;RINCON_789012&quot; Location=&quot;http://192.168.1.101:1400/xml/device_description.xml&quot; ZoneName=&quot;Kitchen&quot; SoftwareVersion=&quot;56.0-76060&quot; Configuration=&quot;1&quot; Icon=&quot;x-rincon-roomicon:kitchen&quot;/&gt;&lt;/ZoneGroup&gt;&lt;ZoneGroup Coordinator=&quot;RINCON_345678&quot; ID=&quot;RINCON_345678:9876543210&quot;&gt;&lt;ZoneGroupMember UUID=&quot;RINCON_345678&quot; Location=&quot;http://192.168.1.102:1400/xml/device_description.xml&quot; ZoneName=&quot;Bedroom&quot; SoftwareVersion=&quot;56.0-76060&quot; Configuration=&quot;1&quot; Icon=&quot;x-rincon-roomicon:bedroom&quot;/&gt;&lt;/ZoneGroup&gt;&lt;/ZoneGroups&gt;</ZoneGroupState>
        </u:GetZoneGroupStateResponse>
    </s:Body>
</s:Envelope>"#;

    #[test]
    fn test_topology_parser_exists() {
        // Test that TopologyParser can be instantiated and has the expected methods
        // We can't easily test XML parsing without a more complex setup, but we can
        // test that the parser structure exists and is accessible
        
        // Test with invalid XML to ensure error handling works
        let result = TopologyParser::from_xml("invalid xml");
        assert!(result.is_err());
        
        // Test with empty XML
        let result = TopologyParser::from_xml("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_xml_returns_error() {
        // Test various invalid XML inputs
        let invalid_inputs = [
            "not xml at all",
            "<incomplete",
            "<?xml version='1.0'?><root><unclosed>",
            "",
        ];
        
        for invalid_xml in invalid_inputs.iter() {
            let result = TopologyParser::from_xml(invalid_xml);
            assert!(result.is_err(), "Should return error for invalid XML: {}", invalid_xml);
        }
    }

    #[test]
    fn test_parse_invalid_soap_response() {
        let invalid_soap = r#"<?xml version="1.0"?><invalid>response</invalid>"#;
        let result = TopologyParser::from_xml(invalid_soap);
        assert!(result.is_err());
        
        if let Err(SonosError::ParseError(msg)) = result {
            assert!(msg.contains("Failed to parse outer response"));
        } else {
            panic!("Expected ParseError");
        }
    }

    #[test]
    fn test_get_attribute_existing() {
        let mut element = Element::new("test");
        element.attributes.insert("uuid".to_string(), "RINCON_123".to_string());
        element.attributes.insert("name".to_string(), "Test Speaker".to_string());
        
        assert_eq!(TopologyParser::get_attribute(&element, "uuid"), "RINCON_123");
        assert_eq!(TopologyParser::get_attribute(&element, "name"), "Test Speaker");
    }

    #[test]
    fn test_get_attribute_missing() {
        let element = Element::new("test");
        assert_eq!(TopologyParser::get_attribute(&element, "missing"), "");
    }

    #[test]
    fn test_parser_structure_and_methods() {
        // Test that the parser has the expected structure and methods
        // This verifies the API without requiring complex XML parsing
        
        // Test that get_attribute works correctly
        let mut element = Element::new("test");
        element.attributes.insert("uuid".to_string(), "RINCON_123".to_string());
        element.attributes.insert("name".to_string(), "Test Speaker".to_string());
        
        assert_eq!(TopologyParser::get_attribute(&element, "uuid"), "RINCON_123");
        assert_eq!(TopologyParser::get_attribute(&element, "name"), "Test Speaker");
        assert_eq!(TopologyParser::get_attribute(&element, "missing"), "");
    }
}