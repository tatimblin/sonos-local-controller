#[derive(Debug)]
pub struct ServiceInfo {
  pub endpoint: &'static str,
  pub service: &'static str,
}

#[derive(Debug)]
pub enum Service {
  AVTransport(ServiceInfo),
}

impl Service {
  pub fn av_transport() -> Self {
    Service::AVTransport(ServiceInfo {
      endpoint: "MediaRenderer/AVTransport/Control",
      service: "urn:schemas-upnp-org:service:AVTransport:1",
    })
  }

  pub fn get_info(&self) -> &ServiceInfo {
    match self {
      Service::AVTransport(info) => info,
    }
  } 
}

#[derive(Debug)]
pub enum Action {
  Play,
  Pause,
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
    }
  }

  fn context(&self) -> Service {
    match self {
      Action::Play
      | Action::Pause
      => Service::av_transport(),
    }
  }
}
