use ratatui::{
  layout::Rect,
  style::{Style, Stylize},
  widgets::{Block, List, ListItem},
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
    let items: Vec<String> = topology
      .items
      .iter()
      .map(|item| match item {
        TopologyItem::Group { name, .. } => format!("Group: {name}"),
        TopologyItem::Speaker { name, .. } => format!("Speaker: {name}"),
        TopologyItem::Satellite { uuid } => format!("Satellite: {uuid}"),
      })
      .collect();

    Self {
      widget: SelectableList::new("Topology", items),
      topology: topology.clone(),
    }
  }

  pub fn new_with<F>(topology: &TopologyList, item_fn: F) -> Self
  where
    F: Fn(&TopologyItem) -> String,
  {
    let items: Vec<String> = topology.items.iter().map(|item| item_fn(item)).collect();

    Self {
      widget: SelectableList::new("Topology", items),
      topology: topology.clone()
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
