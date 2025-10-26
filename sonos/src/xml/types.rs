// Re-export types from service-specific modules
pub use crate::services::zone_group_topology::types::{XmlZoneGroupData, XmlZoneGroupMember, XmlZoneGroups};
pub use crate::services::rendering_control::types::XmlRenderingControlData;
pub use crate::services::av_transport::types::XmlAVTransportData;

use serde::{Deserialize, Serialize};

/// UPnP propertyset wrapper (root element)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename = "propertyset")]
pub struct XmlPropertySet {
    #[serde(rename = "property")]
    pub property: XmlProperty,
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
    #[serde(rename = "InstanceID", default)]
    pub instance_id: Option<XmlInstanceId>,
}

/// InstanceID structure within LastChange events
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct XmlInstanceId {
    #[serde(rename = "@val", default)]
    pub val: Option<String>,
    #[serde(rename = "Volume", default)]
    pub volume: Vec<XmlVolumeChannel>,
    #[serde(rename = "Mute", default)]
    pub mute: Vec<XmlMuteChannel>,
    #[serde(rename = "TransportState", default)]
    pub transport_state: Option<XmlValueAttribute>,
    #[serde(rename = "CurrentTrackMetaData", default)]
    pub current_track_metadata: Option<XmlValueAttribute>,
    #[serde(rename = "CurrentTrackDuration", default)]
    pub current_track_duration: Option<XmlValueAttribute>,
    #[serde(rename = "CurrentTrackURI", default)]
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