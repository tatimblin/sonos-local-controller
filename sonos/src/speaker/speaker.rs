use ureq::{Agent, Error};
use xmltree::Element;

use crate::{
  SonosError,
  SpeakerInfo,
};
use crate::util::http;

use super::api::{
  Action,
  get_child_element_text,
  parse_xml_response,
};

#[derive(Debug)]
pub struct Speaker {
  pub ip: String,
  agent: Agent,
  speaker_info: SpeakerInfo,
}

impl Speaker {
  pub fn from_location(location: &str) -> Result<Speaker, SonosError> {
    let xml = Self::get_speaker_info_xml(location)?;
    let ip = match http::get_ip_from_url(location) {
      Some(ip) => ip,
      None => return Err(SonosError::ParseError("Invalid ip".to_string())),
    };

    match SpeakerInfo::from_xml(&xml) {
      Ok(speaker_info) => Ok(Speaker{
        ip: ip,
        agent: Agent::new(),
        speaker_info: speaker_info,
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

  pub fn play(&self) {
    match self.send_action(Action::Play, "<InstanceID>0</InstanceID><Speed>1</Speed>"
    ) {
      Ok(response) => println!("Response: {:?}", response),
      Err(error) => eprintln!("Error: {}", error),
    }
  }

  pub fn pause(&self) {
    match self.send_action(Action::Pause, "<InstanceID>0</InstanceID>"
    ) {
      Ok(response) => println!("Response: {:?}", response),
      Err(error) => eprintln!("Error: {}", error),
    }
  }

  pub fn get_volume(&self) -> Result<u8, SonosError> {
    match self.send_action(Action::GetVolume, "<InstanceID>0</InstanceID><Channel>Master</Channel>") {
      Ok(response) => {
        let volume = get_child_element_text(&response, "CurrentVolume")?
          .parse::<u8>()
          .map_err(|e| SonosError::ParseError(format!("Failed to parse volume: {}", e)))?;
        Ok(volume)
      },
      Err(error) => Err(error),
    }
  }

  // TODO: Move to api.rs
  fn send_action(&self, action: Action, payload: &str) -> Result<Element, SonosError> {
    let body = format!(r#"
      <s:Envelope
        xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"
        s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding"
      >
        <s:Body>
          <u:{action} xmlns:u="{service}">
            {payload}
          </u:{action}>
        </s:Body>
      </s:Envelope>
    "#,
      action = action.name(),
      payload = payload,
      service = action.service()
    );

    let soap_action = format!("\"{}#{}\"", action.service(), action.name());
    let url = format!("http://{}:1400/{}", self.ip, action.endpoint());

    let response = self.agent.post(&url)
      .set("Content-Type", "text/xml; charset=\"utf-8\"")
      .set("SOAPACTION", &soap_action)
      .send_string(&body);

    match response {
      Ok(response) => parse_xml_response(response, action),
      Err(_) => Err(SonosError::DeviceUnreachable),
    }
  }
}

