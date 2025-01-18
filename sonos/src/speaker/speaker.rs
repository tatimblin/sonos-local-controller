use ureq::{ Agent, Error };

use crate::{
  SonosError,
  SpeakerInfo,
};
use crate::util::http;
use crate::speaker::client::Client;
use crate::speaker::api::ApiClient;

#[derive(Debug, Default)]
pub struct Speaker {
  api: ApiClient,
  speaker_info: SpeakerInfo,

  volume: Option<u8>,
}

impl Speaker {
  fn default() -> Self {
    Self {
      api: ApiClient::default(),
      speaker_info: SpeakerInfo::default(),
      volume: None.into(),
    }
  }
  pub fn from_location(location: &str) -> Result<Speaker, SonosError> {
    let xml = Self::get_speaker_info_xml(location)?;
    let ip = match http::get_ip_from_url(location) {
      Some(ip) => ip,
      None => return Err(SonosError::ParseError("Invalid ip".to_string())),
    };

    match SpeakerInfo::from_xml(&xml) {
      Ok(speaker_info) => Ok(Speaker{
        api: ApiClient::new(Client::new(ip, Agent::new())),
        speaker_info: speaker_info,

        volume: None.into(),
      }),
      Err(err) => Err(err),
    }
  }

  fn get_speaker_info_xml(endpoint: &str) -> Result<String, SonosError> {
    match ureq::get(endpoint).call() {
      Ok(response) => response
        .into_string()
        .map_err(|_| SonosError::ParseError("Failed to read response body".into())),
      Err(Error::Status(code, _)) => Err(SonosError::BadResponse(code)),
      Err(_) => {
        Err(SonosError::DeviceUnreachable)
      }
    }
  }

  pub fn get_info(&self) -> &SpeakerInfo {
    &self.speaker_info
  }

  pub fn play(&self) -> Result<(), SonosError> {
    self.api.play()
  }

  pub fn pause(&self) -> Result<(), SonosError> {
    self.api.pause()
  }


  pub fn get_volume(&self) -> Result<u8, SonosError> {
    self.api.get_volume()
  }

  pub fn set_volume(&self, volume: u8) -> Result<u8, SonosError> {
    self.api.set_volume(volume)
  }

  pub fn set_relative_volume(&self, adjustment: i8) -> Result<u8, SonosError> {
    self.api.set_relative_volume(adjustment)
  }
}

