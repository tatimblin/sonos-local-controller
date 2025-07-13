mod state;
mod hooks;
mod views;
mod widgets;
mod topology;

use std::io;
use std::sync::Arc;
use crossterm::event::{ self, KeyCode, KeyEvent };
use ratatui::DefaultTerminal;
use simplelog::*;
use std::fs::File;

use crate::state::store::Store;
use crate::hooks::use_speakers::use_speakers;
use crate::views::{
  View,
  ViewType,
  startup::StartupView,
  control::ControlView,
};

pub struct App {
  store: Arc<Store>,
  exit: bool,
  current_view: Box<dyn View>,
  current_view_type: ViewType,
}

impl App {
  pub fn new() -> Self {
    let store = Arc::new(Store::new());
    Self {
      store: store.clone(),
      current_view: Box::new(StartupView::new(store.clone())),
      current_view_type: ViewType::Startup,
      exit: false,
    }
  }

  fn update_current_view(&mut self) {
    let current_state_view = self.store.with_state(|state| state.view);

    if current_state_view != self.current_view_type {
      match current_state_view {
        ViewType::Startup => {
          log::debug!("Switching to startup view");
          self.current_view = Box::new(StartupView::new(self.store.clone()));
          self.current_view_type = ViewType::Startup;
        }
        ViewType::Control => {
          log::debug!("Switching to control view");
          self.current_view = Box::new(ControlView::new(&self.store));
          self.current_view_type = ViewType::Control;
        }
      }
    }
  }

  pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
    use_speakers(&self.store, || {
      terminal
        .draw(|frame| self.current_view.render(frame))
        .map(|_| ())
    })?;

    while !self.exit {
      self.update_current_view();
      terminal.draw(|frame| self.current_view.render(frame))?;

      if let event::Event::Key(key_event) = event::read()? {
        self.handle_input(key_event)?;
      }
    }
    Ok(())
  }

  fn handle_input(&mut self, key_event: KeyEvent) -> io::Result<()> {
    match key_event.code {
      KeyCode::Char('q') => {
        self.exit = true;
        return Ok(());
      }
      _ => {}
    }

    self.current_view.handle_input(key_event, &self.store)
  }
}

fn main() -> io::Result<()> {
  CombinedLogger::init(
    vec![
      WriteLogger::new(
        LevelFilter::Debug,
        Config::default(),
        File::create("sonos_debug.log").unwrap(),
      ),
    ]
  ).unwrap();

  let mut terminal = ratatui::init();
  let app_result = App::new().run(&mut terminal);
  ratatui::restore();
  app_result
}
