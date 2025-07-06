use std::collections::HashMap;

use crate::speaker::Speaker;
use crate::SonosError;

pub struct Groups {
  groups: HashMap<String, Vec<Speaker>>,
  size: u8,
  events: String,
}

impl Groups {
  pub fn new() -> Self {
    Self {
      groups: HashMap::new(),
      size: 0,
      events: "initializing...".to_owned(),
    }
  }

  pub fn add_speaker(&mut self, speaker: Speaker) {
    if !self.groups.contains_key(&speaker.group_name) {
      self.groups.insert(speaker.group_name.clone(), Vec::new());
    }

    self.groups
      .get_mut(&speaker.group_name)
      .expect("Group should exist")
      .push(speaker);

    self.size += 1;
  }

  fn remove_speaker(&mut self, speaker_name: &str) -> Result<Speaker, SonosError> {
    let (group_name, speaker_index) = self.find_speaker(speaker_name)
      .ok_or_else(|| SonosError::DeviceNotFound("Speaker not loaded".to_owned()))?;

    let speaker = self.groups
      .get_mut(&group_name)
      .unwrap()
      .remove(speaker_index);

    self.size -= 1;

    Ok(speaker)
  }

  pub fn move_speaker(&mut self, speaker_name: &str, new_group_name: &str) -> Result<(), SonosError> {
    let mut speaker = self.remove_speaker(speaker_name)?;
    speaker.set_group_name(new_group_name);
    self.add_speaker(speaker);

    Ok(())
  }

  fn find_speaker(&mut self, speaker_name: &str) -> Option<(String, usize)> {
    for (group_name, speakers) in &mut self.groups {
      if let Some(index) = speakers.iter().position(|s| s.name == speaker_name) {
        return Some((group_name.clone(), index));
      }
    }
    None
  }

  pub fn len(&self) -> u8 {
    self.size
  }

  pub fn get_last_event(&self) -> String {
    self.events.clone()
  }
}
