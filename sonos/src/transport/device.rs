use crate::error::{Result, SonosError};
use crate::model::{Speaker, SpeakerId};
use serde::Deserialize;

/// UPnP device description root element
#[derive(Debug, Deserialize)]
pub struct Root {
  pub device: Device,
}

/// UPnP device information
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
  pub device_type: String,
  pub friendly_name: String,
  pub manufacturer: String,
  pub manufacturer_url: Option<String>,
  pub model_description: Option<String>,
  pub model_name: String,
  pub model_number: Option<String>,
  pub model_url: Option<String>,
  pub serial_number: Option<String>,
  #[serde(rename = "UDN")]
  pub udn: SpeakerId,
  pub room_name: Option<String>,
  pub display_name: Option<String>,
}

impl Device {
  /// Parse device XML from a URL response
  pub fn from_xml(xml: &str) -> Result<Self> {
    let root: Root = quick_xml::de::from_str(xml)
      .map_err(|e| SonosError::ParseError(format!("Failed to parse device XML: {}", e)))?;

    Ok(root.device)
  }

  /// Convert device to Speaker model with IP address
  pub fn to_speaker(&self, ip_address: String) -> Speaker {
    Speaker {
      id: self.udn.clone(),
      name: self.friendly_name.clone(),
      room_name: self
        .room_name
        .clone()
        .unwrap_or_else(|| "Unknown".to_string()),
      ip_address,
      port: 1400,
      model_name: self.model_name.clone(),
      satellites: vec![],
    }
  }

  /// Check if this device is a Sonos speaker
  pub fn is_sonos_speaker(&self) -> bool {
    self.manufacturer.to_lowercase().contains("sonos")
      || self.device_type.contains("ZonePlayer")
      || self.device_type.contains("MediaRenderer")
  }
}

/// Extract IP address from a URL
pub fn extract_ip_from_url(url: &str) -> Option<String> {
  url.split("//")
    .nth(1)?
    .split(':')
    .next()
    .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_extract_ip_from_url() {
    assert_eq!(
      extract_ip_from_url("http://192.168.1.100:1400/xml/device_description.xml"),
      Some("192.168.1.100".to_string())
    );
    assert_eq!(
      extract_ip_from_url("https://10.0.0.5:8080/path"),
      Some("10.0.0.5".to_string())
    );
    assert_eq!(extract_ip_from_url("invalid-url"), None);
  }

  #[test]
  fn test_device_from_xml() {
    let xml = include_str!("../../tests/fixtures/sonos_one_device.xml");

    let device = Device::from_xml(xml).unwrap();

    assert_eq!(device.friendly_name, "Living Room");
    assert_eq!(device.manufacturer, "Sonos, Inc.");
    assert_eq!(device.model_name, "Sonos One");
    assert_eq!(device.udn, SpeakerId::new("uuid:RINCON_000E58A0123456"));
    assert_eq!(device.room_name, Some("Living Room".to_string()));
    assert!(device.is_sonos_speaker());
  }
}
