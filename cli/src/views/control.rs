use std::io;
use crossterm::event::{ KeyCode, KeyEvent };
use ratatui::Frame;

use crate::state::store::Store;
use crate::widgets::topology_list::TopologyList;
use crate::state::reducers::AppAction;

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
}

impl View for ControlView {
  fn render(&mut self, frame: &mut Frame) {
    self.topology_list.draw(frame, frame.area());
  }

  fn handle_input(&mut self, key_event: KeyEvent, store: &Store) -> io::Result<()> {
      match key_event.code {
        KeyCode::Up => {
          self.topology_list.previous();
          if let Some(index) = self.topology_list.selected() {
            store.dispatch(AppAction::SetSelectedSpeaker(index));
          }
        },
        KeyCode::Down => {
          self.topology_list.next();
          if let Some(index) = self.topology_list.selected() {
            store.dispatch(AppAction::SetSelectedSpeaker(index));
          }
        },
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
