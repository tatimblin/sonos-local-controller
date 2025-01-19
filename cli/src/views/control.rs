use std::io;
use crossterm::event::{ KeyCode, KeyEvent };
use ratatui::Frame;

use crate::state::store::Store;
use crate::widgets::speaker_list::SpeakerList;
use crate::types::AppAction;

use super::View;

pub struct ControlView {
  speaker_list: SpeakerList,
}

impl ControlView {
  pub fn new(store: &Store) -> Self {
    let speaker_list = store.with_state(|state| {
      SpeakerList::new(&state.speakers)
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
        _ => {}
      }
      Ok(())
  }
}
