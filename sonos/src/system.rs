use std::collections::HashMap;
use std::{
  net::UdpSocket,
  io::Result,
};

use crate::topology::Topology;
use crate::Speaker;
use crate::util::ssdp::send_ssdp_request;

pub struct System {
  speakers: HashMap<String, Speaker>,
  topology: Option<Topology>,
}

#[derive(Debug)]
pub enum SystemEvent {
  Found(Speaker),
  GroupUpdate(String, Vec<String>),
  Error(String),
}

impl System {
  pub fn new() -> Result<Self> {
    Ok(System {
      speakers: HashMap::new(),
      topology: None,
    })
  }

  pub fn discover(mut self) -> impl Iterator<Item = SystemEvent> {
    let socket = UdpSocket::bind("0.0.0.0:0")
      .expect("Failed to create socket");
    let responses = send_ssdp_request(
      socket,
      "239.255.255.250:1900",
      "urn:schemas-upnp-org:device:ZonePlayer:1"
    )
      .expect("Failed to send SSDP request");

    let mut is_first_speaker = true;

    responses
      .filter(|response| response.is_ok())
      .filter_map(move |response| {
        match response {
          Ok(ssdp) => {
            match Speaker::from_location(&ssdp.location) {
              Ok(speaker) => {

                if is_first_speaker {
                  is_first_speaker = self.set_topology_from_ip(speaker.ip());
                }

                self.add_speaker(speaker.to_owned());

                Some(SystemEvent::Found(speaker))
              },
              Err(e) => Some(SystemEvent::Error(e.to_string()))
            }
          },
          Err(e) => Some(SystemEvent::Error(e.to_string())),
        }
      })
  }

  fn add_speaker(&mut self, speaker: Speaker) {
    self.speakers.insert(speaker.uuid().to_string(), speaker);
  }

  fn set_topology_from_ip(&mut self, ip: &str) -> bool {
    if let Ok(topology) = Topology::from_ip(ip) {
      self.topology = Some(topology);
      return true;
    }
    false
  }
}
