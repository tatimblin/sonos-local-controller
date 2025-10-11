#[derive(Debug)]
pub enum SonosError {
  CommunicationError(String),
  DeviceNotFound(String),
  SoapFault(String),
}

pub type Result<T> = std::result::Result<T, SonosError>;
