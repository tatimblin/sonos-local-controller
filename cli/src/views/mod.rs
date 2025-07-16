pub mod startup;
pub mod control;

use std::io;
use crossterm::event::KeyEvent;
use ratatui::Frame;
use crate::Store;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ViewType {
  Startup,
  Control,
}

pub trait View {
  fn render(&mut self, frame: &mut Frame);
  fn handle_input(&mut self, key_event: KeyEvent, store: &Store) -> io::Result<()>;
}
