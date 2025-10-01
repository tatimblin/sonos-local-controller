use ratatui::widgets::ListItem;
use sonos::PlayState;

// Include the split implementation files as part of this module
mod group;
mod satellite;
mod speaker;

#[derive(Debug, Clone, PartialEq)]
pub enum TopologyItem {
  Group {
    ip: String,
    name: String,
    uuid: String,
    children: Vec<(String, String)>,
    is_last: bool,
    play_state: PlayState,
    volume: Option<u8>,
    children_count: usize,
  },
  Speaker {
    ip: String,
    coordinator_ip: String,
    group_uuid: String,
    uuid: String,
    name: String,
    model: Option<String>,
    is_last: bool,
    volume: Option<u8>,
  },
  Satellite {
    uuid: String,
    is_last: bool,
  },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TopologyType {
    Group,
    Speaker,
    Satellite,
}

impl TopologyItem {
  pub fn get_type(&self) -> TopologyType {
    match self {
      TopologyItem::Group { .. } => TopologyType::Group,
      TopologyItem::Speaker { .. } => TopologyType::Speaker,
      TopologyItem::Satellite { .. } => TopologyType::Satellite,
    }
  }

  pub fn get_uuid(&self) -> String {
    match self {
      TopologyItem::Group { uuid, .. }
      | TopologyItem::Speaker { uuid, .. }
      | TopologyItem::Satellite { uuid, .. } => uuid.to_string(),
    }
  }

  pub fn set_is_last(&mut self, is_last: bool) {
    match self {
      TopologyItem::Group {
        is_last: ref mut last,
        ..
      }
      | TopologyItem::Speaker {
        is_last: ref mut last,
        ..
      }
      | TopologyItem::Satellite {
        is_last: ref mut last,
        ..
      } => {
        *last = is_last;
      }
    }
  }

  /// Converts this TopologyItem to a ListItem for use in SelectableList
  pub fn to_list_item(&self, highlighted: bool) -> ListItem<'static> {
    match self {
      TopologyItem::Group { .. } => self.group_to_list_item(highlighted),
      TopologyItem::Speaker { .. } => self.speaker_to_list_item(highlighted),
      TopologyItem::Satellite { .. } => self.satellite_to_list_item(highlighted),
    }
  }

  pub fn set_volume(&mut self, volume: u8) {
    match self {
      TopologyItem::Group {
        volume: ref mut vol,
        ..
      }
      | TopologyItem::Speaker {
        volume: ref mut vol,
        ..
      } => {
        *vol = Some(volume);
      }
      TopologyItem::Satellite { .. } => {
        // Satellites don't have volume control
      }
    }
  }
}

pub fn get_play_state_icon(state: &PlayState) -> String {
  let char = match state {
    PlayState::Playing => "⏸ ",
    PlayState::Transitioning => "⏸ ",
    PlayState::Paused => "▶ ",
    PlayState::Stopped => "◼ ",
  };
  char.to_string()
}
