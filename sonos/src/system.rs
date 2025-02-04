use std::collections::HashMap;
use std::{
  net::UdpSocket,
  io::Result,
};

use crate::model::Action;
use crate::topology::{self, Topology};
use crate::{Client, SonosError, Speaker};
use crate::util::ssdp::{
  send_ssdp_request,
  SsdpResponseIter,
};

#[derive(Debug)]
pub enum SpeakerEvent {
  Found(Speaker),
  GroupUpdate(String, Vec<String>),
  Error(String),
}

pub struct System {
  responses: SsdpResponseIter<UdpSocket>,
  speakers: HashMap<String, Speaker>,
  topology: Option<Topology>,
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
      speakers: HashMap::new(),
      topology: None,
    })
  }

  pub fn discover(self) -> impl Iterator<Item = SpeakerEvent> {
    let mut mut_self = self;

    mut_self.responses.filter_map(move |response| {
      match response {
        Ok(ssdp) => {
          match Speaker::from_location(&ssdp.location) {
            Ok(speaker) => {
              let uuid = speaker.uuid().to_string();
              mut_self.speakers.insert(uuid.clone(), speaker.clone());

              if mut_self.topology.is_none() {
                if let Ok(topology) = mut_self.fetch(&speaker) {
                  mut_self.topology = Some(topology);
                  return Some(vec![SpeakerEvent::Found(speaker), create_group_update_event(&topology)]);
                }
              }

              Some(vec![SpeakerEvent::Found(speaker)])
            },
            Err(e) => Some(vec![SpeakerEvent::Error(e.to_string())]),
          }
        },
        Err(e) => Some(vec![SpeakerEvent::Error(e.to_string())]),
      }
    }).flat_map(|events| events.into_iter())
  }

  fn fetch(&self, speaker: &Speaker) -> std::result::Result<Topology, SonosError> {
    let client = Client::default();
    let payload = "<InstanceID>0</InstanceID>";
    match client.send_action(&speaker.ip().to_string(), Action::GetZoneGroupTopology, payload) {
      Ok(response) => {
        println!("{:?}", response);

        Topology::from_xml(r#"
        <speaker>
            <name>Living Room</name>
            <volume>20</volume>
        </speaker>
        "#)
      },
      Err(e) => Err(e),
    }
  }
}

fn create_group_update_event(topology: &Topology) -> SpeakerEvent {
  if let Some((group_id, group)) = topology.groups.iter().next() {
      let mut speaker_uuids = group.member_uuids.iter()
          .map(|uuid| uuid.to_string())
          .collect::<Vec<_>>();
      speaker_uuids.push(group.coordinator_uuid.clone());
      
      SpeakerEvent::GroupUpdate(group_id.clone(), speaker_uuids)
  } else {
      SpeakerEvent::Error("No groups found in topology".to_string())
  }
}
