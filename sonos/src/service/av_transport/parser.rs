use crate::error::{Result, SonosError};
use serde::Deserialize;

#[derive(Debug)]
pub struct AVTransportParser {
    property_set: Option<AVTransportPropertySet>,
}

#[derive(Debug, Deserialize)]
struct AVTransportPropertySet {
  #[serde(rename = "property")]
  property: AVTransportProperty,
}

#[derive(Debug, Deserialize)]
struct AVTransportProperty {
  #[serde(rename = "LastChange", default)]
  last_change: Option<String>,
  #[serde(rename = "TransportState", default)]
  transport_state: Option<String>,
  #[serde(rename = "CurrentTrackURI", default)]
  current_track_uri: Option<String>,
  #[serde(rename = "CurrentTrackDuration", default)]
  current_track_duration: Option<String>,
  #[serde(rename = "CurrentTrackMetaData", default)]
  current_track_metadata: Option<String>,
}

impl AVTransportParser {
    pub fn from_xml(xml: &str) -> Result<Self> {
        let cleaned_xml = xml
            .replace("e:propertyset", "propertyset")
            .replace("e:property", "property");

        let property_set = serde_xml_rs::from_str(&cleaned_xml)
            .map_err(|e| SonosError::ParseError(format!("PropertySet parse error: {}", e)))?;

        Ok(Self {
            property_set: Some(property_set),
        })
    }
}
