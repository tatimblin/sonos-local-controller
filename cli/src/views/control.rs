use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use std::io;

use crate::state::reducers::AppAction;
use crate::state::store::Store;
use crate::topology::topology_item::TopologyItem;
use crate::topology::topology_list::TopologyList;
use crate::widgets::speaker_list::SpeakerList;

use super::View;

pub struct ControlView {
    list_widget: SpeakerList,
}

impl ControlView {
    pub fn new(store: &Store) -> Self {
        let list_widget = store.with_state(|state| {
            if let Some(topology) = &state.topology {
                // Create topology list from actual topology data
                SpeakerList::new(topology)
            } else {
                // Create empty topology list for empty state
                let empty_topology = TopologyList { items: vec!() };
                SpeakerList::new(&empty_topology)
            }
        });

        Self { list_widget }
    }

    // TODO (ttimblin): remove match now that all types have `uuid`
    fn handle_selection(&self, item: &TopologyItem, store: &Store) {
      match item {
        TopologyItem::Group { uuid, .. } => {
          store.dispatch(AppAction::SetSelectedSpeakerUuid(uuid.clone()));
        }
        TopologyItem::Speaker { uuid, .. } => {
          store.dispatch(AppAction::SetSelectedSpeakerUuid(uuid.clone()));
        }
        TopologyItem::Satellite { uuid, ..} => {
          store.dispatch(AppAction::SetSelectedSpeakerUuid(uuid.clone()));
        }
      }
    }
}

impl View for ControlView {
    fn render(&mut self, frame: &mut Frame) {
        self.list_widget.draw(frame);
    }

    fn handle_input(&mut self, key_event: KeyEvent, store: &Store) -> io::Result<()> {
        match key_event.code {
            KeyCode::Up => {
                self.list_widget.previous();
                // if let Some(item) = self.list_widget.selected_item() {
                //     self.handle_selection(item, store);
                // }
            }
            KeyCode::Down => {
                self.list_widget.next();
                // if let Some(item) = self.list_widget.selected_item() {
                //     self.handle_selection(item, store);
                // }
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
