use super::{
    error::XmlParseResult,
    parser::XmlParser,
    types::{XmlZoneGroupData, XmlZoneGroupMember},
};
use quick_xml::events::Event;

/// ZoneGroupTopology service-specific extensions for the XML parser
impl<'a> XmlParser<'a> {
    /// Parse zone group state from a UPnP event XML in one call
    ///
    /// This method handles the complete process:
    /// 1. Extract ZoneGroupState property from UPnP event XML
    /// 2. Decode XML entities and CDATA
    /// 3. Parse the zone group structure
    ///
    /// Input XML structure:
    /// ```xml
    /// <property>
    ///   <ZoneGroupState>&lt;ZoneGroups&gt;...&lt;/ZoneGroups&gt;</ZoneGroupState>
    /// </property>
    /// ```
    pub fn parse_zone_group_state_from_upnp_event(&mut self) -> XmlParseResult<Vec<XmlZoneGroupData>> {
        // First validate that this looks like a UPnP event structure
        let xml_content = std::str::from_utf8(self.reader.get_ref())
            .map_err(|e| crate::xml::XmlParseError::SyntaxError(format!("Invalid UTF-8: {}", e)))?;
        
        if !Self::validate_upnp_event_structure(xml_content)? {
            return Err(crate::xml::XmlParseError::InvalidStructure(
                "Invalid UPnP event structure".to_string(),
            ));
        }

        // Extract ZoneGroupState property from UPnP event
        if let Some(zone_group_state_xml) = self.extract_property_value("ZoneGroupState")? {
            if zone_group_state_xml.trim().is_empty() {
                return Ok(Vec::new());
            }

            // Decode XML entities and CDATA
            let decoded_xml = Self::decode_entities_with_cdata(&zone_group_state_xml);
            
            // Parse the zone group structure
            let mut zone_parser = XmlParser::new(&decoded_xml);
            zone_parser.parse_zone_group_state()
        } else {
            // No ZoneGroupState property found
            Ok(Vec::new())
        }
    }

    /// Validate that the XML has a basic UPnP event structure
    fn validate_upnp_event_structure(xml_content: &str) -> XmlParseResult<bool> {
        let mut temp_parser = XmlParser::new(xml_content);
        let mut buffer = Vec::new();
        let mut found_property = false;

        loop {
            buffer.clear();
            match temp_parser.reader.read_event_into(&mut buffer)? {
                Event::Start(ref e) => {
                    // Check if this is a property element (with or without namespace)
                    if e.name().as_ref() == b"property" || e.name().as_ref().ends_with(b":property") {
                        found_property = true;
                        break;
                    }
                }
                Event::Eof => break,
                _ => {}
            }
        }

        Ok(found_property)
    }

    /// Decode XML entities and handle CDATA sections
    pub fn decode_entities_with_cdata(text: &str) -> String {
        let mut result = text.to_string();
        
        // Handle CDATA sections first
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
        
        // Use standard entity decoding
        Self::decode_entities(&result)
    }
    /// Parse zone group state from ZoneGroupTopology event XML
    ///
    /// This method parses the ZoneGroupState property which contains XML like:
    /// ```xml
    /// <ZoneGroups>
    ///   <ZoneGroup Coordinator="RINCON_123456789" ID="RINCON_123456789:1">
    ///     <ZoneGroupMember UUID="RINCON_123456789" Location="http://192.168.1.100:1400/xml/device_description.xml" ZoneName="Living Room" Icon="x-rincon-roomicon:living" Configuration="1" SoftwareVersion="56.0-76060" MinCompatibleVersion="49.2-64250" LegacyCompatibleVersion="36.0-00000" BootSeq="33" TVConfigurationError="0" HdmiCecAvailable="0" WirelessMode="0" WirelessLeafOnly="0" HasConfiguredSSID="1" ChannelFreq="2437" BehindWifiExtender="0" WifiEnabled="1" Orientation="0" RoomCalibrationState="4" SecureRegState="3" VoiceConfigState="0" MicEnabled="1" AirPlayEnabled="1" IdleState="1" MoreInfo="" />
    ///   </ZoneGroup>
    /// </ZoneGroups>
    /// ```
    pub fn parse_zone_group_state(&mut self) -> XmlParseResult<Vec<XmlZoneGroupData>> {
        let mut zone_groups = Vec::new();
        let mut buffer = Vec::new();
        let mut current_zone_group: Option<XmlZoneGroupData> = None;
        let mut current_member: Option<XmlZoneGroupMember> = None;
        let mut _depth = 0;

        loop {
            buffer.clear();
            match self.reader.read_event_into(&mut buffer)? {
                Event::Start(ref e) => {
                    match e.name().as_ref() {
                        b"ZoneGroup" => {
                            _depth += 1;
                            // Extract coordinator from attributes
                            let mut coordinator = String::new();
                            for attr in e.attributes() {
                                let attr = attr?;
                                if attr.key.as_ref() == b"Coordinator" {
                                    coordinator = attr.unescape_value()?.into_owned();
                                    break;
                                }
                            }
                            current_zone_group = Some(XmlZoneGroupData {
                                coordinator,
                                members: Vec::new(),
                            });
                        }
                        b"ZoneGroupMember" => {
                            // Extract UUID from attributes
                            let mut uuid = String::new();
                            for attr in e.attributes() {
                                let attr = attr?;
                                if attr.key.as_ref() == b"UUID" {
                                    uuid = attr.unescape_value()?.into_owned();
                                    break;
                                }
                            }
                            current_member = Some(XmlZoneGroupMember {
                                uuid,
                                satellites: Vec::new(),
                            });
                        }
                        b"Satellite" => {
                            if let Some(ref mut member) = current_member {
                                // Extract UUID from satellite
                                for attr in e.attributes() {
                                    let attr = attr?;
                                    if attr.key.as_ref() == b"UUID" {
                                        let satellite_uuid = attr.unescape_value()?.into_owned();
                                        member.satellites.push(satellite_uuid);
                                        break;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Event::Empty(ref e) => {
                    match e.name().as_ref() {
                        b"ZoneGroup" => {
                            // Handle empty zone groups
                            let mut coordinator = String::new();
                            for attr in e.attributes() {
                                let attr = attr?;
                                if attr.key.as_ref() == b"Coordinator" {
                                    coordinator = attr.unescape_value()?.into_owned();
                                    break;
                                }
                            }
                            if !coordinator.is_empty() {
                                zone_groups.push(XmlZoneGroupData {
                                    coordinator,
                                    members: Vec::new(),
                                });
                            }
                        }
                        b"ZoneGroupMember" => {
                            // Handle self-closing member
                            let mut uuid = String::new();
                            let mut satellites = Vec::new();
                            
                            for attr in e.attributes() {
                                let attr = attr?;
                                match attr.key.as_ref() {
                                    b"UUID" => {
                                        uuid = attr.unescape_value()?.into_owned();
                                    }
                                    b"Satellites" => {
                                        let satellites_attr = attr.unescape_value()?.into_owned();
                                        if !satellites_attr.trim().is_empty() {
                                            for satellite_uuid in satellites_attr.split(',') {
                                                let satellite_uuid = satellite_uuid.trim();
                                                if !satellite_uuid.is_empty() {
                                                    satellites.push(satellite_uuid.to_string());
                                                }
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            
                            let member = XmlZoneGroupMember { uuid, satellites };
                            if let Some(ref mut zone_group) = current_zone_group {
                                zone_group.members.push(member);
                            }
                        }
                        b"Satellite" => {
                            if let Some(ref mut member) = current_member {
                                // Extract UUID from satellite
                                for attr in e.attributes() {
                                    let attr = attr?;
                                    if attr.key.as_ref() == b"UUID" {
                                        let satellite_uuid = attr.unescape_value()?.into_owned();
                                        member.satellites.push(satellite_uuid);
                                        break;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Event::End(ref e) => {
                    match e.name().as_ref() {
                        b"ZoneGroup" => {
                            _depth -= 1;
                            if let Some(zone_group) = current_zone_group.take() {
                                zone_groups.push(zone_group);
                            }
                        }
                        b"ZoneGroupMember" => {
                            if let Some(member) = current_member.take() {
                                if let Some(ref mut zone_group) = current_zone_group {
                                    zone_group.members.push(member);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                Event::Eof => break,
                _ => {}
            }
        }

        Ok(zone_groups)
    }

    /// Parse a single zone group member from XML element
    ///
    /// This method parses ZoneGroupMember elements like:
    /// ```xml
    /// <ZoneGroupMember UUID="RINCON_123456789" Location="http://192.168.1.100:1400/xml/device_description.xml" ZoneName="Living Room" ... />
    /// ```
    /// or with satellites:
    /// ```xml
    /// <ZoneGroupMember UUID="RINCON_123456789" Satellites="RINCON_SAT001,RINCON_SAT002" ... />
    /// ```
    pub fn parse_zone_group_member(
        &mut self,
        member_xml: &str,
    ) -> XmlParseResult<XmlZoneGroupMember> {
        let mut parser = XmlParser::new(member_xml);
        let mut buffer = Vec::new();

        buffer.clear();
        match parser.reader.read_event_into(&mut buffer)? {
            Event::Start(ref e) | Event::Empty(ref e) => {
                if e.name().as_ref() == b"ZoneGroupMember" {
                    // Extract UUID attribute
                    let uuid = parser.extract_attribute(member_xml, "UUID")?;

                    let mut satellites = Vec::new();

                    // Format 1: Check for Satellites attribute (comma-separated UUIDs)
                    if let Ok(satellites_attr) = parser.extract_attribute(member_xml, "Satellites") {
                        if !satellites_attr.trim().is_empty() {
                            for satellite_uuid in satellites_attr.split(',') {
                                let uuid = satellite_uuid.trim();
                                if !uuid.is_empty() {
                                    satellites.push(uuid.to_string());
                                }
                            }
                        }
                    }

                    // Format 2: Parse nested satellite elements (if any)
                    let nested_satellites = parser.parse_satellites()?;
                    satellites.extend(nested_satellites);

                    return Ok(XmlZoneGroupMember { uuid, satellites });
                }
            }
            _ => {}
        }

        Err(super::error::XmlParseError::InvalidStructure(
            "Expected ZoneGroupMember element".to_string(),
        ))
    }



    /// Parse satellite speakers for a zone group member
    fn parse_satellites(&mut self) -> XmlParseResult<Vec<String>> {
        let mut satellites = Vec::new();
        let mut buffer = Vec::new();
        let mut member_depth = 1; // We're already inside a ZoneGroupMember

        loop {
            buffer.clear();
            match self.reader.read_event_into(&mut buffer)? {
                Event::Start(ref e) => {
                    if e.name().as_ref() == b"Satellite" {
                        // Extract UUID from satellite
                        for attr in e.attributes() {
                            let attr = attr?;
                            if attr.key.as_ref() == b"UUID" {
                                let satellite_uuid = attr.unescape_value()?.into_owned();
                                satellites.push(satellite_uuid);
                                break;
                            }
                        }
                    } else if e.name().as_ref() == b"ZoneGroupMember" {
                        member_depth += 1;
                    }
                }
                Event::Empty(ref e) => {
                    if e.name().as_ref() == b"Satellite" {
                        // Extract UUID from satellite
                        for attr in e.attributes() {
                            let attr = attr?;
                            if attr.key.as_ref() == b"UUID" {
                                let satellite_uuid = attr.unescape_value()?.into_owned();
                                satellites.push(satellite_uuid);
                                break;
                            }
                        }
                    }
                }
                Event::End(ref e) => {
                    if e.name().as_ref() == b"ZoneGroupMember" {
                        member_depth -= 1;
                        if member_depth == 0 {
                            break;
                        }
                    } else if e.name().as_ref() == b"ZoneGroup" {
                        // Also break if we reach the end of the zone group
                        break;
                    }
                }
                Event::Eof => break,
                _ => {}
            }
        }

        Ok(satellites)
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
        let mut parser = XmlParser::new(xml);
        let result = parser.parse_zone_group_state().unwrap();
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
        let mut parser = XmlParser::new(xml);
        let result = parser.parse_zone_group_state().unwrap();
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
        let mut parser = XmlParser::new(xml);
        let result = parser.parse_zone_group_state().unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].coordinator, "RINCON_123456789");
        assert_eq!(result[0].members.len(), 1);
        assert_eq!(result[0].members[0].uuid, "RINCON_123456789");
        assert_eq!(result[0].members[0].satellites.len(), 2);
        assert_eq!(result[0].members[0].satellites[0], "RINCON_111111111");
        assert_eq!(result[0].members[0].satellites[1], "RINCON_222222222");
    }

    #[test]
    fn test_parse_zone_group_member() {
        let member_xml = r#"<ZoneGroupMember UUID="RINCON_123456789" Location="http://192.168.1.100:1400/xml/device_description.xml" ZoneName="Living Room" />"#;
        let mut parser = XmlParser::new("");
        let result = parser.parse_zone_group_member(member_xml).unwrap();
        assert_eq!(result.uuid, "RINCON_123456789");
        assert_eq!(result.satellites.len(), 0);
    }

    #[test]
    fn test_parse_zone_group_state_empty() {
        let xml = r#"<ZoneGroups></ZoneGroups>"#;
        let mut parser = XmlParser::new(xml);
        let result = parser.parse_zone_group_state().unwrap();
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
        let mut parser = XmlParser::new(xml);
        let result = parser.parse_zone_group_state().unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].coordinator, "RINCON_123456789");
        assert_eq!(result[0].members.len(), 2);
        assert_eq!(result[0].members[0].uuid, "RINCON_123456789");
        assert_eq!(result[0].members[1].uuid, "RINCON_987654321");
    }
}
