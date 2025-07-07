use std::fmt;

#[derive(Debug)]
pub enum SonosError {
  ParseError(String),
  DeviceUnreachable,
  BadResponse(u16),
  DeviceNotFound(String),
  NetworkTimeout,
}

impl fmt::Display for SonosError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      SonosError::ParseError(msg) => write!(f, "Failed to parse Sonos response XML ({})", msg),
      SonosError::DeviceUnreachable => write!(f, "Failed to call Sonos endpoint"),
      SonosError::BadResponse(code) => write!(f, "Received a non-success ({}) response from Sonos", code),
      SonosError::DeviceNotFound(identifier) => write!(f, "Couldn't find a device by the given identifier ({})", identifier),
      SonosError::NetworkTimeout => write!(f, "Network request timed out"),
    }
  }
}

impl std::error::Error for SonosError {}
