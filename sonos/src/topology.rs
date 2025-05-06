use serde_derive::Deserialize;
use xmltree::Element;

use std::fs::OpenOptions;
use std::io::Write;

use crate::{model::Action, Client, SonosError};

// START

#[derive(Debug, Deserialize)]
#[serde(rename = "GetZoneGroupStateResponse")]
struct GetZoneGroupStateResponse {
    #[serde(rename = "ZoneGroupState")]
    zone_group_state: ZoneGroupStateWrapper,
}

#[derive(Debug, Deserialize)]
struct ZoneGroupStateWrapper {
    #[serde(rename = "ZoneGroupState")]
    zone_group_state: ZoneGroupState,
}


#[derive(Debug, Deserialize)]
struct ZoneGroupState {
    #[serde(rename = "ZoneGroups")]
    zone_groups: ZoneGroups,
    
    #[serde(rename = "VanishedDevices")]
    vanished_devices: Option<VanishedDevices>,
}

#[derive(Debug, Deserialize)]
struct ZoneGroups {
    #[serde(rename = "ZoneGroup")]
    zone_group: Vec<ZoneGroup>,
}

#[derive(Debug, Deserialize)]
struct ZoneGroup {
    #[serde(rename = "@Coordinator")]
    coordinator: String,

    #[serde(rename = "@ID")]
    id: String,

    #[serde(rename = "ZoneGroupMember")]
    members: Vec<ZoneGroupMember>,
}

#[derive(Debug, Deserialize)]
struct ZoneGroupMember {
    #[serde(rename = "@UUID")]
    uuid: String,

    #[serde(rename = "@Location")]
    location: String,

    #[serde(rename = "@ZoneName")]
    zone_name: String,

    #[serde(rename = "@SoftwareVersion")]
    software_version: String,

    #[serde(rename = "Satellite")]
    satellites: Option<Vec<Satellite>>,
}

#[derive(Debug, Deserialize)]
struct Satellite {
    #[serde(rename = "@UUID")]
    uuid: String,

    #[serde(rename = "@Location")]
    location: String,

    #[serde(rename = "@ZoneName")]
    zone_name: String,
}

#[derive(Debug, Deserialize)]
struct VanishedDevices {}

// END

#[derive(Debug, Deserialize)]
pub struct Topology {
  #[serde(rename = "GetZoneGroupStateResponse")]
  boop: GetZoneGroupStateResponse,
}

impl Topology {
  pub fn from_ip(ip: &str) -> Result<Self, SonosError> {
    let client = Client::default();
    let payload = "<InstanceID>0</InstanceID>";

    match client.send_action(ip, Action::GetZoneGroupState, payload) {
      Ok(response) => {
        // log to file?
        let mut file = OpenOptions::new()
          .create(true)
          .append(true)
          .open("log.txt")
          .expect("Failed to open log file");

        let _ = file.write_all(element_to_str(&response).as_bytes());
        // end log to file?

        match Self::from_xml(&element_to_str(&response)) {
          Ok(topology) => Ok(topology),
          Err(e) => Err(e),
        }
      },
      Err(e) => {
        println!("Failed to parse Topology 2: {}", e);
        Err(e)
      },
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
