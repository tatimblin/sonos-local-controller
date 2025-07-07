use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use std::io;

use crate::state::reducers::AppAction;
use crate::state::store::Store;
use crate::widgets::topology_list::{HierarchicalItem, TopologyList};

use super::View;

pub struct ControlView {
    topology_list: TopologyList,
}

impl ControlView {
    pub fn new(store: &Store) -> Self {
        let topology_list = store.with_state(|state| {
            if let Some(topology) = &state.topology {
                // Create topology list from actual topology data
                TopologyList::new(topology)
            } else {
                // Create empty topology list for empty state
                let empty_topology = crate::types::Topology { groups: vec![] };
                TopologyList::new(&empty_topology)
            }
        });

        Self { topology_list }
    }

    fn handle_selection(&self, item: &HierarchicalItem, store: &Store) {
      match item {
        HierarchicalItem::Group { name: _, uuid, .. } => {
          store.dispatch(AppAction::SetSelectedGroupUuid(uuid.clone()));
        }
        HierarchicalItem::Speaker { name: _, uuid, .. } => {
          store.dispatch(AppAction::SetSelectedSpeakerUuid(uuid.clone()));
        }
        HierarchicalItem::Satellite { name: _, uuid, ..} => {
          store.dispatch(AppAction::SetSelectedSpeakerUuid(uuid.clone()));
        }
      }
    }
}

impl View for ControlView {
    fn render(&mut self, frame: &mut Frame) {
        self.topology_list.draw(frame, frame.area());
    }

    fn handle_input(&mut self, key_event: KeyEvent, store: &Store) -> io::Result<()> {
        match key_event.code {
            KeyCode::Up => {
                self.topology_list.previous();
                if let Some(item) = self.topology_list.selected_item() {
                    self.handle_selection(item, store);
                }
            }
            KeyCode::Down => {
                self.topology_list.next();
                if let Some(item) = self.topology_list.selected_item() {
                    self.handle_selection(item, store);
                }
            }
            KeyCode::Left => {
                store.dispatch(AppAction::AdjustVolume(-4));
            }
            KeyCode::Right => {
                store.dispatch(AppAction::AdjustVolume(4));
            }
            _ => {}
        }
        Ok(())
    }
}
