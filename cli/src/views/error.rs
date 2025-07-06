use std::io;
use std::sync::Arc;
use crossterm::event::KeyEvent;
use ratatui::{
  layout::{ Alignment, Constraint, Direction, Layout },
  text::Text,
  widgets::Paragraph,
  Frame,
};

use crate::state::store::Store;
use crate::widgets::{ logo::logo, util };

use super::ViewTrait;

pub struct ErrorView {
  store: Arc<Store>,
}

impl ErrorView {
  pub fn new(store: Arc<Store>) -> Self {
    Self { store }
  }

  fn get_error(&self) -> String {
    self.store.with_state(|state| {
      state.error.clone()
        .map(|e| e.to_string())
        .unwrap_or_else(|| "Unknown error".to_string())
    })
  }
}

impl ViewTrait for ErrorView {
  fn render(&mut self, frame: &mut Frame) {
    let logo = logo();
    let body = Text::from(self.get_error());

    let inner_layout = Layout::default()
      .direction(Direction::Vertical)
      .constraints([
        Constraint::Length(5),
        Constraint::Length(1),
      ]);
    let (logo_area, text_area) = util::vertically_centered_layout(frame.area(), inner_layout);

    let logo_paragraph = Paragraph::new(logo).alignment(Alignment::Center);
    frame.render_widget(logo_paragraph, logo_area);

    let body_paragraph = Paragraph::new(body).alignment(Alignment::Center);
    frame.render_widget(body_paragraph, text_area);
  }

  fn handle_input(&mut self, _key_event: KeyEvent, _store: &Store) -> io::Result<()> {
    Ok(())
  }
}
