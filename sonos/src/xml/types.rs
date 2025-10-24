use serde::{Deserialize, Serialize};

/// XML data structure for zone group information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct XmlZoneGroupData {
    #[serde(rename = "@Coordinator")]
    pub coordinator: String,
    #[serde(rename = "ZoneGroupMember", default)]
    pub members: Vec<XmlZoneGroupMember>,
}

/// XML data structure for zone group member information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct XmlZoneGroupMember {
    #[serde(rename = "@UUID")]
    pub uuid: String,
    #[serde(rename = "@Satellites", default)]
    pub satellites_attr: Option<String>,
    #[serde(rename = "Satellite", default)]
    pub satellite_elements: Vec<XmlSatellite>,
}

impl XmlZoneGroupMember {
    /// Get all satellites as a unified list
    pub fn satellites(&self) -> Vec<String> {
        let mut satellites = Vec::new();
        
        // Add satellites from attribute (comma-separated)
        if let Some(ref attr) = self.satellites_attr {
            for uuid in attr.split(',') {
                let uuid = uuid.trim();
                if !uuid.is_empty() {
                    satellites.push(uuid.to_string());
                }
            }
        }
        
        // Add satellites from nested elements
        for satellite in &self.satellite_elements {
            satellites.push(satellite.uuid.clone());
        }
        
        satellites
    }
}

/// XML data structure for satellite speakers
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct XmlSatellite {
    #[serde(rename = "@UUID")]
    pub uuid: String,
}

/// Root structure for zone group topology
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename = "ZoneGroups")]
pub struct XmlZoneGroups {
    #[serde(rename = "ZoneGroup", default)]
    pub zone_groups: Vec<XmlZoneGroupData>,
}

/// UPnP property wrapper
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
    #[serde(rename = "Volume")]
    pub volume: Option<XmlValueAttribute>,
    #[serde(rename = "Mute")]
    pub mute: Option<XmlValueAttribute>,
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

/// DIDL-Lite metadata structure
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename = "DIDL-Lite")]
pub struct XmlDidlLite {
    #[serde(rename = "item")]
    pub item: Option<XmlDidlItem>,
}

/// DIDL-Lite item structure
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct XmlDidlItem {
    #[serde(rename = "title", alias = "dc:title")]
    pub title: Option<String>,
    #[serde(rename = "creator", alias = "dc:creator")]
    pub artist: Option<String>,
    #[serde(rename = "album", alias = "upnp:album")]
    pub album: Option<String>,
}

/// XML data structure for DIDL metadata information
#[derive(Debug, Clone, PartialEq)]
pub struct XmlDidlMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
}

impl From<XmlDidlLite> for XmlDidlMetadata {
    fn from(didl: XmlDidlLite) -> Self {
        if let Some(item) = didl.item {
            XmlDidlMetadata {
                title: item.title,
                artist: item.artist,
                album: item.album,
            }
        } else {
            XmlDidlMetadata {
                title: None,
                artist: None,
                album: None,
            }
        }
    }
}

/// XML data structure for rendering control event data
#[derive(Debug, Clone)]
pub struct XmlRenderingControlData {
    pub volume: Option<u8>,
    pub muted: Option<bool>,
}

/// XML data structure for AVTransport event data
#[derive(Debug, Clone)]
pub struct XmlAVTransportData {
    pub transport_state: Option<String>,
    pub current_track_metadata: Option<XmlDidlMetadata>,
    pub current_track_duration: Option<String>,
    pub current_track_uri: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xml_zone_group_data() {
        let member = XmlZoneGroupMember {
            uuid: "RINCON_123456789".to_string(),
            satellites_attr: Some("RINCON_987654321".to_string()),
            satellite_elements: vec![],
        };
        assert_eq!(member.uuid, "RINCON_123456789");
        assert_eq!(member.satellites().len(), 1);

        let zone_group = XmlZoneGroupData {
            coordinator: "RINCON_123456789".to_string(),
            members: vec![member],
        };
        assert_eq!(zone_group.coordinator, "RINCON_123456789");
        assert_eq!(zone_group.members.len(), 1);
    }

    #[test]
    fn test_xml_didl_metadata() {
        let metadata = XmlDidlMetadata {
            title: Some("Test Song".to_string()),
            artist: Some("Test Artist".to_string()),
            album: None,
        };
        assert_eq!(metadata.title, Some("Test Song".to_string()));
        assert_eq!(metadata.artist, Some("Test Artist".to_string()));
        assert_eq!(metadata.album, None);
    }

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
    fn test_xml_av_transport_data() {
        let metadata = XmlDidlMetadata {
            title: Some("Test Song".to_string()),
            artist: Some("Test Artist".to_string()),
            album: None,
        };

        let av_data = XmlAVTransportData {
            transport_state: Some("PLAYING".to_string()),
            current_track_metadata: Some(metadata),
            current_track_duration: Some("00:03:45".to_string()),
            current_track_uri: Some("x-sonos-spotify:spotify%3atrack%3a123".to_string()),
        };
        assert_eq!(av_data.transport_state, Some("PLAYING".to_string()));
        assert!(av_data.current_track_metadata.is_some());
        assert_eq!(av_data.current_track_duration, Some("00:03:45".to_string()));
    }
}