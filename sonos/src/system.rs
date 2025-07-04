use std::collections::HashMap;
use std::{
  net::UdpSocket,
  io::Result,
};
use std::thread;
use std::time::Duration;
use log::{info, warn, error, debug};

use crate::topology::Topology;
use crate::speaker::{Speaker, SpeakerFactory, SpeakerTrait};
use crate::util::ssdp::send_ssdp_request;

pub struct System {
  speakers: HashMap<String, Box<dyn SpeakerTrait>>,
  topology: Option<Topology>,
}

#[derive(Debug)]
pub enum SystemEvent {
  Found(Speaker),
  Error(String),
  GroupUpdate(String, Vec<String>),
}

impl System {
  pub fn new() -> Result<Self> {
    Ok(System {
      speakers: HashMap::new(),
      topology: None,
    })
  }

  pub fn discover(self) -> impl Iterator<Item = SystemEvent> {
    info!("Starting discovery process...");

    let socket = UdpSocket::bind("0.0.0.0:0")
      .expect("Failed to create socket");

    let responses = send_ssdp_request(
      socket,
      "239.255.255.250:1900",
      "urn:schemas-upnp-org:device:ZonePlayer:1"
    )
      .expect("Failed to send SSDP request");

    info!("SSDP request sent, waiting for responses...");

    let mut is_first_speaker = true;

    responses
      .filter(|response| response.is_ok())
      .flat_map(move |response| {
        match response {
          Ok(ssdp) => {
            info!("Processing SSDP response from location: {}", ssdp.location);
            
            match Speaker::from_location(&ssdp.location) {
              Ok(speaker) => {
                info!("Successfully created speaker: {}", speaker.ip());
                
                if is_first_speaker {
                  is_first_speaker = false;
                  info!("This is the first speaker, attempting to get topology...");
                  
                  match Topology::from_ip(&speaker.ip()) {
                    Ok(topology) => {
                      debug!("Topology details: {:?}", topology);
                    },
                    Err(e) => {
                      error!("Failed to retrieve topology: {:?}", e);
                    }
                  }
                }

                Some(SystemEvent::Found(speaker))
              },
              Err(e) => {
                error!("Failed to create speaker from location {}: {}", ssdp.location, e);
                Some(SystemEvent::Error(e.to_string()))
              }
            }
          },
          Err(e) => {
            error!("Error in SSDP response: {}", e);
            Some(SystemEvent::Error(e.to_string()))
          },
        }
      })
  }
}
