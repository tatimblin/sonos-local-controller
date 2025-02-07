use std::collections::{HashMap, HashSet};

use std::fs::OpenOptions;
use std::io::Write;

use crate::{model::Action, Client, SonosError};

// START



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

    println!("Sending GetZoneGroupTopology to {}", ip);

    match client.send_action(ip, Action::GetZoneGroupState, payload) {
      Ok(response) => {
        let mut file = OpenOptions::new()
          .create(true)
          .append(true)
          .open("log.txt")
          .expect("Failed to open log file");

        let _ = file.write_all(element_to_str(&response).as_bytes());

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
