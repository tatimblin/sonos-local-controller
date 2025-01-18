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

use crate::widget::selectable_list::SelectableList;

pub struct ControlState {
  list: SelectableList,
}

impl ControlState {
  pub fn new(speakers: &mut Vec<Speaker>) -> Self {
    let labels: Vec<String> = speakers
      .iter_mut()
      .map(|speaker| {
        let name = speaker.get_info().get_name().to_string();
        let room = speaker.get_info().get_room_name().to_string();
        let volume = speaker.get_volume().unwrap_or(0);

        format!(
          "{} - {}: {}",
          name,
          room,
          volume
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
    },
    KeyCode::Left => {
      let speaker = state.list.
    },
    _ => {}
  }
  Ok(())
}
