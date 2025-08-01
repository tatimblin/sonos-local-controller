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
      .map(|item| item.to_list_item(false))
      .collect();

    Self {
      widget: SelectableList::new("Topology", items),
      topology: topology.clone(),
    }
  }

  pub fn draw(&mut self, frame: &mut Frame, layout: Rect) {
    let selected_index = self.widget.selected();

    let items: Vec<ListItem> = self.topology
      .items
      .iter()
      .enumerate()
      .map(|(i, item)| {
        let is_highlighted = selected_index == Some(i);
        item.to_list_item(is_highlighted)
      })
      .collect();
    
    self.widget.update_items(items);
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
