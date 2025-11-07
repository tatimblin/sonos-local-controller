use std::collections::{HashMap, HashSet};

use crate::{GroupId, SpeakerId, service::zone_group_topology::parser::{ZoneGroup, ZoneGroups}};

#[derive(Clone)]
pub struct TopologySnapshot {
  pub groups: HashMap<GroupId, (SpeakerId, HashSet<SpeakerId>)>,
}

impl TopologySnapshot {
  pub fn from_parser(topology: &ZoneGroups) -> Self {
    let groups = topology.zone_groups
      .iter()
      .map(|group| {
        let coordinator = Self::speaker_id_from_rincon(&group.coordinator);
        let group_id = GroupId::from_coordinator(coordinator);
        let members = Self::collect_all_members(group);
        (group_id, (coordinator, members))
      })
      .collect();

    Self { groups }
  }

  fn speaker_id_from_rincon(rincon: &str) -> SpeakerId {
    SpeakerId::from_udn(&format!("uuid:{}", rincon))
  }

  fn collect_all_members(group: &ZoneGroup) -> HashSet<SpeakerId> {
    let mut members = HashSet::new();

    for member in &group.zone_group_members {
      members.insert(Self::speaker_id_from_rincon(&member.uuid));
      for satellite in &member.satellites {
        members.insert(Self::speaker_id_from_rincon(&satellite.uuid));
      }
    }

    members
  }

  pub fn get_speaker_group_map(&self) -> HashMap<SpeakerId, GroupId> {
    self.groups
      .iter()
      .flat_map(|(group_id, (_, members))| {
        members.iter().map(move |speaker_id| (*speaker_id, *group_id))
      })
      .collect()
  }
}
