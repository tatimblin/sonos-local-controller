use serde::{Deserialize, Serialize};

/// XML data structure for rendering control event data
#[derive(Debug, Clone)]
pub struct XmlRenderingControlData {
    pub volume: Option<u8>,
    pub muted: Option<bool>,
}

/// LastChange event structure for nested XML
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename = "Event")]
pub struct XmlLastChangeEvent {
    #[serde(rename = "InstanceID")]
    pub instance_id: Option<XmlInstanceId>,
}

/// InstanceID structure within LastChange events
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct XmlInstanceId {
    #[serde(rename = "@val")]
    pub val: Option<String>,
    #[serde(rename = "Volume", default)]
    pub volume: Vec<XmlVolumeChannel>,
    #[serde(rename = "Mute", default)]
    pub mute: Vec<XmlMuteChannel>,
    #[serde(rename = "TransportState")]
    pub transport_state: Option<XmlValueAttribute>,
    #[serde(rename = "CurrentTrackMetaData")]
    pub current_track_metadata: Option<XmlValueAttribute>,
    #[serde(rename = "CurrentTrackDuration")]
    pub current_track_duration: Option<XmlValueAttribute>,
    #[serde(rename = "CurrentTrackURI")]
    pub current_track_uri: Option<XmlValueAttribute>,
}

/// Generic structure for elements with val attribute
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct XmlValueAttribute {
    #[serde(rename = "@val")]
    pub val: String,
}

/// Volume channel structure with channel and val attributes
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct XmlVolumeChannel {
    #[serde(rename = "@channel")]
    pub channel: String,
    #[serde(rename = "@val")]
    pub val: String,
}

/// Mute channel structure with channel and val attributes
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct XmlMuteChannel {
    #[serde(rename = "@channel")]
    pub channel: String,
    #[serde(rename = "@val")]
    pub val: String,
}

/// UPnP property wrapper (includes rendering control properties)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct XmlProperty {
    #[serde(rename = "ZoneGroupState")]
    pub zone_group_state: Option<String>,
    #[serde(rename = "Volume")]
    pub volume: Option<String>,
    #[serde(rename = "Mute")]
    pub mute: Option<String>,
    #[serde(rename = "TransportState")]
    pub transport_state: Option<String>,
    #[serde(rename = "CurrentTrackMetaData")]
    pub current_track_metadata: Option<String>,
    #[serde(rename = "CurrentTrackDuration")]
    pub current_track_duration: Option<String>,
    #[serde(rename = "CurrentTrackURI")]
    pub current_track_uri: Option<String>,
    #[serde(rename = "LastChange")]
    pub last_change: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xml_rendering_control_data() {
        let rendering_data = XmlRenderingControlData {
            volume: Some(50),
            muted: Some(false),
        };
        assert_eq!(rendering_data.volume, Some(50));
        assert_eq!(rendering_data.muted, Some(false));
    }

    #[test]
    fn test_xml_value_attribute() {
        let value_attr = XmlValueAttribute {
            val: "50".to_string(),
        };
        assert_eq!(value_attr.val, "50");
    }

    #[test]
    fn test_xml_instance_id() {
        let instance_id = XmlInstanceId {
            val: Some("0".to_string()),
            volume: vec![XmlVolumeChannel { 
                channel: "Master".to_string(), 
                val: "50".to_string() 
            }],
            mute: vec![XmlMuteChannel { 
                channel: "Master".to_string(), 
                val: "0".to_string() 
            }],
            transport_state: None,
            current_track_metadata: None,
            current_track_duration: None,
            current_track_uri: None,
        };
        assert_eq!(instance_id.val, Some("0".to_string()));
        assert!(!instance_id.volume.is_empty());
        assert!(!instance_id.mute.is_empty());
    }
}