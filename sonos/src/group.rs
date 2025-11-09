use crate::{GroupId, SonosError, SpeakerId, service::zone_group_topology::parser::{ZoneGroup, ZoneGroupMember}};

#[derive(Debug, Clone)]
pub struct SpeakerRef {
  id: SpeakerId,
  satellite_ids: Vec<SpeakerId>,
}

impl SpeakerRef {
  pub fn from_zone_group_member(zone_group_member: &ZoneGroupMember) -> Result<Self, SonosError> {
    Ok(SpeakerRef {
      id: zone_group_member.uuid.clone(),
      satellite_ids: zone_group_member.satellites
        .iter()
        .map(|satellite| satellite.uuid.clone())
        .collect()
    })
  }

  pub fn get_id(&self) -> &SpeakerId {
    &self.id
  }

  pub fn get_satellites(&self) -> &[SpeakerId] {
    &self.satellite_ids
  }
}

#[derive(Debug, Clone)]
pub struct Group {
  id: GroupId,
  coordinator_id: SpeakerId,
  members: Vec<SpeakerRef>,
}

impl Group {
  pub fn from_zone_group(zone_group: &ZoneGroup) -> Result<Self, SonosError> {
    let members = zone_group.zone_group_members
      .iter()
      .map(|zone_group_member| SpeakerRef::from_zone_group_member(zone_group_member))
      .collect::<Result<Vec<_>, _>>()?;

    Ok(Group {
      id: zone_group.id.clone(),
      coordinator_id: zone_group.coordinator.clone(),
      members,
    })
  }

  pub fn get_id(&self) -> &GroupId {
    &self.id
  }

  pub fn get_coordinator_id(&self) -> &SpeakerId {
    &self.coordinator_id
  }

  pub fn get_members(&self) -> &[SpeakerRef] {
    &self.members
  }
}
