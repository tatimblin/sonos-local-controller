use crate::error::{Result, SonosError};
use html_escape::decode_html_entities;

use serde::Deserialize;

#[derive(Debug)]
pub struct AVTransportParser {
    property_set: Option<AVTransportPropertySet>,
}

#[derive(Debug, Deserialize)]
struct AVTransportPropertySet {
    #[serde(rename = "property")]
    property: AVTransportProperty,
}

#[derive(Debug, Deserialize)]
struct AVTransportProperty {
    #[serde(rename = "LastChange", default)]
    last_change: Option<String>,
    #[serde(rename = "TransportState", default)]
    transport_state: Option<String>,
    #[serde(rename = "CurrentTrackURI", default)]
    current_track_uri: Option<String>,
    #[serde(rename = "CurrentTrackDuration", default)]
    current_track_duration: Option<String>,
    #[serde(rename = "CurrentTrackMetaData", default)]
    current_track_metadata: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LastChangeEvent {
    #[serde(rename = "InstanceID")]
    pub instance_id: InstanceID,
}

#[derive(Debug, Deserialize)]
pub struct InstanceID {
    #[serde(rename = "TransportState", default)]
    pub transport_state: Option<ValueElement>,
    #[serde(rename = "CurrentTrackURI", default)]
    pub current_track_uri: Option<ValueElement>,
    #[serde(rename = "CurrentTrackDuration", default)]
    pub current_track_duration: Option<ValueElement>,
    #[serde(rename = "CurrentTrackMetaData", default)]
    pub current_track_metadata: Option<ValueElement>,
    #[serde(rename = "CurrentPlayMode", default)]
    pub current_play_mode: Option<ValueElement>,
    #[serde(rename = "CurrentCrossfadeMode", default)]
    pub current_crossfade_mode: Option<ValueElement>,
    #[serde(rename = "NumberOfTracks", default)]
    pub number_of_tracks: Option<ValueElement>,
    #[serde(rename = "CurrentTrack", default)]
    pub current_track: Option<ValueElement>,
    #[serde(rename = "CurrentSection", default)]
    pub current_section: Option<ValueElement>,
}

#[derive(Debug, Deserialize)]
pub struct ValueElement {
    #[serde(rename = "@val", default)]
    pub val: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DidlLite {
    #[serde(rename = "item")]
    pub item: DidlItem,
}

#[derive(Debug, Deserialize)]
pub struct DidlItem {
    #[serde(rename = "title", default)]
    pub title: Option<String>,
    #[serde(rename = "creator", default)]
    pub creator: Option<String>,
    #[serde(rename = "album", default)]
    pub album: Option<String>,
}

impl AVTransportParser {
    pub fn from_xml(xml: &str) -> Result<Self> {
        println!("🔍 Original XML: {}", xml);

        let cleaned_xml = xml
            .replace("e:propertyset", "propertyset")
            .replace("e:property", "property");

        println!("🔍 Cleaned XML: {}", cleaned_xml);

        let property_set: AVTransportPropertySet =
            serde_xml_rs::from_str(&cleaned_xml).map_err(|e| {
                println!("❌ PropertySet parse error: {}", e);
                SonosError::ParseError(format!("PropertySet parse error: {}", e))
            })?;

        println!("✅ Parsed property set: {:?}", property_set);

        Ok(Self {
            property_set: Some(property_set),
        })
    }

    pub fn to_event(&self) -> Result<AVTransportEvent> {
        let property_set = self
            .property_set
            .as_ref()
            .ok_or_else(|| SonosError::ParseError("No property set available".to_string()))?;

        let property = &property_set.property;

        let last_change = if let Some(ref last_change_xml) = property.last_change {
            match Self::parse_last_change(last_change_xml) {
                Ok(last_change) => {
                    // If we have metadata in the LastChange, try to parse the nested XML
                    if let Some(ref metadata_elem) = last_change.instance_id.current_track_metadata
                    {
                        if let Some(ref metadata_xml) = metadata_elem.val {
                            // Parse the nested DIDL-Lite XML from the metadata string
                            if let Ok(didl) = Self::parse_didl_metadata(metadata_xml) {
                                // You can store the parsed DIDL data or use it as needed
                                println!("✅ Parsed DIDL metadata: {:?}", didl);
                            }
                        }
                    }
                    Some(last_change)
                }
                Err(e) => {
                    println!("❌ Failed to parse LastChange: {}", e);
                    None
                }
            }
        } else {
            None
        };

        Ok(AVTransportEvent {
            transport_state: property.transport_state.clone(),
            current_track_uri: property.current_track_uri.clone(),
            current_track_duration: property.current_track_duration.clone(),
            current_track_metadata: property.current_track_metadata.clone(),
            last_change,
        })
    }

    fn parse_last_change(xml_content: &str) -> Result<LastChangeEvent> {
        let decoded = decode_html_entities(xml_content);
        let fixed_xml = Self::escape_nested_xml_in_attributes(&decoded);

        serde_xml_rs::from_str(&fixed_xml)
            .map_err(|e| SonosError::ParseError(format!("LastChange parse error: {}", e)))
    }

    fn escape_nested_xml_in_attributes(xml: &str) -> String {
        let mut result = xml.to_string();

        // First, escape unescaped ampersands in all attribute values
        result = Self::escape_ampersands_in_attributes(&result);

        // Then handle nested XML in metadata attributes
        loop {
            let mut found_match = false;

            // Look for unescaped XML content starting with val="<
            if let Some(val_pos) = result.find(r#"val="<"#) {
                let content_start = val_pos + r#"val=""#.len();

                // Find the end of the nested XML by looking for the closing tag
                if let Some(didl_start) = result[content_start..].find("<DIDL-Lite") {
                    let search_from = content_start + didl_start;

                    // Look for the closing </DIDL-Lite> tag
                    if let Some(didl_end) = result[search_from..].find("</DIDL-Lite>") {
                        let content_end = search_from + didl_end + "</DIDL-Lite>".len();

                        // Extract the nested XML content
                        let nested_content = &result[content_start..content_end];

                        // Properly escape the nested XML content (ampersands already escaped)
                        let escaped_content = nested_content
                            .replace('<', "&lt;") // Escape less-than
                            .replace('>', "&gt;") // Escape greater-than
                            .replace('"', "&quot;"); // Escape quotes

                        result.replace_range(content_start..content_end, &escaped_content);
                        found_match = true;
                    }
                }
            }

            if !found_match {
                break;
            }
        }

        result
    }

    fn escape_ampersands_in_attributes(xml: &str) -> String {
        let mut result = String::new();
        let mut chars = xml.chars().peekable();
        let mut in_attribute_value = false;
        let mut quote_char = None;

        while let Some(ch) = chars.next() {
            match ch {
                '"' | '\'' if !in_attribute_value => {
                    // Starting an attribute value
                    in_attribute_value = true;
                    quote_char = Some(ch);
                    result.push(ch);
                }
                '"' | '\'' if in_attribute_value && Some(ch) == quote_char => {
                    // Ending an attribute value
                    in_attribute_value = false;
                    quote_char = None;
                    result.push(ch);
                }
                '&' if in_attribute_value => {
                    // Check if this is already an entity
                    let remaining: String = chars.clone().collect();
                    if remaining.starts_with("amp;")
                        || remaining.starts_with("lt;")
                        || remaining.starts_with("gt;")
                        || remaining.starts_with("quot;")
                        || remaining.starts_with("apos;")
                    {
                        // Already an entity, keep as is
                        result.push(ch);
                    } else {
                        // Unescaped ampersand, escape it
                        result.push_str("&amp;");
                    }
                }
                _ => {
                    result.push(ch);
                }
            }
        }

        result
    }

    fn parse_didl_metadata(metadata_xml: &str) -> Result<DidlLite> {
        let decoded = decode_html_entities(metadata_xml);

        // Remove namespace prefixes to simplify parsing
        let cleaned = decoded
            .replace("dc:", "")
            .replace("upnp:", "")
            .replace("r:", "");

        serde_xml_rs::from_str(&cleaned)
            .map_err(|e| SonosError::ParseError(format!("DIDL parse error: {}", e)))
    }
}

#[derive(Debug)]
pub struct AVTransportEvent {
    pub transport_state: Option<String>,
    pub current_track_uri: Option<String>,
    pub current_track_duration: Option<String>,
    pub current_track_metadata: Option<String>,
    pub last_change: Option<LastChangeEvent>,
}

impl AVTransportEvent {
    /// Get transport state from either direct property or parsed LastChange
    pub fn get_transport_state(&self) -> Option<&str> {
        // Prefer LastChange data if available
        if let Some(ref last_change) = self.last_change {
            if let Some(ref state) = last_change.instance_id.transport_state {
                if let Some(ref val) = state.val {
                    return Some(val);
                }
            }
        }
        // Fall back to direct property
        self.transport_state.as_deref()
    }

    /// Get current track URI from either direct property or parsed LastChange
    pub fn get_current_track_uri(&self) -> Option<&str> {
        if let Some(ref last_change) = self.last_change {
            if let Some(ref uri) = last_change.instance_id.current_track_uri {
                if let Some(ref val) = uri.val {
                    return Some(val);
                }
            }
        }
        self.current_track_uri.as_deref()
    }

    /// Get current track duration from either direct property or parsed LastChange
    pub fn get_current_track_duration(&self) -> Option<&str> {
        if let Some(ref last_change) = self.last_change {
            if let Some(ref duration) = last_change.instance_id.current_track_duration {
                if let Some(ref val) = duration.val {
                    return Some(val);
                }
            }
        }
        self.current_track_duration.as_deref()
    }

    /// Get current track metadata from either direct property or parsed LastChange
    pub fn get_current_track_metadata(&self) -> Option<&str> {
        if let Some(ref last_change) = self.last_change {
            if let Some(ref metadata) = last_change.instance_id.current_track_metadata {
                if let Some(ref val) = metadata.val {
                    return Some(val);
                }
            }
        }
        self.current_track_metadata.as_deref()
    }
}
