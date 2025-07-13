use ratatui::{layout::Rect, Frame};

use crate::{topology::{topology_item::TopologyItem, topology_list::TopologyList}, widgets::selectable_list::SelectableList};

pub struct SpeakerList {
  widget: SelectableList,
  uuids: Vec<String>,
}

impl SpeakerList {
  pub fn new(topology: &TopologyList) -> Self {
    let (items, uuids): (Vec<String>, Vec<String>) = topology
      .items
      .iter()
      .enumerate()
      .map(|(i, item)| match item {
        TopologyItem::Group { uuid } => (format!("Group: {uuid}"), uuid.clone()),
        TopologyItem::Speaker { uuid } => (format!("Speaker: {uuid}"), uuid.clone()),
        TopologyItem::Satellite { uuid } => (format!("Satellite: {uuid}"), uuid.clone()),
      })
      .unzip();

    Self {
      widget: SelectableList::new(
        "Topology",
        items
      ),
      uuids
    }
  }

  pub fn new_with<FGroup, FSpeaker, FSatellite>(
    topology: &TopologyList,
    group_item: FGroup,
    speaker_item: FSpeaker,
    satellite_item: FSatellite,
  ) -> Self
  where
    FGroup: Fn(&String) -> (String + String),
    FSpeaker: Fn(&String) -> (String + String),
    FSatellite: Fn(&String) -> (String + String),
  {
    let (items, uuids): (Vec<String>, Vec<String>) = topology
      .items
      .iter()
      .enumerate()
      .map(|(i, item)| match item {
        TopologyItem::Group { uuid } => (group_item(&uuid), uuid),
        TopologyItem::Speaker { uuid } => (speaker_item(&uuid), uuid),
        TopologyItem::Satellite { uuid } => (satellite_item(&uuid), uuid),
      })
      .unzip();

    Self {
      widget: SelectableList::new(
        "Topology",
        items
      ),
      uuids
    }
  }

  pub fn draw(&mut self, frame: &mut Frame, layout: Rect) {
    self.widget.draw(frame, layout);
  }

  pub fn next(&mut self) {
    self.widget.next();
  }

  pub fn previous(&mut self) {
    self.widget.previous();
  }

  fn selected(&self) -> Option<usize> {
    self.widget.selected()
  }

  pub fn selected_uuid(&self) -> Option<&str> {
    self.selected().and_then(|i| self.uuids.get(i).map(String::as_str))
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
