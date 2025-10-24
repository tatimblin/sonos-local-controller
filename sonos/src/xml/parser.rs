use super::error::{XmlParseError, XmlParseResult};
use super::types::*;
use quick_xml::{events::Event, Reader};
use serde_xml_rs;

/// Core XML parser that wraps quick-xml::Reader and provides high-level parsing methods
pub struct XmlParser<'a> {
    pub(crate) reader: Reader<&'a [u8]>,
    pub(crate) buffer: Vec<u8>,
}

impl<'a> XmlParser<'a> {
    /// Create a new XML parser from a string
    pub fn new(xml: &'a str) -> Self {
        let reader = Reader::from_str(xml);

        Self {
            reader,
            buffer: Vec::new(),
        }
    }

    /// Parse zone groups using serde
    pub fn parse_zone_groups_serde(xml: &str) -> XmlParseResult<Vec<XmlZoneGroupData>> {
        let decoded_xml = Self::decode_entities(xml);

        match serde_xml_rs::from_str::<XmlZoneGroups>(&decoded_xml) {
            Ok(zone_groups) => Ok(zone_groups.zone_groups),
            Err(e) => Err(XmlParseError::SyntaxError(format!(
                "Serde XML error: {}",
                e
            ))),
        }
    }

    /// Parse UPnP property using serde-xml-rs
    pub fn parse_property_serde(xml: &str) -> XmlParseResult<XmlProperty> {
        match serde_xml_rs::from_str::<XmlProperty>(xml) {
            Ok(property) => Ok(property),
            Err(e) => Err(XmlParseError::SyntaxError(format!(
                "Serde XML error: {}",
                e
            ))),
        }
    }

    /// Parse DIDL metadata using serde-xml-rs
    pub fn parse_didl_serde(xml: &str) -> XmlParseResult<XmlDidlMetadata> {
        let decoded_xml = Self::decode_entities(xml);

        match serde_xml_rs::from_str::<XmlDidlLite>(&decoded_xml) {
            Ok(didl) => Ok(didl.into()),
            Err(e) => Err(XmlParseError::SyntaxError(format!(
                "Serde XML error: {}",
                e
            ))),
        }
    }

    /// Parse rendering control data from property using serde
    pub fn parse_rendering_control_serde(xml: &str) -> XmlParseResult<XmlRenderingControlData> {
        let property = Self::parse_property_serde(xml)?;

        let mut volume = None;
        let mut muted = None;

        // Parse direct properties
        if let Some(vol_str) = property.volume {
            volume = vol_str.parse::<u8>().ok();
        }

        if let Some(mute_str) = property.mute {
            muted = Some(Self::parse_boolean_value(&mute_str));
        }

        // Parse LastChange if direct properties not found and LastChange exists
        if let Some(last_change) = property.last_change {
            let decoded = Self::decode_entities(&last_change);

            // Try to parse the nested XML as a LastChange event
            if let Ok(event) = serde_xml_rs::from_str::<XmlLastChangeEvent>(&decoded) {
                if let Some(instance_id) = event.instance_id {
                    if volume.is_none() {
                        if let Some(vol_elem) = instance_id.volume {
                            volume = vol_elem.val.parse::<u8>().ok();
                        }
                    }

                    if muted.is_none() {
                        if let Some(mute_elem) = instance_id.mute {
                            muted = Some(Self::parse_boolean_value(&mute_elem.val));
                        }
                    }
                }
            }
        }

        Ok(XmlRenderingControlData { volume, muted })
    }

    /// Parse AVTransport data from property using serde
    pub fn parse_av_transport_serde(xml: &str) -> XmlParseResult<XmlAVTransportData> {
        let property = Self::parse_property_serde(xml)?;

        let mut transport_state = property.transport_state;
        let mut current_track_metadata = None;
        let mut current_track_duration = property.current_track_duration;
        let mut current_track_uri = property.current_track_uri;

        // Parse metadata if present
        if let Some(metadata_xml) = property.current_track_metadata {
            current_track_metadata = Some(Self::parse_didl_serde(&metadata_xml)?);
        }

        // Parse LastChange if direct properties not found
        if let Some(last_change) = property.last_change {
            let decoded = Self::decode_entities(&last_change);

            // Try to parse the nested XML as a LastChange event
            if let Ok(event) = serde_xml_rs::from_str::<XmlLastChangeEvent>(&decoded) {
                if let Some(instance_id) = event.instance_id {
                    if transport_state.is_none() {
                        if let Some(state_elem) = instance_id.transport_state {
                            transport_state = Some(state_elem.val);
                        }
                    }

                    if current_track_metadata.is_none() {
                        if let Some(metadata_elem) = instance_id.current_track_metadata {
                            current_track_metadata =
                                Some(Self::parse_didl_serde(&metadata_elem.val)?);
                        }
                    }

                    if current_track_duration.is_none() {
                        if let Some(duration_elem) = instance_id.current_track_duration {
                            current_track_duration = Some(duration_elem.val);
                        }
                    }

                    if current_track_uri.is_none() {
                        if let Some(uri_elem) = instance_id.current_track_uri {
                            current_track_uri = Some(uri_elem.val);
                        }
                    }
                }
            }
        }

        Ok(XmlAVTransportData {
            transport_state,
            current_track_metadata,
            current_track_duration,
            current_track_uri,
        })
    }

    /// Helper method to parse boolean values from UPnP XML
    pub fn parse_boolean_value(value: &str) -> bool {
        match value.to_lowercase().as_str() {
            "1" | "true" => true,
            "0" | "false" => false,
            _ => false,
        }
    }

    /// Decode XML entities in text content
    pub fn decode_entities(text: &str) -> String {
        text.replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&amp;", "&")
            .replace("&quot;", "\"")
            .replace("&apos;", "'")
            .replace("&#39;", "'")
            .replace("&#34;", "\"")
            .replace("&#60;", "<")
            .replace("&#62;", ">")
            .replace("&#38;", "&")
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xml_parser_creation() {
        let xml = "<root><element>value</element></root>";
        let _parser = XmlParser::new(xml);
        // Just verify it doesn't panic
        assert!(true);
    }

    #[test]
    fn test_decode_entities() {
        let text = "&lt;test&gt; &amp; &quot;quoted&quot; &apos;single&apos;";
        let decoded = XmlParser::decode_entities(text);
        assert_eq!(decoded, "<test> & \"quoted\" 'single'");
    }

    #[test]
    fn test_serde_rendering_control() {
        let xml = r#"
            <property>
                <Volume>50</Volume>
                <Mute>1</Mute>
            </property>
        "#;

        let result = XmlParser::parse_rendering_control_serde(xml).unwrap();
        assert_eq!(result.volume, Some(50));
        assert_eq!(result.muted, Some(true));
    }

    #[test]
    fn test_serde_zone_groups() {
        let xml = r#"
            <ZoneGroups>
                <ZoneGroup Coordinator="RINCON_123456789" ID="RINCON_123456789:1">
                    <ZoneGroupMember UUID="RINCON_123456789" Location="http://192.168.1.100:1400/xml/device_description.xml" ZoneName="Living Room" />
                </ZoneGroup>
            </ZoneGroups>
        "#;

        let result = XmlParser::parse_zone_groups_serde(xml).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].coordinator, "RINCON_123456789");
        assert_eq!(result[0].members.len(), 1);
    }
}
