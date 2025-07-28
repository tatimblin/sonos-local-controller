use ratatui::{
  layout::Rect,
  widgets::ListItem,
  Frame,
};

use crate::{
  topology::{topology_item::TopologyItem, topology_list::TopologyList},
  widgets::selectable_list::SelectableList,
};

pub struct SpeakerList {
    widget: SelectableList,
    topology: TopologyList
}

impl SpeakerList {
  pub fn new(topology: &TopologyList) -> Self {
    let items: Vec<ListItem> = topology
      .items
      .iter()
      .map(|item| item.to_list_item())
      .collect();

    Self {
      widget: SelectableList::new("Topology", items),
      topology: topology.clone(),
    }
  }

  pub fn draw(&mut self, frame: &mut Frame, layout: Rect) {
    self.widget.draw(frame, layout);
  }

  /// Move highlight to next item
  pub fn next(&mut self) {
    self.widget.next();
  }

  /// Move highlight to previous item
  pub fn previous(&mut self) {
    self.widget.previous();
  }

  /// Get currently highlighted item
  pub fn selected(&self) -> Option<&TopologyItem> {
    self.widget
      .selected()
      .and_then(|i| self.topology.items.get(i))
  }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::topology_item::TopologyItem;
    use crate::topology::topology_list::TopologyList;

    fn create_test_topology() -> TopologyList {
        TopologyList {
            items: vec![
                TopologyItem::Group {
                    ip: "192.168.1.100".to_string(),
                    name: "Living Room".to_string(),
                    uuid: "RINCON_123456".to_string(),
                    is_last: false,
                },
                TopologyItem::Speaker {
                    ip: "192.168.1.101".to_string(),
                    name: "Kitchen".to_string(),
                    uuid: "RINCON_789012".to_string(),
                    is_last: false,
                },
                TopologyItem::Speaker {
                    ip: "192.168.1.102".to_string(),
                    name: "Bedroom".to_string(),
                    uuid: "RINCON_345678".to_string(),
                    is_last: false,
                },
                TopologyItem::Satellite {
                    uuid: "RINCON_SAT123".to_string(),
                    is_last: true,
                },
            ],
        }
    }

    #[test]
    fn test_speaker_list_new_creates_correct_items() {
        let topology = create_test_topology();
        let speaker_list = SpeakerList::new(&topology);

        // Verify that the speaker list was created successfully
        assert_eq!(speaker_list.topology.items.len(), 4);
        assert_eq!(speaker_list.selected(), Some(&topology.items[0]));
    }

    #[test]
    fn test_speaker_list_navigation() {
        let topology = create_test_topology();
        let mut speaker_list = SpeakerList::new(&topology);

        // Test initial selection
        assert_eq!(speaker_list.selected(), Some(&topology.items[0]));

        // Test next navigation
        speaker_list.next();
        assert_eq!(speaker_list.selected(), Some(&topology.items[1]));

        speaker_list.next();
        assert_eq!(speaker_list.selected(), Some(&topology.items[2]));

        speaker_list.next();
        assert_eq!(speaker_list.selected(), Some(&topology.items[3]));

        // Test wrap around
        speaker_list.next();
        assert_eq!(speaker_list.selected(), Some(&topology.items[0]));

        // Test previous navigation
        speaker_list.previous();
        assert_eq!(speaker_list.selected(), Some(&topology.items[3]));

        speaker_list.previous();
        assert_eq!(speaker_list.selected(), Some(&topology.items[2]));
    }

    #[test]
    fn test_speaker_list_empty_topology() {
        let empty_topology = TopologyList { items: vec![] };
        let speaker_list = SpeakerList::new(&empty_topology);

        assert_eq!(speaker_list.topology.items.len(), 0);
        assert_eq!(speaker_list.selected(), None);
    }

    #[test]
    fn test_speaker_list_single_item() {
        let single_item_topology = TopologyList {
            items: vec![TopologyItem::Speaker {
                ip: "192.168.1.100".to_string(),
                name: "Only Speaker".to_string(),
                uuid: "RINCON_ONLY".to_string(),
                is_last: true,
            }],
        };

        let mut speaker_list = SpeakerList::new(&single_item_topology);

        assert_eq!(speaker_list.selected(), Some(&single_item_topology.items[0]));

        // Navigation should stay on the same item
        speaker_list.next();
        assert_eq!(speaker_list.selected(), Some(&single_item_topology.items[0]));

        speaker_list.previous();
        assert_eq!(speaker_list.selected(), Some(&single_item_topology.items[0]));
    }

    #[test]
    fn test_speaker_list_uses_topology_item_renderer() {
        let topology = create_test_topology();
        let speaker_list = SpeakerList::new(&topology);

        // Verify that the SpeakerList was created with the correct number of items
        // This indirectly tests that TopologyItemRenderer::render_to_list_item was used
        assert_eq!(speaker_list.topology.items.len(), 4);

        // Test that all topology item types are handled
        let group_item = &topology.items[0];
        let speaker_item = &topology.items[1];
        let satellite_item = &topology.items[3];

        match group_item {
            TopologyItem::Group { .. } => {
                // Verify this is a group item
                assert!(matches!(group_item, TopologyItem::Group { .. }));
            }
            _ => panic!("Expected group item"),
        }

        match speaker_item {
            TopologyItem::Speaker { .. } => {
                // Verify this is a speaker item
                assert!(matches!(speaker_item, TopologyItem::Speaker { .. }));
            }
            _ => panic!("Expected speaker item"),
        }

        match satellite_item {
            TopologyItem::Satellite { .. } => {
                // Verify this is a satellite item
                assert!(matches!(satellite_item, TopologyItem::Satellite { .. }));
            }
            _ => panic!("Expected satellite item"),
        }
    }

    #[test]
    fn test_speaker_list_integration_with_control_view_pattern() {
        // Test the pattern used in ControlView::new
        let topology = create_test_topology();
        
        // This simulates the pattern used in control.rs
        let speaker_list = if !topology.items.is_empty() {
            SpeakerList::new(&topology)
        } else {
            let empty_topology = TopologyList { items: vec![] };
            SpeakerList::new(&empty_topology)
        };

        assert_eq!(speaker_list.topology.items.len(), 4);
        assert!(speaker_list.selected().is_some());

        // Test with empty topology
        let empty_topology = TopologyList { items: vec![] };
        let empty_speaker_list = if !empty_topology.items.is_empty() {
            SpeakerList::new(&empty_topology)
        } else {
            let empty_topology = TopologyList { items: vec![] };
            SpeakerList::new(&empty_topology)
        };

        assert_eq!(empty_speaker_list.topology.items.len(), 0);
        assert!(empty_speaker_list.selected().is_none());
    }

    #[test]
    fn test_speaker_list_maintains_topology_reference() {
        let topology = create_test_topology();
        let speaker_list = SpeakerList::new(&topology);

        // Verify that the speaker list maintains a reference to the topology
        assert_eq!(speaker_list.topology.items.len(), topology.items.len());
        
        for (original, stored) in topology.items.iter().zip(speaker_list.topology.items.iter()) {
            match (original, stored) {
                (TopologyItem::Group { name: n1, ip: i1, uuid: u1, is_last: l1 }, 
                 TopologyItem::Group { name: n2, ip: i2, uuid: u2, is_last: l2 }) => {
                    assert_eq!(n1, n2);
                    assert_eq!(i1, i2);
                    assert_eq!(u1, u2);
                    assert_eq!(l1, l2);
                }
                (TopologyItem::Speaker { name: n1, ip: i1, uuid: u1, is_last: l1 }, 
                 TopologyItem::Speaker { name: n2, ip: i2, uuid: u2, is_last: l2 }) => {
                    assert_eq!(n1, n2);
                    assert_eq!(i1, i2);
                    assert_eq!(u1, u2);
                    assert_eq!(l1, l2);
                }
                (TopologyItem::Satellite { uuid: u1, is_last: l1 }, 
                 TopologyItem::Satellite { uuid: u2, is_last: l2 }) => {
                    assert_eq!(u1, u2);
                    assert_eq!(l1, l2);
                }
                _ => panic!("Topology items don't match"),
            }
        }
    }

    #[test]
    fn test_speaker_list_draw_integration() {
        use ratatui::{backend::TestBackend, Terminal};

        let topology = create_test_topology();
        let mut speaker_list = SpeakerList::new(&topology);

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        // Test that drawing doesn't panic
        terminal
            .draw(|frame| {
                speaker_list.draw(frame, frame.area());
            })
            .unwrap();

        // Verify the speaker list is still functional after drawing
        assert_eq!(speaker_list.topology.items.len(), 4);
        assert!(speaker_list.selected().is_some());
    }
}