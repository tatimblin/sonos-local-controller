#[derive(Debug)]
pub enum SonosError {
  CommunicationError(String),
  DeviceNotFound(String),
  DiscoveryFailed(String),
  ParseError(String),
  SoapFault(String),
}

impl std::fmt::Display for SonosError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SonosError::CommunicationError(msg) => write!(f, "Communication error: {}", msg),
            SonosError::DeviceNotFound(msg) => write!(f, "Device not found: {}", msg),
            SonosError::DiscoveryFailed(msg) => write!(f, "Discovery failed: {}", msg),
            SonosError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            SonosError::SoapFault(msg) => write!(f, "SOAP fault: {}", msg),
        }
    }
}

impl std::error::Error for SonosError {}

pub type Result<T> = std::result::Result<T, SonosError>;
