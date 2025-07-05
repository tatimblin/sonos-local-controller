use std::io;
use crossterm::event::{ KeyCode, KeyEvent };
use ratatui::Frame;

use crate::state::store::Store;
use crate::widgets::speaker_list::SpeakerList;
use crate::state::reducers::AppAction;

use super::View;

pub struct ControlView {
  speaker_list: SpeakerList,
}

impl ControlView {
  pub fn new(store: &Store) -> Self {
    let speaker_list = store.with_state(|state| {
      if let Some(topology) = &state.topology {
        // Create speaker list from actual topology data
        let speakers: Vec<String> = topology.groups.iter()
          .flat_map(|group| group.speakers.iter().cloned())
          .collect();
        SpeakerList::from_names(&speakers)
      } else {
        SpeakerList::from_names(&Vec::new())
      }
    });

    Self { speaker_list }
  }
}

impl View for ControlView {
  fn render(&mut self, frame: &mut Frame) {
    self.speaker_list.draw(frame);
  }

  fn handle_input(&mut self, key_event: KeyEvent, store: &Store) -> io::Result<()> {
      match key_event.code {
        KeyCode::Up => {
          self.speaker_list.previous();
          if let Some(index) = self.speaker_list.selected() {
            store.dispatch(AppAction::SetSelectedSpeaker(index));
          }
        },
        KeyCode::Down => {
          self.speaker_list.next();
          if let Some(index) = self.speaker_list.selected() {
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
