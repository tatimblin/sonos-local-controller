use std::fmt;

#[derive(Debug, Clone)]
pub enum SonosError {
  ParseError(String),
  DeviceUnreachable,
  BadResponse(u16),
  DeviceNotFound(String),
  NetworkTimeout,
  NetworkError(String),
  InvalidVolume(u8),
}

impl fmt::Display for SonosError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      SonosError::ParseError(msg) => write!(f, "Failed to parse Sonos response XML ({})", msg),
      SonosError::DeviceUnreachable => write!(f, "Failed to call Sonos endpoint"),
      SonosError::BadResponse(code) => write!(f, "Received a non-success ({}) response from Sonos", code),
      SonosError::DeviceNotFound(identifier) => write!(f, "Couldn't find a device by the given identifier ({})", identifier),
      SonosError::NetworkTimeout => write!(f, "Network request timed out"),
      SonosError::NetworkError(msg) => write!(f, "Network error: {}", msg),
      SonosError::InvalidVolume(volume) => write!(f, "Invalid volume level: {} (must be 0-100)", volume),
    }
  }
}

impl std::error::Error for SonosError {}
