use crate::error::{Result, SonosError};
use serde::Deserialize;

/// Simple interface for parsing AV Transport XML data
pub struct AVTransportParser {
    property_set: Option<AVTransportPropertySet>,
}

/// Track metadata information
#[derive(Debug, Clone)]
pub struct TrackMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
}

/// UPnP PropertySet - internal XML mapping
#[derive(Debug, Deserialize)]
struct AVTransportPropertySet {
    #[serde(rename = "property")]
    property: AVTransportProperty,
}

/// UPnP Property containing LastChange or direct elements - internal XML mapping
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

/// LastChange Event structure - internal XML mapping
#[derive(Debug, Deserialize)]
struct AVTransportEvent {
    #[serde(rename = "InstanceID")]
    instance_id: AVTransportInstanceId,
}

/// InstanceID containing transport information - internal XML mapping
#[derive(Debug, Deserialize)]
struct AVTransportInstanceId {
    #[serde(rename = "TransportState", default)]
    transport_state: Option<AVTransportValueElement>,
    #[serde(rename = "CurrentTrackURI", default)]
    current_track_uri: Option<AVTransportValueElement>,
    #[serde(rename = "CurrentTrackDuration", default)]
    current_track_duration: Option<AVTransportValueElement>,
    #[serde(rename = "CurrentTrackMetaData", default)]
    current_track_metadata: Option<AVTransportValueElement>,
}

/// Element with val attribute - internal XML mapping
#[derive(Debug, Deserialize)]
struct AVTransportValueElement {
    #[serde(rename = "@val", default)]
    val: Option<String>,
}

/// DIDL-Lite structure for metadata - public for legacy compatibility
#[derive(Debug, Deserialize)]
pub struct DidlLite {
    #[serde(rename = "item")]
    pub item: DidlItem,
}

/// DIDL Item containing metadata - public for legacy compatibility
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
    /// Create a new parser from XML string
    pub fn from_xml(xml: &str) -> Result<Self> {
        let cleaned_xml = xml
            .replace("e:propertyset", "propertyset")
            .replace("e:property", "property");

        let property_set = serde_xml_rs::from_str(&cleaned_xml)
            .map_err(|e| SonosError::ParseError(format!("PropertySet parse error: {}", e)))?;

        Ok(Self {
            property_set: Some(property_set),
        })
    }

    /// Get the current transport state (PLAYING, PAUSED_PLAYBACK, STOPPED, etc.)
    pub fn transport_state(&self) -> Option<String> {
        let property_set = self.property_set.as_ref()?;

        // Check direct property first
        if let Some(state) = &property_set.property.transport_state {
            return Some(state.clone());
        }

        // Check LastChange if present
        if let Some(last_change) = &property_set.property.last_change {
            // Try serde parsing first
            if let Ok(event) = parse_last_change_event(last_change) {
                if let Some(transport_state) = event.instance_id.transport_state {
                    return transport_state.val;
                }
            } else {
                // Fallback to manual parsing for complex XML with nested quotes
                let decoded = decode_html_entities(last_change);
                if let Some(value) = extract_attribute_value(&decoded, "TransportState") {
                    return Some(value);
                }
            }
        }

        None
    }

    /// Get the current track URI
    pub fn current_track_uri(&self) -> Option<String> {
        let property_set = self.property_set.as_ref()?;

        // Check direct property first
        if let Some(uri) = &property_set.property.current_track_uri {
            return Some(uri.clone());
        }

        // Check LastChange if present
        if let Some(last_change) = &property_set.property.last_change {
            // Try serde parsing first
            if let Ok(event) = parse_last_change_event(last_change) {
                if let Some(track_uri) = event.instance_id.current_track_uri {
                    return track_uri.val;
                }
            } else {
                // Fallback to manual parsing for complex XML with nested quotes
                let decoded = decode_html_entities(last_change);
                if let Some(value) = extract_attribute_value(&decoded, "CurrentTrackURI") {
                    return Some(value);
                }
            }
        }

        None
    }

    /// Get the current track duration as a string
    pub fn current_track_duration_string(&self) -> Option<String> {
        let property_set = self.property_set.as_ref()?;

        // Check direct property first
        if let Some(duration) = &property_set.property.current_track_duration {
            if !duration.is_empty() {
                return Some(duration.clone());
            }
        }

        // Check LastChange if present
        if let Some(last_change) = &property_set.property.last_change {
            if let Ok(event) = parse_last_change_event(last_change) {
                if let Some(track_duration) = event.instance_id.current_track_duration {
                    if let Some(val) = track_duration.val {
                        if !val.is_empty() {
                            return Some(val);
                        }
                    }
                }
            }
        }

        None
    }

    /// Get the current track duration in seconds
    pub fn current_track_duration_seconds(&self) -> Option<u64> {
        self.current_track_duration_string()
            .and_then(|duration_str| parse_duration(&duration_str))
    }

    /// Get the current track metadata
    pub fn current_track_metadata(&self) -> Option<TrackMetadata> {
        let property_set = self.property_set.as_ref()?;

        // Check direct property first
        if let Some(metadata_xml) = &property_set.property.current_track_metadata {
            let decoded = decode_html_entities(metadata_xml);
            if let Ok(didl) = parse_didl_metadata(&decoded) {
                return Some(TrackMetadata {
                    title: didl.item.title,
                    artist: didl.item.creator,
                    album: didl.item.album,
                });
            }
        }

        // Check LastChange if present
        if let Some(last_change) = &property_set.property.last_change {
            if let Ok(event) = parse_last_change_event(last_change) {
                if let Some(metadata_elem) = event.instance_id.current_track_metadata {
                    if let Some(val) = metadata_elem.val {
                        let decoded = decode_html_entities(&val);
                        if let Ok(didl) = parse_didl_metadata(&decoded) {
                            return Some(TrackMetadata {
                                title: didl.item.title,
                                artist: didl.item.creator,
                                album: didl.item.album,
                            });
                        }
                    }
                }
            }
        }

        None
    }

    /// Get the current track duration as a string (legacy method for backward compatibility)
    pub fn current_track_duration(&self) -> Option<String> {
        self.current_track_duration_string()
    }
}

// Helper functions (internal)
fn parse_last_change_event(last_change: &str) -> Result<AVTransportEvent> {
    let decoded = decode_html_entities(last_change);
    serde_xml_rs::from_str(&decoded)
        .map_err(|e| SonosError::ParseError(format!("LastChange parse error: {}", e)))
}

fn parse_didl_metadata(metadata_xml: &str) -> Result<DidlLite> {
    let cleaned = metadata_xml.replace("dc:", "").replace("upnp:", "");
    serde_xml_rs::from_str(&cleaned)
        .map_err(|e| SonosError::ParseError(format!("DIDL parse error: {}", e)))
}

// Public helper functions

/// Parse a duration string into seconds
pub fn parse_duration(duration_str: &str) -> Option<u64> {
    if duration_str.is_empty() {
        return None;
    }

    // Try parsing as seconds first
    if let Ok(seconds) = duration_str.parse::<u64>() {
        return Some(seconds);
    }

    // Try parsing as time format (H:MM:SS or MM:SS)
    let parts: Vec<&str> = duration_str.split(':').collect();
    match parts.len() {
        2 => {
            // MM:SS format
            if let (Ok(minutes), Ok(seconds)) = (parts[0].parse::<u64>(), parts[1].parse::<u64>()) {
                Some(minutes * 60 + seconds)
            } else {
                None
            }
        }
        3 => {
            // H:MM:SS format
            if let (Ok(hours), Ok(minutes), Ok(seconds)) = (
                parts[0].parse::<u64>(),
                parts[1].parse::<u64>(),
                parts[2].parse::<u64>(),
            ) {
                Some(hours * 3600 + minutes * 60 + seconds)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Parse track metadata from DIDL-Lite XML
pub fn parse_track_metadata(metadata_xml: &str) -> Result<TrackMetadata> {
    let decoded = decode_html_entities(metadata_xml);
    let didl = parse_didl_metadata(&decoded)?;

    Ok(TrackMetadata {
        title: didl.item.title,
        artist: didl.item.creator,
        album: didl.item.album,
    })
}

/// Parse transport state from XML (legacy function for backward compatibility)
pub fn parse_transport_state(xml: &str) -> Result<Option<String>> {
    let parser = AVTransportParser::from_xml(xml)?;
    Ok(parser.transport_state())
}

/// Extract DIDL metadata (legacy function for backward compatibility)
pub fn extract_didl_metadata(metadata_xml: &str) -> Result<DidlLite> {
    parse_didl_metadata(metadata_xml)
}

// Internal helper functions
fn decode_html_entities(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

fn extract_attribute_value(xml: &str, element_name: &str) -> Option<String> {
    let pattern = format!(r#"<{} val=""#, element_name);

    if let Some(start_pos) = xml.find(&pattern) {
        let value_start = start_pos + pattern.len();
        let mut in_entity = false;
        let mut i = value_start;

        while i < xml.len() {
            let c = xml.chars().nth(i).unwrap();

            if c == '&' {
                in_entity = true;
            } else if c == ';' && in_entity {
                in_entity = false;
            } else if c == '"' && !in_entity {
                // This is the closing quote for the attribute
                let value = &xml[value_start..i];
                return Some(value.to_string());
            }

            i += 1;
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_from_xml_with_direct_properties() {
        let xml = r#"
            <propertyset>
                <property>
                    <TransportState>PLAYING</TransportState>
                    <CurrentTrackURI>x-sonos-spotify:spotify%3atrack%3a123</CurrentTrackURI>
                    <CurrentTrackDuration>0:03:45</CurrentTrackDuration>
                </property>
            </propertyset>
        "#;

        let parser = AVTransportParser::from_xml(xml).unwrap();

        assert_eq!(parser.transport_state(), Some("PLAYING".to_string()));
        assert_eq!(
            parser.current_track_uri(),
            Some("x-sonos-spotify:spotify%3atrack%3a123".to_string())
        );
        assert_eq!(
            parser.current_track_duration_string(),
            Some("0:03:45".to_string())
        );
        assert_eq!(parser.current_track_duration_seconds(), Some(225));
    }

    #[test]
    fn test_parser_from_xml_with_lastchange() {
        let xml = r#"
            <propertyset>
                <property>
                    <LastChange>&lt;Event xmlns="urn:schemas-upnp-org:metadata-1-0/AVT/"&gt;&lt;InstanceID val="0"&gt;&lt;TransportState val="PAUSED_PLAYBACK"/&gt;&lt;CurrentTrackURI val="x-sonos-http:_uuid_rincon-playlist"/&gt;&lt;/InstanceID&gt;&lt;/Event&gt;</LastChange>
                </property>
            </propertyset>
        "#;

        let parser = AVTransportParser::from_xml(xml).unwrap();

        assert_eq!(
            parser.transport_state(),
            Some("PAUSED_PLAYBACK".to_string())
        );
        assert_eq!(
            parser.current_track_uri(),
            Some("x-sonos-http:_uuid_rincon-playlist".to_string())
        );
    }

    #[test]
    fn test_parser_with_metadata() {
        let xml = r#"
            <propertyset>
                <property>
                    <TransportState>PLAYING</TransportState>
                    <CurrentTrackMetaData>&lt;DIDL-Lite xmlns="urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:upnp="urn:schemas-upnp-org:metadata-1-0/upnp/"&gt;&lt;item&gt;&lt;dc:title&gt;Test Song&lt;/dc:title&gt;&lt;dc:creator&gt;Test Artist&lt;/dc:creator&gt;&lt;upnp:album&gt;Test Album&lt;/upnp:album&gt;&lt;/item&gt;&lt;/DIDL-Lite&gt;</CurrentTrackMetaData>
                </property>
            </propertyset>
        "#;

        let parser = AVTransportParser::from_xml(xml).unwrap();
        let metadata = parser.current_track_metadata().unwrap();

        assert_eq!(metadata.title, Some("Test Song".to_string()));
        assert_eq!(metadata.artist, Some("Test Artist".to_string()));
        assert_eq!(metadata.album, Some("Test Album".to_string()));
    }

    #[test]
    fn test_parser_with_namespace_prefixes() {
        let xml = r#"<e:propertyset xmlns:e="urn:schemas-upnp-org:event-1-0">
            <e:property>
                <TransportState>STOPPED</TransportState>
            </e:property>
        </e:propertyset>"#;

        let parser = AVTransportParser::from_xml(xml).unwrap();
        assert_eq!(parser.transport_state(), Some("STOPPED".to_string()));
    }

    #[test]
    fn test_parser_empty_values() {
        let xml = r#"
            <propertyset>
                <property>
                    <TransportState></TransportState>
                    <CurrentTrackDuration></CurrentTrackDuration>
                </property>
            </propertyset>
        "#;

        let parser = AVTransportParser::from_xml(xml).unwrap();

        assert_eq!(parser.transport_state(), Some("".to_string()));
        assert_eq!(parser.current_track_duration_string(), None); // Empty duration should return None
        assert_eq!(parser.current_track_duration_seconds(), None);
    }

    #[test]
    fn test_parser_real_sonos_xml() {
        // Real XML from Sonos device with complex nested quotes
        let xml = r#"<e:propertyset xmlns:e="urn:schemas-upnp-org:event-1-0"><e:property><LastChange>&lt;Event xmlns=&quot;urn:schemas-upnp-org:metadata-1-0/AVT/&quot;&gt;&lt;InstanceID val=&quot;0&quot;&gt;&lt;TransportState val=&quot;PAUSED_PLAYBACK&quot;/&gt;&lt;CurrentTrackURI val=&quot;x-sonos-vli:RINCON_804AF2AA2FA201400:2,spotify:246cdc0654993a86eef68e9531af6087&quot;/&gt;&lt;/InstanceID&gt;&lt;/Event&gt;</LastChange></e:property></e:propertyset>"#;

        let parser = AVTransportParser::from_xml(xml).unwrap();

        assert_eq!(
            parser.transport_state(),
            Some("PAUSED_PLAYBACK".to_string())
        );
        assert_eq!(
            parser.current_track_uri(),
            Some(
                "x-sonos-vli:RINCON_804AF2AA2FA201400:2,spotify:246cdc0654993a86eef68e9531af6087"
                    .to_string()
            )
        );
    }

    #[test]
    fn test_parse_duration_helper() {
        // Test the public helper function
        assert_eq!(parse_duration("225"), Some(225));
        assert_eq!(parse_duration("3:45"), Some(225));
        assert_eq!(parse_duration("1:03:45"), Some(3825));
        assert_eq!(parse_duration("00:03:45"), Some(225));
        assert_eq!(parse_duration(""), None);
        assert_eq!(parse_duration("invalid"), None);
        assert_eq!(parse_duration("1:2:3:4"), None);
    }

    #[test]
    fn test_parse_track_metadata_helper() {
        let didl_xml = r#"
            <DIDL-Lite xmlns="urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:upnp="urn:schemas-upnp-org:metadata-1-0/upnp/">
                <item>
                    <dc:title>Test Song</dc:title>
                    <dc:creator>Test Artist</dc:creator>
                    <upnp:album>Test Album</upnp:album>
                </item>
            </DIDL-Lite>
        "#;

        let metadata = parse_track_metadata(didl_xml).unwrap();
        assert_eq!(metadata.title, Some("Test Song".to_string()));
        assert_eq!(metadata.artist, Some("Test Artist".to_string()));
        assert_eq!(metadata.album, Some("Test Album".to_string()));
    }

    #[test]
    fn test_parse_track_metadata_partial() {
        let didl_xml = r#"
            <DIDL-Lite xmlns="urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/" xmlns:dc="http://purl.org/dc/elements/1.1/">
                <item>
                    <dc:title>Only Title</dc:title>
                </item>
            </DIDL-Lite>
        "#;

        let metadata = parse_track_metadata(didl_xml).unwrap();
        assert_eq!(metadata.title, Some("Only Title".to_string()));
        assert_eq!(metadata.artist, None);
        assert_eq!(metadata.album, None);
    }

    #[test]
    fn test_parser_invalid_xml() {
        let invalid_xml = r#"<invalid>xml</invalid>"#;
        let result = AVTransportParser::from_xml(invalid_xml);
        assert!(result.is_err());
    }
}
