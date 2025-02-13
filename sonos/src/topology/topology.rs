use crate::SonosError;

use super::get_zone_group_state_response::GetZoneGroupStateResponse;

pub struct Topology {

}

impl Topology {
  pub fn from_ip(ip: &str) -> Result<Self, SonosError> {
    let get_zone_group_state_response = GetZoneGroupStateResponse::from_ip(ip);

    println!("{:?}", get_zone_group_state_response);

    Ok(Topology{})
  }
}
