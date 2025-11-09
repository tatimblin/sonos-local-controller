use ratatui::{layout::Rect, widgets::ListItem, Frame};

use crate::{
    topology::{topology_item::TopologyItem, topology_list::TopologyList},
    widgets::selectable_list::SelectableList,
};

pub struct SpeakerList {
    widget: SelectableList,
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
        }
    }

    pub fn draw(&mut self, frame: &mut Frame, layout: Rect, topology: &TopologyList) {
        let selected_index = self.widget.selected();

        let items: Vec<ListItem> = topology
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
    pub fn selected<'a>(&self, topology: &'a TopologyList) -> Option<&'a TopologyItem> {
        self.widget.selected().and_then(|i| topology.items.get(i))
    }
}
