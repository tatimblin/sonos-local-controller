use std::net::IpAddr;

use crate::error::SonosError;

#[derive(Debug)]
pub struct Speaker {
  pub ip: IpAddr,
  pub model: String,
  pub model_number: String,
  pub software_version: String,
  pub hardware_version: String,
  pub serial_number: String,
  pub name: String,
  pub uuid: String,
}

impl Speaker {
  pub async fn from_ip(ip: IpAddr) -> Result<Speaker, SonosError> {

    if ip.is_unspecified() {
      return Err(SonosError::BadResponse(400).into());
    }

    Ok(Speaker {
      ip,
      model: "mock".to_string(),
      model_number: "mock".to_string(),
      software_version: "mock".to_string(),
      hardware_version: "mock".to_string(),
      serial_number: "mock".to_string(),
      name: "mock".to_string(),
      uuid: "mock".to_string(),
    })
  }
}
