use ureq::{ Agent, Response };
use xmltree::Element;
use std::borrow::Cow;
use log::{info, warn, error, debug};

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
    debug!("Preparing SOAP request for action: {:?}", action);
    debug!("Target IP: {}", ip);
    debug!("Payload: {}", payload);
    
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
    
    debug!("SOAP Action: {}", soap_action);
    debug!("URL: {}", url);
    debug!("Request body length: {} characters", body.len());

    info!("Sending HTTP POST request to {}...", url);
    let response = self.agent.post(&url)
      .set("Content-Type", "text/xml; charset=\"utf-8\"")
      .set("SOAPACTION", &soap_action)
      .send_string(&body);

    match response {
      Ok(response) => {
        info!("Received HTTP response with status: {}", response.status());
        self.parse_xml_response(response, action)
      },
      Err(e) => {
        error!("HTTP request failed: {:?}", e);
        Err(SonosError::DeviceUnreachable)
      },
    }
  }

  fn parse_xml_response(&self, response: Response, action: Action) -> Result<Element, SonosError> {
    debug!("Parsing XML response...");
    
    match response.into_string() {
      Ok(xml_string) => {
        debug!("Response body length: {} characters", xml_string.len());
        debug!("First 200 chars of response: {}", 
                 if xml_string.len() > 200 { &xml_string[..200] } else { &xml_string });
        
        debug!("Parsing XML...");
        let xml = Element::parse(xml_string.as_bytes())
          .map_err(|e| {
            error!("Failed to parse XML: {}", e);
            SonosError::ParseError(format!("Failed to parse XML: {}", e))
          })?;
  
        debug!("Successfully parsed XML, looking for Body element...");
        let body = self.get_child_element(&xml, "Body")?;
        debug!("Found Body element");
  
        if let Some(fault) = body.get_child("Fault") {
          warn!("Found SOAP Fault in response");
          let error_code = fault
            .get_child("detail")
            .and_then(|c| c.get_child("UpnPError"))
            .and_then(|c| c.get_child("errorCode"))
            .and_then(|c| c.get_text())
            .ok_or_else(|| SonosError::ParseError("failed to parse error".to_string()))?
            .parse::<u16>()
            .map_err(|_| SonosError::ParseError("Invalid error code format".to_string()))?;
  
          error!("SOAP Fault error code: {}", error_code);
          Err(SonosError::BadResponse(error_code))
        } else {
          let response_element_name = format!("{}Response", action.name());
          debug!("Looking for response element: {}", response_element_name);
          
          match self.get_child_element(body, &response_element_name) {
            Ok(response_element) => {
              debug!("Successfully found response element");
              Ok(response_element.clone())
            },
            Err(e) => {
              error!("Failed to find response element: {:?}", e);
              Err(e)
            }
          }
        }
      }
      Err(e) => {
        error!("Failed to convert response to string: {:?}", e);
        Err(SonosError::BadResponse(400)) // TODO: Use proper code
      }
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
