use std::{
  net::UdpSocket,
  io::Result,
};

use crate::Speaker;
use crate::util::ssdp::{
  send_ssdp_request,
  SsdpResponseIter,
};

pub struct System {
  responses: SsdpResponseIter<UdpSocket>,
}

impl System {
  pub fn new() -> Result<Self> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    let responses = send_ssdp_request(
      socket,
      "239.255.255.250:1900",
      "urn:schemas-upnp-org:device:ZonePlayer:1"
    )?;

    Ok(System {
      responses,
    })
  }

  pub fn discover(self) -> impl Iterator<Item = Speaker> {
    self.responses.filter_map(|response| {
      match response {
        Ok(ssdp) => Speaker::from_location(&ssdp.location).ok(),
        Err(e) => {
          println!("Error receiving SSDP response: {}", e);
          None
        }
      }
    })
  }
}
