use crossterm::event::{
  KeyCode,
  KeyEvent
};
use ratatui::{
  layout::Alignment,
  symbols::border,
  style::Stylize,
  text::Text,
  widgets::{
    block::Title,
    Block,
    Paragraph
  },
  Frame,
};

pub fn draw(frame: &mut Frame) {
  let title = Title::from(" Sonos Rooms ".bold());
  let body = Text::from("body");
  let block = Block::bordered()
      .title(title.alignment(Alignment::Center))
      .border_set(border::THICK);
  let paragraph = Paragraph::new(body)
      .centered()
      .block(block);
  frame.render_widget(paragraph, frame.area());
}

pub fn handle_event(app: &mut crate::App, key_event: KeyEvent) {
  match key_event.code {
    KeyCode::Char('a') => app.exit(),
    _ => {}
  }
}