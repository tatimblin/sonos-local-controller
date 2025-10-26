use crate::error::{Result, SonosError};
use serde::Deserialize;

/// Simple interface for parsing Rendering Control XML data
pub struct RenderingControlParser {
    property_set: Option<RenderingControlPropertySet>,
}

/// UPnP PropertySet - internal XML mapping
#[derive(Debug, Deserialize)]
struct RenderingControlPropertySet {
    #[serde(rename = "property")]
    property: RenderingControlProperty,
}

/// UPnP Property containing LastChange or direct elements - internal XML mapping
#[derive(Debug, Deserialize)]
struct RenderingControlProperty {
    #[serde(rename = "LastChange", default)]
    last_change: Option<String>,
    #[serde(rename = "Volume", default)]
    volume: Option<String>,
    #[serde(rename = "Mute", default)]
    mute: Option<String>,
}

/// LastChange Event structure - internal XML mapping
#[derive(Debug, Deserialize)]
struct RenderingControlEvent {
    #[serde(rename = "InstanceID")]
    instance_id: RenderingControlInstanceId,
}

/// InstanceID containing rendering control information - internal XML mapping
#[derive(Debug, Deserialize)]
struct RenderingControlInstanceId {
    #[serde(rename = "Volume", default)]
    volume: Vec<RenderingControlVolumeElement>,
    #[serde(rename = "Mute", default)]
    mute: Vec<RenderingControlMuteElement>,
}

/// Volume element with channel and val attributes - internal XML mapping
#[derive(Debug, Deserialize)]
struct RenderingControlVolumeElement {
    #[serde(rename = "@channel")]
    channel: String,
    #[serde(rename = "@val")]
    val: String,
}

/// Mute element with channel and val attributes - internal XML mapping
#[derive(Debug, Deserialize)]
struct RenderingControlMuteElement {
    #[serde(rename = "@channel")]
    channel: String,
    #[serde(rename = "@val")]
    val: String,
}

impl RenderingControlParser {
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

    /// Get the current volume level (0-100)
    pub fn volume(&self) -> Option<u8> {
        let property_set = self.property_set.as_ref()?;

        // Check direct property first
        if let Some(vol_str) = &property_set.property.volume {
            if let Ok(volume) = vol_str.parse::<u8>() {
                return Some(volume);
            }
        }

        // Check LastChange if present
        if let Some(last_change) = &property_set.property.last_change {
            // Try serde parsing first
            if let Ok(event) = parse_last_change_event(last_change) {
                // Look for Master channel volume
                for volume_elem in &event.instance_id.volume {
                    if volume_elem.channel == "Master" {
                        if let Ok(volume) = volume_elem.val.parse::<u8>() {
                            return Some(volume);
                        }
                    }
                }
            } else {
                // Fallback to manual parsing for complex XML with nested quotes
                let decoded = decode_html_entities(last_change);
                if let Some(value) = extract_master_channel_value(&decoded, "Volume") {
                    if let Ok(volume) = value.parse::<u8>() {
                        return Some(volume);
                    }
                }
            }
        }

        None
    }

    /// Get the current mute state
    pub fn muted(&self) -> Option<bool> {
        let property_set = self.property_set.as_ref()?;

        // Check direct property first
        if let Some(mute_str) = &property_set.property.mute {
            return Some(parse_boolean_value(mute_str));
        }

        // Check LastChange if present
        if let Some(last_change) = &property_set.property.last_change {
            // Try serde parsing first
            if let Ok(event) = parse_last_change_event(last_change) {
                // Look for Master channel mute
                for mute_elem in &event.instance_id.mute {
                    if mute_elem.channel == "Master" {
                        return Some(parse_boolean_value(&mute_elem.val));
                    }
                }
            } else {
                // Fallback to manual parsing for complex XML with nested quotes
                let decoded = decode_html_entities(last_change);
                if let Some(value) = extract_master_channel_value(&decoded, "Mute") {
                    return Some(parse_boolean_value(&value));
                }
            }
        }

        None
    }
}

// Helper functions (internal)
fn parse_last_change_event(last_change: &str) -> Result<RenderingControlEvent> {
    let decoded = decode_html_entities(last_change);
    serde_xml_rs::from_str(&decoded)
        .map_err(|e| SonosError::ParseError(format!("LastChange parse error: {}", e)))
}

// Public helper functions

/// Parse volume information from XML (legacy function for backward compatibility)
pub fn parse_volume(xml: &str) -> Result<Option<u8>> {
    let parser = RenderingControlParser::from_xml(xml)?;
    Ok(parser.volume())
}

/// Parse mute state information from XML (legacy function for backward compatibility)
pub fn parse_mute_state(xml: &str) -> Result<Option<bool>> {
    let parser = RenderingControlParser::from_xml(xml)?;
    Ok(parser.muted())
}

/// Parse a complete RenderingControl event (legacy function for backward compatibility)
pub fn parse_rendering_control_event(xml: &str) -> Result<super::types::XmlRenderingControlData> {
    let parser = RenderingControlParser::from_xml(xml)?;
    Ok(super::types::XmlRenderingControlData {
        volume: parser.volume(),
        muted: parser.muted(),
    })
}

// Internal helper functions
fn decode_html_entities(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

fn parse_boolean_value(value: &str) -> bool {
    match value {
        "1" | "true" | "True" | "TRUE" => true,
        _ => false,
    }
}

fn extract_master_channel_value(xml: &str, element_name: &str) -> Option<String> {
    // Use regex to find the Master channel value for the given element
    let pattern = format!(r#"<{}\s+channel="Master"\s+val="([^"]+)""#, element_name);
    
    if let Ok(regex) = regex::Regex::new(&pattern) {
        if let Some(captures) = regex.captures(xml) {
            return Some(captures[1].to_string());
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
                    <Volume>75</Volume>
                    <Mute>1</Mute>
                </property>
            </propertyset>
        "#;

        let parser = RenderingControlParser::from_xml(xml).unwrap();

        assert_eq!(parser.volume(), Some(75));
        assert_eq!(parser.muted(), Some(true));
    }

    #[test]
    fn test_parser_from_xml_with_lastchange() {
        let xml = r#"
            <propertyset>
                <property>
                    <LastChange>&lt;Event xmlns="urn:schemas-upnp-org:metadata-1-0/RCS/"&gt;&lt;InstanceID val="0"&gt;&lt;Volume channel="Master" val="50"/&gt;&lt;Mute channel="Master" val="0"/&gt;&lt;/InstanceID&gt;&lt;/Event&gt;</LastChange>
                </property>
            </propertyset>
        "#;

        let parser = RenderingControlParser::from_xml(xml).unwrap();

        assert_eq!(parser.volume(), Some(50));
        assert_eq!(parser.muted(), Some(false));
    }

    #[test]
    fn test_parser_with_namespace_prefixes() {
        let xml = r#"<e:propertyset xmlns:e="urn:schemas-upnp-org:event-1-0">
            <e:property>
                <Volume>25</Volume>
                <Mute>0</Mute>
            </e:property>
        </e:propertyset>"#;

        let parser = RenderingControlParser::from_xml(xml).unwrap();
        assert_eq!(parser.volume(), Some(25));
        assert_eq!(parser.muted(), Some(false));
    }

    #[test]
    fn test_parser_empty_values() {
        let xml = r#"
            <propertyset>
                <property>
                    <Volume></Volume>
                    <Mute></Mute>
                </property>
            </propertyset>
        "#;

        let parser = RenderingControlParser::from_xml(xml).unwrap();

        assert_eq!(parser.volume(), None); // Empty volume should return None
        assert_eq!(parser.muted(), Some(false)); // Empty mute should return false
    }

    #[test]
    fn test_parser_partial_data() {
        let xml = r#"
            <propertyset>
                <property>
                    <Volume>30</Volume>
                </property>
            </propertyset>
        "#;

        let parser = RenderingControlParser::from_xml(xml).unwrap();

        assert_eq!(parser.volume(), Some(30));
        assert_eq!(parser.muted(), None); // Missing mute should return None
    }

    #[test]
    fn test_parser_real_sonos_xml() {
        // Real XML from Sonos device with complex nested quotes
        let xml = r#"<e:propertyset xmlns:e="urn:schemas-upnp-org:event-1-0"><e:property><LastChange>&lt;Event xmlns=&quot;urn:schemas-upnp-org:metadata-1-0/RCS/&quot;&gt;&lt;InstanceID val=&quot;0&quot;&gt;&lt;Volume channel=&quot;Master&quot; val=&quot;21&quot;/&gt;&lt;Volume channel=&quot;LF&quot; val=&quot;100&quot;/&gt;&lt;Volume channel=&quot;RF&quot; val=&quot;100&quot;/&gt;&lt;Mute channel=&quot;Master&quot; val=&quot;0&quot;/&gt;&lt;Mute channel=&quot;LF&quot; val=&quot;0&quot;/&gt;&lt;Mute channel=&quot;RF&quot; val=&quot;0&quot;/&gt;&lt;Bass val=&quot;0&quot;/&gt;&lt;Treble val=&quot;0&quot;/&gt;&lt;Loudness channel=&quot;Master&quot; val=&quot;1&quot;/&gt;&lt;OutputFixed val=&quot;0&quot;/&gt;&lt;HeadphoneConnected val=&quot;0&quot;/&gt;&lt;SpeakerSize val=&quot;3&quot;/&gt;&lt;SubGain val=&quot;0&quot;/&gt;&lt;SubCrossover val=&quot;0&quot;/&gt;&lt;SubPolarity val=&quot;0&quot;/&gt;&lt;SubEnabled val=&quot;1&quot;/&gt;&lt;SonarEnabled val=&quot;1&quot;/&gt;&lt;SonarCalibrationAvailable val=&quot;1&quot;/&gt;&lt;PresetNameList val=&quot;FactoryDefaults&quot;/&gt;&lt;/InstanceID&gt;&lt;/Event&gt;</LastChange></e:property></e:propertyset>"#;

        let parser = RenderingControlParser::from_xml(xml).unwrap();

        assert_eq!(parser.volume(), Some(21)); // Should parse Master channel volume
        assert_eq!(parser.muted(), Some(false)); // Should parse Master channel mute (0 = false)
    }

    #[test]
    fn test_parser_real_sonos_xml_muted() {
        // Real XML with muted state
        let xml = r#"<e:propertyset xmlns:e="urn:schemas-upnp-org:event-1-0"><e:property><LastChange>&lt;Event xmlns=&quot;urn:schemas-upnp-org:metadata-1-0/RCS/&quot;&gt;&lt;InstanceID val=&quot;0&quot;&gt;&lt;Volume channel=&quot;Master&quot; val=&quot;50&quot;/&gt;&lt;Mute channel=&quot;Master&quot; val=&quot;1&quot;/&gt;&lt;/InstanceID&gt;&lt;/Event&gt;</LastChange></e:property></e:propertyset>"#;

        let parser = RenderingControlParser::from_xml(xml).unwrap();

        assert_eq!(parser.volume(), Some(50));
        assert_eq!(parser.muted(), Some(true)); // 1 = true (muted)
    }

    #[test]
    fn test_parser_multiple_channels() {
        // Test that we correctly pick the Master channel when multiple channels are present
        let xml = r#"
            <propertyset>
                <property>
                    <LastChange>&lt;Event xmlns="urn:schemas-upnp-org:metadata-1-0/RCS/"&gt;&lt;InstanceID val="0"&gt;&lt;Volume channel="LF" val="100"/&gt;&lt;Volume channel="Master" val="35"/&gt;&lt;Volume channel="RF" val="100"/&gt;&lt;Mute channel="LF" val="0"/&gt;&lt;Mute channel="Master" val="1"/&gt;&lt;Mute channel="RF" val="0"/&gt;&lt;/InstanceID&gt;&lt;/Event&gt;</LastChange>
                </property>
            </propertyset>
        "#;

        let parser = RenderingControlParser::from_xml(xml).unwrap();

        assert_eq!(parser.volume(), Some(35)); // Should get Master channel, not LF or RF
        assert_eq!(parser.muted(), Some(true)); // Should get Master channel mute state
    }

    #[test]
    fn test_parser_invalid_xml() {
        let invalid_xml = r#"<invalid>xml</invalid>"#;
        let result = RenderingControlParser::from_xml(invalid_xml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_boolean_value_helper() {
        assert_eq!(parse_boolean_value("1"), true);
        assert_eq!(parse_boolean_value("0"), false);
        assert_eq!(parse_boolean_value("true"), true);
        assert_eq!(parse_boolean_value("True"), true);
        assert_eq!(parse_boolean_value("TRUE"), true);
        assert_eq!(parse_boolean_value("false"), false);
        assert_eq!(parse_boolean_value("False"), false);
        assert_eq!(parse_boolean_value("invalid"), false);
        assert_eq!(parse_boolean_value(""), false);
    }

    // Legacy function tests for backward compatibility
    #[test]
    fn test_parse_volume_legacy() {
        let xml = r#"
            <propertyset>
                <property>
                    <Volume>75</Volume>
                </property>
            </propertyset>
        "#;
        let result = parse_volume(xml).unwrap();
        assert_eq!(result, Some(75));
    }

    #[test]
    fn test_parse_mute_state_legacy() {
        let xml = r#"
            <propertyset>
                <property>
                    <Mute>1</Mute>
                </property>
            </propertyset>
        "#;
        let result = parse_mute_state(xml).unwrap();
        assert_eq!(result, Some(true));
    }

    #[test]
    fn test_parse_rendering_control_event_legacy() {
        let xml = r#"
            <propertyset>
                <property>
                    <Volume>75</Volume>
                    <Mute>1</Mute>
                </property>
            </propertyset>
        "#;
        let result = parse_rendering_control_event(xml).unwrap();
        assert_eq!(result.volume, Some(75));
        assert_eq!(result.muted, Some(true));
    }
}
