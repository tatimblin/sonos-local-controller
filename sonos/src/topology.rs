use std::collections::{HashMap, HashSet};

use crate::SonosError;

#[derive(Debug, Clone)]
pub struct Topology {
  pub groups: HashMap<String, Group>,
  speaker_to_group: HashSet<String>,
}

#[derive(Debug, Clone)]
pub struct Group {
  pub coordinator_uuid: String,
  pub member_uuids: HashSet<String>,
}

impl Topology {
  pub fn from_xml(xml: &str) -> Result<Self, SonosError> {
    serde_xml_rs::from_str(xml).map_err(|e| SonosError::ParseError(format!("Failed to parse Speaker: {}", e)))
  }

  pub fn get_group_id(&self, speaker_uuid: &str) -> Option<&String> {
    self.speaker_to_group.get(speaker_uuid)
  }

  pub fn is_coordinator(&self, speaker_uuid: &str) -> bool {
    self.speaker_to_group
      .get(speaker_uuid)
      .and_then(|group_id| self.groups.get(group_id))
      .map(|group| group.coordinator_uuid == speaker_uuid)
      .unwrap_or(false)
  }
}
