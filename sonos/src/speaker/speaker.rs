use std::fmt::Debug;

use serde_derive::Deserialize;
use xmltree::Element;

use crate::model::Action;
use crate::speaker::Device;
use crate::error::SonosError;
use crate::util::http;
use crate::client::Client;

pub trait SpeakerFactory: Sized {
  fn default() -> Speaker;
  fn from_location(ip: &str) -> Result<Speaker, SonosError>;
  fn get_xml(endpoint: &str) -> Result<String, SonosError>;
  fn from_xml(xml: &str) -> Result<Speaker, SonosError>;
}

pub trait SpeakerTrait: Debug {
  fn name(&self) -> &str;
  fn room_name(&self) -> &str;
  fn ip(&self) -> &str;
  fn uuid(&self) -> &str;

  fn play(&self) -> Result<(), SonosError>;
  fn pause(&self) -> Result<(), SonosError>;
  fn get_volume(&self) -> Result<u8, SonosError>;
  fn set_volume(&self, volume: u8) -> Result<u8, SonosError>;
  fn adjust_volume(&self, adjustment: i8) -> Result<u8, SonosError>;

  fn parse_element_u8(&self, element: &Element, key: &str) -> Result<u8, SonosError>;
}

#[derive(Clone, Debug, Deserialize, Default)]
pub struct Speaker {
  #[serde(rename = "device")]
  device: Device,

  #[serde(skip)]
  client: Client,

  #[serde(skip)]
  ip: String,
}

impl SpeakerFactory for Speaker {
  fn default() -> Speaker {
    Speaker {
      device: Device::default(),
      client: Client::default(),
      ip: "0.0.0.0".to_owned(),
    }
  }

  fn from_location(location: &str) -> Result<Speaker, SonosError> {
    let xml = Self::get_xml(location)?;
    let ip = match http::get_ip_from_url(location) {
      Some(ip) => ip,
      None => return Err(SonosError::ParseError("Invalid ip".to_string())),
    };

    let mut speaker: Speaker = Self::from_xml(&xml)?;
    speaker.ip = ip;
    Ok(speaker)
  }

  fn get_xml(endpoint: &str) -> Result<String, SonosError> {
    match ureq::get(endpoint).call() {
      Ok(response) => response
        .into_string()
        .map_err(|_| SonosError::ParseError("Failed to read response body".into())),
      Err(_) => Err(SonosError::DeviceUnreachable)
    }
  }

  fn from_xml(xml: &str) -> Result<Self, SonosError> {
    serde_xml_rs::from_str(xml).map_err(|e| SonosError::ParseError(format!("Failed to parse Speaker: {}", e)))
  }
}

impl SpeakerTrait for Speaker {
  fn name(&self) -> &str {
    &self.device.name
  }

  fn room_name(&self) -> &str {
    &self.device.room_name
  }

  fn ip(&self) -> &str {
    &self.ip
  }

  fn uuid(&self) -> &str {
    &self.device.udn
  }
 
  // Controls
  fn play(&self) -> Result<(), SonosError> {
    let payload = "<InstanceID>0</InstanceID><Speed>1</Speed>";
    match self.client.send_action(&self.ip.to_string(), Action::Play, payload) {
      Ok(_) => Ok(()),
      Err(e) => Err(e),
    }
  }

  fn pause(&self) -> Result<(), SonosError> {
    let payload: &str = "<InstanceID>0</InstanceID>";
    match self.client.send_action(&self.ip.to_string(), Action::Pause, payload) {
      Ok(_) => Ok(()),
      Err(e) => Err(e),
    }
  }

  fn get_volume(&self) -> Result<u8, SonosError> {
    let payload = "<InstanceID>0</InstanceID><Channel>Master</Channel>";
    match self.client.send_action(&self.ip.to_string(), Action::GetVolume, payload) {
      Ok(response) => self.parse_element_u8(&response, "GetVolume"),
      Err(err) => Err(err),
    }
  }

  fn set_volume(&self, volume: u8) -> Result<u8, SonosError> {
    let payload = format!("<InstanceID>0</InstanceID><Channel>Master</Channel><DesiredVolume>{}</DesiredVolume>", volume);
    match self.client.send_action(&self.ip.to_string(), Action::SetVolume, &payload) {
      Ok(response) => {
        let result = self.parse_element_u8(&response, "NewVolume");
        // TODO: update local state
        result
      },
      Err(err) => Err(err),
    }
  }

  fn adjust_volume(&self, adjustment: i8) -> Result<u8, SonosError> {
    let payload = format!("<InstanceID>0</InstanceID><Channel>Master</Channel><Adjustment>{}</Adjustment>", adjustment);
    match self.client.send_action(&self.ip.to_owned(), Action::SetRelativeVolume, &payload) {
      Ok(response) => {
        let result = self.parse_element_u8(&response, "NewVolume");
        // TODO: update local state
        result
      }
      Err(err) => Err(err),
    }
  }

  // TODO: this should probably be built in to the client
  fn parse_element_u8(&self, element: &Element, key: &str) -> Result<u8, SonosError> {
    self.client
      .get_child_element_text(element, key)?
      .parse()
      .map_err(|e| SonosError::ParseError(format!("Failed to parse {}: {}", key, e)))
  }
}
