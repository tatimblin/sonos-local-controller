#[derive(Debug)]
pub struct ServiceInfo {
  pub endpoint: &'static str,
  pub service: &'static str,
}

#[derive(Debug)]
pub enum Service {
  AVTransport(ServiceInfo),
  RenderingControl(ServiceInfo),
  GroupRenderingControl(ServiceInfo),
  ZoneGroupTopology(ServiceInfo),
  DeviceProperties(ServiceInfo),
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

  pub fn group_rendering_control() -> Self {
    Service::RenderingControl(ServiceInfo {
      endpoint: "MediaRenderer/GroupRenderingControl/Control",
      service: "urn:schemas-upnp-org:service:GroupRenderingControl:1",
    })
  }

  pub fn zone_group_topology() -> Self {
    Service::ZoneGroupTopology(ServiceInfo {
      endpoint: "ZoneGroupTopology/Control",
      service: "urn:schemas-upnp-org:service:ZoneGroupTopology:1",
    })
  }

  pub fn device_properties() -> Self {
    Service::DeviceProperties(ServiceInfo {
      endpoint: "DeviceProperties/Control",
      service: "urn:schemas-upnp-org:service:DeviceProperties:1",
    })
  }

  pub fn get_info(&self) -> &ServiceInfo {
    match self {
      Service::AVTransport(info) => info,
      Service::RenderingControl(info) => info,
      Service::GroupRenderingControl(info) => info,
      Service::ZoneGroupTopology(info) => info,
      Service::DeviceProperties(info) => info,
    }
  }
}

#[derive(Debug, Clone)]
pub enum Action {
  Play,
  Pause,
  Stop,
  GetVolume,
  GetGroupVolume,
  SetVolume,
  SetRelativeVolume,
  GetZoneGroupState,
  GetTransportInfo,
  SetAVTransportURI,
  GetPositionInfo,
  GetZoneInfo,
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
      Action::Stop => "Stop",
      Action::GetVolume => "GetVolume",
      Action::GetGroupVolume => "GetGroupVolume",
      Action::SetVolume => "SetVolume",
      Action::SetRelativeVolume => "SetRelativeVolume",
      Action::GetZoneGroupState => "GetZoneGroupState",
      Action::GetTransportInfo => "GetTransportInfo",
      Action::SetAVTransportURI => "SetAVTransportURI",
      Action::GetPositionInfo => "GetPositionInfo",
      Action::GetZoneInfo => "GetZoneInfo",
    }
  }

  fn context(&self) -> Service {
    match self {
      Action::Play
      | Action::Pause
      | Action::Stop
      | Action::GetTransportInfo
      | Action::SetAVTransportURI
      | Action::GetPositionInfo
      => Service::av_transport(),
      Action::GetVolume
      | Action::SetVolume
      | Action::SetRelativeVolume
      => Service::rendering_control(),
      Action::GetGroupVolume
      => Service::group_rendering_control(),
      Action::GetZoneGroupState
      => Service::zone_group_topology(),
      Action::GetZoneInfo
      => Service::device_properties(),
    }
  }
}

/// Represents the current playback state of a Sonos speaker
#[derive(Debug, Clone, PartialEq)]
pub enum PlayState {
  /// Speaker is currently playing audio
  Playing,
  /// Speaker is paused
  Paused,
  /// Speaker is stopped
  Stopped,
  /// Speaker is transitioning between states
  Transitioning,
}

impl PlayState {
  /// Parse PlayState from Sonos transport state string
  pub fn from_transport_state(state: &str) -> Self {
    match state {
      "PLAYING" => PlayState::Playing,
      "PAUSED_PLAYBACK" => PlayState::Paused,
      "STOPPED" => PlayState::Stopped,
      "TRANSITIONING" => PlayState::Transitioning,
      _ => PlayState::Stopped, // Default to stopped for unknown states
    }
  }
}
