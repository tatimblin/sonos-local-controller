use crossterm::event::{
  KeyCode,
  KeyEvent
};
use ratatui::{
  widgets::{
    Block,
    Borders,
    List,
    ListItem,
  },
  Frame,
};

use sonos::Speaker;

pub fn draw(frame: &mut Frame, speakers: &mut Vec<Speaker>) {
  let labels: Vec<ListItem> = speakers
    .iter()
    .map(|speaker| {
      let text = format!(
        "{} - {}",
        speaker.get_info().get_name(),
        speaker.get_info().get_room_name()
      );
      ListItem::new(text)
    })
    .collect();
  
  let list = List::new(labels)
    .block(Block::default().borders(Borders::ALL).title("Speakers"));

  frame.render_widget(list, frame.area());
}

pub fn handle_event(app: &mut crate::App, key_event: KeyEvent) {
  match key_event.code {
    KeyCode::Char('a') => app.exit(),
    _ => {}
  }
}
