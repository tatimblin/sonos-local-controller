use std::io;
use crossterm::event::{
  KeyCode,
  KeyEvent
};
use ratatui::{
  DefaultTerminal,
  Frame
};

use sonos::Speaker;

// use crate::EventHandler;
// use crate::Page;
use crate::widget::selectable_list::SelectableList;

pub struct ControlState {
  list: SelectableList,
}

impl ControlState {
  pub fn new(speakers: &Vec<Speaker>) -> Self {
    let labels: Vec<String> = speakers
      .iter()
      .map(|speaker| {
        format!(
          "{} - {}",
          speaker.get_info().get_name(),
          speaker.get_info().get_room_name()
        )
      })
      .collect();

    Self {
      list: SelectableList::new("Speakers", labels),
    }
  }
}

pub fn render(frame: &mut Frame, state: &mut ControlState) {
  state.list.draw(frame, frame.area());
}

pub fn handle_input(state: &mut ControlState, key_event: KeyEvent, terminal: &mut DefaultTerminal) -> io::Result<()> {
  match key_event.code {
    KeyCode::Down => {
      state.list.next();
      terminal.draw(|frame| {
        state.list.draw(frame, frame.area());
      })?;
    },
    KeyCode::Up => {
      state.list.previous();
      terminal.draw(|frame| {
        state.list.draw(frame, frame.area());
      })?;
    }
    _ => {}
  }
  Ok(())
}
