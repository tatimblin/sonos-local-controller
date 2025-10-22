/// XML data structure for zone group information
#[derive(Debug, Clone)]
pub struct XmlZoneGroupData {
    pub coordinator: String,
    pub members: Vec<XmlZoneGroupMember>,
}

/// XML data structure for zone group member information
#[derive(Debug, Clone)]
pub struct XmlZoneGroupMember {
    pub uuid: String,
    pub satellites: Vec<String>,
}

/// XML data structure for DIDL metadata information
#[derive(Debug, Clone, PartialEq)]
pub struct XmlDidlMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
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
            satellites: vec!["RINCON_987654321".to_string()],
        };
        assert_eq!(member.uuid, "RINCON_123456789");
        assert_eq!(member.satellites.len(), 1);

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