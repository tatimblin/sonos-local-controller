use std::collections::{HashMap, HashSet};

use crate::{model::Action, Client, SonosError};

// START
use std::str;
use xmltree::Element;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TopologyResponse {
    pub players: Vec<Player>,
    pub groups: Vec<Group>,
    pub household_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Player {
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    #[serde(rename = "deviceId")]
    pub device_id: String,
    #[serde(rename = "householdId")]
    pub household_id: String,
    pub id: String,
    #[serde(rename = "isGroupCoordinator")]
    pub is_group_coordinator: bool,
    pub name: String,
    #[serde(rename = "restUrl")]
    pub rest_url: String,
    #[serde(rename = "softwareVersion")]
    pub software_version: String,
    #[serde(rename = "deviceIp")]
    pub device_ip: String,
    #[serde(rename = "webSocketUrl")]
    pub websocket_url: String,
    #[serde(rename = "groupId")]
    pub group_id: Option<String>,
    pub capabilities: Vec<String>,
    #[serde(rename = "deviceCapabilities")]
    pub device_capabilities: DeviceCapabilities,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceCapabilities {
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
    pub features: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {
    pub name: String,
    #[serde(rename = "type")]
    pub input_type: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Output {
    pub name: String,
    #[serde(rename = "type")]
    pub output_type: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    #[serde(rename = "playbackState")]
    pub playback_state: String,
    #[serde(rename = "playerIds")]
    pub player_ids: Vec<String>,
    #[serde(rename = "coordinatorId")]
    pub coordinator_id: String,
    #[serde(rename = "groupLabel")]
    pub group_label: String,
    #[serde(rename = "mediaSessionId")]
    pub media_session_id: Option<String>,
    #[serde(rename = "streamInfo")]
    pub stream_info: Option<StreamInfo>,
    #[serde(rename = "currentTrack")]
    pub current_track: Option<Track>,
    pub attributes: HashMap<String, String>,
    pub member_uuids: HashSet<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StreamInfo {
    pub service: Option<String>,
    #[serde(rename = "serviceId")]
    pub service_id: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Track {
    pub artist: Option<String>,
    pub album: Option<String>,
    pub track: Option<String>,
    #[serde(rename = "albumArtUri")]
    pub album_art_uri: Option<String>,
    pub duration: Option<i32>,
    #[serde(rename = "trackNumber")]
    pub track_number: Option<i32>,
    pub provider: Option<String>,
}

// END

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Topology {
  pub players: Vec<Player>,
  pub groups: Vec<Group>,
  pub household_id: String,
}

impl Topology {
  pub fn from_ip(ip: &str) -> Result<Self, SonosError> {
    let client = Client::default();
    let payload = "<InstanceID>0</InstanceID>";

    match client.send_action(ip, Action::GetZoneGroupTopology, payload) {
      Ok(response) => {
        match Self::from_xml(&element_to_str(&response)) {
          Ok(topology) => Ok(topology),
          Err(e) => Err(e),
        }
      },
      Err(e) => Err(e),
    }
  }

  pub fn from_xml(xml: &str) -> Result<Self, SonosError> {
    serde_xml_rs::from_str(xml).map_err(|e| SonosError::ParseError(format!("Failed to parse Topology: {}", e)))
  }
}

fn element_to_str(element: &Element) -> String {
  let mut buffer = Vec::new();
  element.write(&mut buffer).expect("Failed to write XML element");
  String::from_utf8_lossy(&buffer).into_owned()
}
