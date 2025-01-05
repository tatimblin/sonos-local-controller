use ureq::Response;
use xmltree::Element;
use core::error;
use std::borrow::Cow;

use crate::SonosError;

#[derive(Debug)]
pub struct ServiceInfo {
  pub endpoint: &'static str,
  pub service: &'static str,
}

#[derive(Debug)]
pub enum Service {
  AVTransport(ServiceInfo),
  RenderingControl(ServiceInfo),
}

impl Service {
  pub fn av_transport() -> Self {
    Service::AVTransport(ServiceInfo {
      endpoint: "MediaRenderer/AVTransport/Control",
      service: "urn:schemas-upnp-org:service:AVTransport:1",
    })
  }

  pub fn rendering_control() -> Self {
    Service::RenderingControl(ServiceInfo {
      endpoint: "MediaRenderer/RenderingControl/Control",
      service: "urn:schemas-upnp-org:service:RenderingControl:1",
    })
  }

  pub fn get_info(&self) -> &ServiceInfo {
    match self {
      Service::AVTransport(info) => info,
      Service::RenderingControl(info) => info,
    }
  } 
}

#[derive(Debug)]
pub enum Action {
  Play,
  Pause,
  GetVolume,
}

impl Action {
  pub fn endpoint(&self) -> &str {
    self.context().get_info().endpoint
  }

  pub fn service(&self) -> &str {
    self.context().get_info().service
  }

  pub fn name(&self) -> &str {
    match self {
      Action::Play => "Play",
      Action::Pause => "Pause",
      Action::GetVolume => "GetVolume",
    }
  }

  fn context(&self) -> Service {
    match self {
      Action::Play
      | Action::Pause
      => Service::av_transport(),
      Action::GetVolume
      => Service::rendering_control(),
    }
  }
}

pub fn parse_xml_response(response: Response, action: Action) -> Result<Element, SonosError> {
  match response.into_string() {
    Ok(xml_string) => {
      let xml = Element::parse(xml_string.as_bytes())
        .map_err(|e| SonosError::ParseError(format!("Failed to parse XML: {}", e)))?;

      let body = get_child_element(&xml, "Body")?;

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
        Ok(get_child_element(body, &format!("{}Response", action.name()))?.clone())
      }
    }
    Err(e) => Err(SonosError::BadResponse(400)), // TODO: Use proper code
  }
}

pub fn get_child_element<'a>(el: &'a Element, name: &str) -> Result<&'a Element, SonosError> {
  el
    .get_child(name)
    .ok_or_else(|| SonosError::ParseError(format!("missing {} element", name)).into())
}

pub fn get_child_element_text<'a>(el: &'a Element, name: &str) -> Result<Cow<'a, str>, SonosError> {
  get_child_element(el, name)?
    .get_text()
    .ok_or_else(|| SonosError::ParseError(format!("no text on {} element", name)).into())
}
