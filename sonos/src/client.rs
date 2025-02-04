use ureq::{ Agent, Response };
use xmltree::Element;
use std::borrow::Cow;

use crate::SonosError;
use crate::model::Action;

#[derive(Debug)]
pub struct Client {
  agent: Agent,
}

impl Client {
  pub fn new(agent: Agent) -> Self {
    Self { agent }
  }

  pub fn send_action(&self, ip: &str, action: Action, payload: &str) -> Result<Element, SonosError> {
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
    let url = format!("http://{}:1400/{}", ip, action.endpoint());

    let response = self.agent.post(&url)
      .set("Content-Type", "text/xml; charset=\"utf-8\"")
      .set("SOAPACTION", &soap_action)
      .send_string(&body);

    match response {
      Ok(response) => self.parse_xml_response(response, action),
      Err(_) => Err(SonosError::DeviceUnreachable),
    }
  }

  fn parse_xml_response(&self, response: Response, action: Action) -> Result<Element, SonosError> {
    match response.into_string() {
      Ok(xml_string) => {
        let xml = Element::parse(xml_string.as_bytes())
          .map_err(|e| SonosError::ParseError(format!("Failed to parse XML: {}", e)))?;
  
        let body = self.get_child_element(&xml, "Body")?;
  
        if let Some(fault) = body.get_child("Fault") {
          let error_code = fault
            .get_child("detail")
            .and_then(|c| c.get_child("UpnPError"))
            .and_then(|c| c.get_child("errorCode"))
            .and_then(|c| c.get_text())
            .ok_or_else(|| SonosError::ParseError("failed to parse error".to_string()))?
            .parse::<u16>()
            .map_err(|_| SonosError::ParseError("Invalid error code format".to_string()))?;
  
          Err(SonosError::BadResponse(error_code))
        } else {
          Ok(self.get_child_element(body, &format!("{}Response", action.name()))?.clone())
        }
      }
      Err(e) => Err(SonosError::BadResponse(400)), // TODO: Use proper code
    }
  }
  
  pub fn get_child_element<'a>(&self, el: &'a Element, name: &str) -> Result<&'a Element, SonosError> {
    el
      .get_child(name)
      .ok_or_else(|| SonosError::ParseError(format!("missing {} element", name)).into())
  }
  
  pub fn get_child_element_text<'a>(&self, el: &'a Element, name: &str) -> Result<Cow<'a, str>, SonosError> {
    self.get_child_element(el, name)?
      .get_text()
      .ok_or_else(|| SonosError::ParseError(format!("no text on {} element", name)).into())
  }
}

impl Default for Client {
  fn default() -> Self {
    Self {
      agent: Agent::new(),
    }
  }
}

impl Clone for Client {
  fn clone(&self) -> Self {
    Self {
      agent: Agent::new(),
    }
  }
}
