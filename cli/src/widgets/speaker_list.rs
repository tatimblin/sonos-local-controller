use ratatui::{layout::Rect, Frame, style::{Style, Stylize}, widgets::{Block, List, ListItem, ListState}};

use crate::{topology::{topology_item::TopologyItem, topology_list::TopologyList}, state::store::{AppState, SpeakerDisplayState}};

pub struct SpeakerList {
  topology_items: Vec<TopologyItem>,
  state: ListState,
}

impl SpeakerList {
  pub fn new(topology: &TopologyList) -> Self {
    let mut state = ListState::default();
    if !topology.items.is_empty() {
      state.select(Some(0));
    }
    
    Self {
      topology_items: topology.items.clone(),
      state,
    }
  }

  pub fn new_with<F>(
    topology: &TopologyList,
    _item_fn: F
  ) -> Self
  where
    F: Fn(&TopologyItem) -> (String, String),
  {
    let mut state = ListState::default();
    if !topology.items.is_empty() {
      state.select(Some(0));
    }
    
    Self {
      topology_items: topology.items.clone(),
      state,
    }
  }

  pub fn draw(&mut self, frame: &mut Frame, layout: Rect, app_state: &AppState) {
    let list_items: Vec<ListItem> = self
      .topology_items
      .iter()
      .enumerate()
      .map(|(_i, item)| {
        let display_text = match item {
          TopologyItem::Group { uuid: _, name } => format!("Group: {name}"),
          TopologyItem::Speaker { uuid } => format!("  Speaker: {uuid}"),
          TopologyItem::Satellite { uuid } => format!("  Satellite: {uuid}"),
        };
        
        let uuid = item.get_uuid();
        let display_state = app_state.get_speaker_display_state(uuid);
        let style = self.get_item_style(&display_state);
        
        ListItem::new(display_text).style(style)
      })
      .collect();

    let list = List::new(list_items)
      .block(Block::default().title("Topology"))
      .highlight_style(Style::new().reversed())
      .highlight_symbol(">> ");

    frame.render_stateful_widget(list, layout, &mut self.state);
  }

  /// Get the appropriate style for a speaker based on its display state
  fn get_item_style(&self, display_state: &SpeakerDisplayState) -> Style {
    match display_state {
      SpeakerDisplayState::Normal => Style::default(),
      SpeakerDisplayState::Active => Style::default().reversed(),
      SpeakerDisplayState::Locked => Style::default().bold().yellow(),
      SpeakerDisplayState::ActiveAndLocked => Style::default().bold().yellow().reversed(),
    }
  }

  /// Move highlight to next item
  pub fn next(&mut self) {
    if self.topology_items.is_empty() {
      return;
    }

    let i = self.state.selected().unwrap_or(0);
    let next = (i + 1) % self.topology_items.len();
    self.state.select(Some(next));
  }

  /// Move highlight to previous item
  pub fn previous(&mut self) {
    if self.topology_items.is_empty() {
      return;
    }

    let i = match self.state.selected() {
      Some(i) => {
        if i == 0 {
          self.topology_items.len() - 1
        } else {
          i - 1
        }
      }
      None => 0,
    };
    self.state.select(Some(i));
  }

  /// Get currently highlighted item
  pub fn selected(&self) -> Option<&TopologyItem> {
    self.state.selected().and_then(|i| self.topology_items.get(i))
  }




}

// #[cfg(test)]
// mod tests {
//   use super::*;
//   use sonos::testing::MockSpeakerBuilder;

//   #[test]
//   fn test_new_speaker() {
//     let speaker = MockSpeakerBuilder::new().build();
//     let mut speaker_list = SpeakerList::new(&[
//       Box::new(speaker)
//     ]);

//     assert_eq!(speaker_list.len(), 1, "List should have 1 speaker");

//     let selected = speaker_list.selected();
//     assert_eq!(selected, Some(0), "First speaker should be initially selected");
//   }

//   #[test]
//   fn test_select_next_speaker() {
//     let speaker_a = MockSpeakerBuilder::new().build();
//     let speaker_b = MockSpeakerBuilder::new().build();
//     let mut speaker_list = SpeakerList::new(&[
//       Box::new(speaker_a),
//       Box::new(speaker_b)
//     ]);

//     assert_eq!(speaker_list.len(), 2, "List should have 2 speakers");

//     assert_eq!(speaker_list.selected(), Some(0), "First speaker should be initially selected");
    
//     speaker_list.next();

//     assert_eq!(speaker_list.selected(), Some(1), "Second speaker should then be selected");
//   }

//   #[test]
//   fn test_select_previous_speaker() {
//     let speaker_a = MockSpeakerBuilder::new().build();
//     let speaker_b = MockSpeakerBuilder::new().build();
//     let mut speaker_list = SpeakerList::new(&[
//       Box::new(speaker_a),
//       Box::new(speaker_b)
//     ]);

//     speaker_list.next();

//     assert_eq!(speaker_list.len(), 2, "List should have 2 speakers");

//     assert_eq!(speaker_list.selected(), Some(1), "Second speaker should be selected");
    
//     speaker_list.previous();

//     assert_eq!(speaker_list.selected(), Some(0), "First speaker should then be selected");
//   }

//   #[test]
//   fn test_select_next_speaker_wrapped() {
//     let speaker_a = MockSpeakerBuilder::new().build();
//     let speaker_b = MockSpeakerBuilder::new().build();
//     let mut speaker_list = SpeakerList::new(&[
//       Box::new(speaker_a),
//       Box::new(speaker_b)
//       ]);

//     speaker_list.next();
    
//     assert_eq!(speaker_list.len(), 2, "List should have 2 speakers");
    
//     assert_eq!(speaker_list.selected(), Some(1), "Seconds speaker should be selected");
    
//     speaker_list.next();

//     assert_eq!(speaker_list.selected(), Some(0), "First speaker should then be selected");
//   }

//   #[test]
//   fn test_select_previous_speaker_wrapped() {
//     let speaker_a = MockSpeakerBuilder::new().build();
//     let speaker_b = MockSpeakerBuilder::new().build();
//     let mut speaker_list = SpeakerList::new(&[
//       Box::new(speaker_a),
//       Box::new(speaker_b)
//     ]);

//     assert_eq!(speaker_list.len(), 2, "List should have 2 speakers");

//     assert_eq!(speaker_list.selected(), Some(0), "First speaker should be initially selected");
    
//     speaker_list.previous();

//     assert_eq!(speaker_list.selected(), Some(1), "Second speaker should then be selected");
//   }
// }
