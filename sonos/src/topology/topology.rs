use std::collections::HashMap;

use crate::SonosError;

use crate::topology::group::Group;
use super::get_zone_group_state_response::GetZoneGroupStateResponse;

#[derive(Debug)]
pub struct Topology {
  groups: HashMap<String, Group>,
}

impl Topology {
  pub fn from_ip(ip: &str) -> Result<Self, SonosError> {
    match GetZoneGroupStateResponse::from_ip(ip) {
      Ok(response) => Self::parse_response(response),
      Err(err) => Err(err),
    }
  }

  fn parse_response(response: GetZoneGroupStateResponse) -> Result<Self, SonosError> {
    let topology = Self {
      groups: HashMap::new(),
    };

    

    Ok(topology)
  }

  pub fn create_group(&mut self, name: &str) {
    self.groups.insert(name.to_owned(), Group {
      name: name.to_owned(),
    });
  }
}
