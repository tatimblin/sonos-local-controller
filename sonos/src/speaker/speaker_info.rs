use crate::speaker::DeviceRoot;
use crate::SonosError;
use serde_derive::Deserialize;

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct SpeakerInfo {
    /// IP address of the speaker
    pub ip: String,
    /// Friendly name of the speaker (e.g., "Living Room")
    pub name: String,
    /// Room name where the speaker is located
    pub room_name: String,
    /// Unique identifier (UUID) of the speaker
    pub uuid: String,
    /// Model name of the speaker (e.g., "Sonos One")
    pub model: String,
    /// Software version running on the speaker
    pub software_version: String,
}

impl SpeakerInfo {
    pub fn from_location(ip: &str) -> Result<SpeakerInfo, SonosError> {
        let location = format!("http://{}:1400/xml/device_description.xml", ip);
        let xml = Self::get_xml(&location)?;

        let mut speaker = Self::from_xml(&xml)?;
        speaker.ip = ip.to_string();
        Ok(speaker)
    }

    fn get_xml(endpoint: &str) -> Result<String, SonosError> {
        match ureq::get(endpoint).call() {
            Ok(response) => response
                .into_string()
                .map_err(|_| SonosError::ParseError("Failed to read response body".into())),
            Err(_) => Err(SonosError::DeviceUnreachable),
        }
    }

   pub fn from_xml(xml: &str) -> Result<Self, SonosError> {
      log::debug!("Attempting to parse XML: {}", xml);

        let root: DeviceRoot = serde_xml_rs::from_str(xml).map_err(|e| {
            log::debug!("XML parsing failed with error: {}", e);
            log::debug!("XML content that failed to parse: {}", xml);
            SonosError::ParseError(format!("Failed to parse DeviceRoot: {}", e))
        })?;

        let device = root.device;
        log::debug!("Successfully parsed device: {:?}", device);

        Ok(SpeakerInfo {
            ip: String::new(), // This will be set by the caller
            name: device.name,
            room_name: device.room_name,
            uuid: device.udn,
            model: device.model_name,
            software_version: device.software_version,
        })
    }
}
