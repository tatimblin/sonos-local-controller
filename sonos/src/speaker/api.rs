#[derive(Debug)]
pub enum Endpoint {
    MediaRendererRenderingControlControl,
    // Add other endpoints as needed
}

impl Endpoint {
    pub fn as_str(&self) -> &'static str {
        match self {
            Endpoint::MediaRendererRenderingControlControl => "MediaRenderer/RenderingControl/Control",
            // Map other endpoint variants to strings here
        }
    }
}

#[derive(Debug)]
pub enum Service {
  AVTransport,
  DeviceProperties,
  RenderingControl,
  ZoneGroupTopology,
  Queue,
  MusicServices,
}

impl Service {
    pub fn as_str(&self) -> &'static str {
        match self {
            Service::AVTransport => "urn:schemas-upnp-org:service:RenderingControl:1",
            Service::DeviceProperties => "urn:schemas-upnp-org:service:DeviceProperties:1",
            Service::RenderingControl => "urn:schemas-upnp-org:service:RenderingControl:1",
            Service::ZoneGroupTopology => "urn:schemas-upnp-org:service:ZoneGroupTopology:1",
            Service::Queue => "urn:schemas-upnp-org:service:Queue:1",
            Service::MusicServices => "urn:schemas-upnp-org:service:MusicServices:1",
        }
    }
}

#[derive(Debug)]
pub enum Action {
    SetMute,
}

impl Action {
    pub fn endpoint(&self) -> Endpoint {
        match self {
            Action::SetMute => Endpoint::MediaRendererRenderingControlControl,
            // Handle other actions here
        }
    }

    pub fn service(&self) -> Service {
        match self {
            Action::SetMute => Service::RenderingControl,
            // Handle other actions here
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Action::SetMute => "SetMute",
            // Add other action names here
        }
    }
}
