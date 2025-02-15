use super::get_zone_group_state_response::ZoneGroup;

#[derive(Debug)]
pub struct Group {
  pub name: String,
}

impl Group {
  pub fn from_zone_group(zone_group: ZoneGroup) -> Self {
    Self { name: "Hey".to_owned() }
  }
}
