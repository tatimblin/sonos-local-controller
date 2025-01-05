mod view;
pub mod widget;
mod page;

use std::io;

use crossterm::event::{
  self,
  KeyCode,
  KeyEvent,
};

use ratatui::{
  DefaultTerminal,
  Frame,
};

use sonos::Speaker;
use view::control::ControlState;

use crate::page::Page;
use crate::view::{
  startup,
  control,
};

fn main() -> io::Result<()> {
  let mut terminal = ratatui::init();
  let app_result = App::default().run(&mut terminal);
  ratatui::restore();
  app_result
}

#[derive(Default)]
pub struct App {
  exit: bool,
  state: AppState,
}

#[derive(Default)]
struct AppState {
  page: Page,
  speakers: Vec<Speaker>,
  control_state: Option<control::ControlState>,
}

impl App {
  fn default() -> Self {
    Self {
      exit: false,
      state: AppState {
        page: Page::Startup,
        speakers: Vec::new(),
        control_state: None,
      },
    }
  }

  pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
    let system = sonos::System::new()?;

    for speaker in system.speakers() {
      terminal.draw(|frame| self.render(frame))?;
      self.state.speakers.push(speaker);
    }
    self.state.page = Page::Control;
  
    self.state.control_state = Some(ControlState::new(&self.state.speakers));

    while !self.exit {
      terminal.draw(|frame| self.render(frame))?;
      
      if let event::Event::Key(key_event) = event::read()? {
        self.handle_input(key_event, terminal)?;
      }
    }
    Ok(())
  }

  fn render(&mut self, frame: &mut Frame) {
    match self.state.page {
      Page::Startup => startup::draw(frame, self.state.speakers.last()),
      Page::Control => {
        if let Some(control_state) = &mut self.state.control_state {
          control::render(frame, control_state)
        }
      },
      Page::Unknown => startup::draw(frame, self.state.speakers.last()) // Add error page.
    }
  }

  fn handle_input(&mut self, key_event: KeyEvent, terminal: &mut DefaultTerminal) -> io::Result<()> {
    if self.handle_shared_event(key_event) {
      return Ok(());
    }

    match self.state.page {
      Page::Control => {
        if let Some(control_state) = &mut self.state.control_state {
          control::handle_input(control_state, key_event, terminal)?;
        }
      }
      _ => {}
    }
    Ok(())
  }

  fn handle_shared_event(&mut self, key_event: KeyEvent) -> bool {
    match key_event.code {
      KeyCode::Char('q') => {
        self.exit();
        true
      },
      _ => false,
    }
  }

  fn exit(&mut self) {
    self.exit = true;
  }
}
