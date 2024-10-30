use ureq::Error;

use crate::{
  SonosError,
  SpeakerInfo,
};

pub struct Speaker {
  pub name: String,
}

impl Speaker {
  pub fn from_ip(ip: String) -> Result<Speaker, SonosError> {
    let xml = Self::get_speaker_info_xml(ip)?;
    let speaker_info = SpeakerInfo::from_xml(&xml);

    println!("{:#?}", speaker_info);

    Ok(Speaker {
      name: "Tristan's Speaker".to_string(),
    })
  }

  fn get_speaker_info_xml(ip: String) -> Result<String, SonosError> {
    match ureq::get(&ip).call() {
      Ok(response) => response
        .into_string()
        .map_err(|_| SonosError::ParseError("Failed to read response body".into())),
      Err(Error::Status(code, _)) => Err(SonosError::BadResponse(code)),
      Err(_) => {
        Err(SonosError::DeviceUnreachable)
      }
    }
  }
}

