use super::error::{XmlParseError, XmlParseResult};
use super::types::*;
use crate::services::av_transport::types::{XmlAVTransportData, XmlDidlMetadata, XmlDidlLite};
use crate::services::rendering_control::types::XmlRenderingControlData;
use crate::services::zone_group_topology::types::{XmlZoneGroupData, XmlZoneGroups};
use quick_xml::Reader;
use regex;
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
        // First try parsing as regular propertyset
        if let Ok(propertyset) = serde_xml_rs::from_str::<XmlPropertySet>(xml) {
            return Ok(propertyset.property);
        }

        // If that fails, try parsing with namespace support by removing namespace prefixes
        let cleaned_xml = xml
            .replace("e:propertyset", "propertyset")
            .replace("e:property", "property");

        match serde_xml_rs::from_str::<XmlPropertySet>(&cleaned_xml) {
            Ok(propertyset) => Ok(propertyset.property),
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

            // Use manual parsing for RenderingControl LastChange events due to complex structure
            if volume.is_none() {
                // Look for Master channel volume using regex
                if let Some(captures) = regex::Regex::new(r#"<Volume\s+channel="Master"\s+val="([^"]+)""#)
                    .unwrap()
                    .captures(&decoded)
                {
                    volume = captures[1].parse::<u8>().ok();
                }
            }

            if muted.is_none() {
                // Look for Master channel mute using regex
                if let Some(captures) = regex::Regex::new(r#"<Mute\s+channel="Master"\s+val="([^"]+)""#)
                    .unwrap()
                    .captures(&decoded)
                {
                    muted = Some(Self::parse_boolean_value(&captures[1]));
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

            // Use manual parsing for LastChange event due to serde-xml-rs limitations
            match Self::parse_last_change_manually(&decoded) {
                Ok(parsed_data) => {
                    if transport_state.is_none() {
                        transport_state = parsed_data.transport_state;
                    }

                    if current_track_metadata.is_none() {
                        if let Some(metadata_xml) = parsed_data.current_track_metadata {
                            current_track_metadata = Some(Self::parse_didl_serde(&metadata_xml)?);
                        }
                    }

                    if current_track_duration.is_none() {
                        current_track_duration = parsed_data.current_track_duration;
                    }

                    if current_track_uri.is_none() {
                        current_track_uri = parsed_data.current_track_uri;
                    }
                }
                Err(_) => {
                    // Ignore parsing errors for LastChange - not critical
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

    /// Manual parser for LastChange events to work around serde-xml-rs limitations
    fn parse_last_change_manually(xml: &str) -> XmlParseResult<ManualLastChangeData> {
        let mut transport_state = None;
        let mut current_track_metadata = None;
        let mut current_track_duration = None;
        let mut current_track_uri = None;

        // Simple regex-based parsing for the specific elements we need
        // Look for TransportState val attribute
        let transport_regex = regex::Regex::new(r#"<TransportState\s+val="([^"]+)""#).unwrap();
        if let Some(captures) = transport_regex.captures(xml) {
            transport_state = Some(captures[1].to_string());
        }

        // Look for CurrentTrackDuration val attribute
        if let Some(captures) = regex::Regex::new(r#"<CurrentTrackDuration\s+val="([^"]+)""#)
            .unwrap()
            .captures(xml)
        {
            current_track_duration = Some(captures[1].to_string());
        }

        // Look for CurrentTrackURI val attribute
        if let Some(captures) = regex::Regex::new(r#"<CurrentTrackURI\s+val="([^"]+)""#)
            .unwrap()
            .captures(xml)
        {
            current_track_uri = Some(captures[1].to_string());
        }

        // Look for CurrentTrackMetaData val attribute
        if let Some(captures) = regex::Regex::new(r#"<CurrentTrackMetaData\s+val="([^"]+)""#)
            .unwrap()
            .captures(xml)
        {
            let metadata_xml = Self::decode_entities(&captures[1]);
            current_track_metadata = Some(metadata_xml);
        }

        Ok(ManualLastChangeData {
            transport_state,
            current_track_metadata,
            current_track_duration,
            current_track_uri,
        })
    }
}

/// Helper struct for manual LastChange parsing
struct ManualLastChangeData {
    pub transport_state: Option<String>,
    pub current_track_metadata: Option<String>,
    pub current_track_duration: Option<String>,
    pub current_track_uri: Option<String>,
}