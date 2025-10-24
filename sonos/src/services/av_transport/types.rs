use serde::{Deserialize, Serialize};

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

    #[test]
    fn test_didl_lite_conversion() {
        let didl_item = XmlDidlItem {
            title: Some("Test Title".to_string()),
            artist: Some("Test Artist".to_string()),
            album: Some("Test Album".to_string()),
        };

        let didl_lite = XmlDidlLite {
            item: Some(didl_item),
        };

        let metadata: XmlDidlMetadata = didl_lite.into();
        assert_eq!(metadata.title, Some("Test Title".to_string()));
        assert_eq!(metadata.artist, Some("Test Artist".to_string()));
        assert_eq!(metadata.album, Some("Test Album".to_string()));
    }

    #[test]
    fn test_empty_didl_lite_conversion() {
        let didl_lite = XmlDidlLite { item: None };
        let metadata: XmlDidlMetadata = didl_lite.into();
        assert_eq!(metadata.title, None);
        assert_eq!(metadata.artist, None);
        assert_eq!(metadata.album, None);
    }
}