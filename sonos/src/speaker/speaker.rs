use ureq::{Agent, Error};

use crate::{
  SonosError,
  SpeakerInfo,
};
use crate::util::http;

use super::api::Action;

pub struct Speaker {
  pub ip: String,
  agent: Agent,
  pub name: String,
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
        name: speaker_info.device.name,
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

  pub fn mute(&self) {
    match self.send_action(Action::SetMute, "<InstanceID>0</InstanceID><Channel>Master</Channel><DesiredMute>1</DesiredMute>"
    ) {
      Ok(response) => println!("Response: {}", response),
      Err(error) => eprintln!("Error: {}", error),
    }
  }

  fn send_action(&self, action: Action, payload: &str) -> Result<String, String> {
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
      service = action.service().as_str()
    );

    let soap_action = format!("\"{}#{}\"", action.service().as_str(), action.name());
    let url = format!("http://{}:1400/{}", self.ip, action.endpoint().as_str());

    let response = self.agent.post(&url)
      .set("Content-Type", "text/xml; charset=\"utf-8\"")
      .set("SOAPACTION", &soap_action)
      .send_string(&body);

    match response {
      Ok(res) => res.into_string().map_err(|e| e.to_string()),
      Err(err) => Err(format!("Failed to send command: {}", err)),
    }
  }
}

