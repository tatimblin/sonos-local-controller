use serde_derive::Deserialize;

use crate::speaker::Device;
use crate::error::SonosError;

#[derive(Debug, Deserialize)]
pub struct SpeakerInfo {
  #[serde(rename = "device")]
  device: Device,
}

impl SpeakerInfo {
  pub fn from_xml(xml: &str) -> Result<Self, SonosError> {
    serde_xml_rs::from_str(xml).map_err(|e| SonosError::ParseError(format!("Failed to parse SpeakerInfo: {}", e)))
  }

  pub fn get_name(&self) -> &str {
    &self.device.name
  }

  pub fn get_room_name(&self) -> &str {
    &self.device.room_name
  }
}
