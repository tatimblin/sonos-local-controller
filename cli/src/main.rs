mod view;

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

use view::{startup, control};

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let app_result = App::default().run(&mut terminal);
    ratatui::restore();
    app_result
}

#[derive(Debug)]
enum Page {
    Startup,
    Control,
}

impl Default for Page {
    fn default() -> Self {
        Page::Startup
    }
}

#[derive(Debug, Default)]
pub struct App {
    exit: bool,
    page: Page,
    speakers: Vec<Speaker>,
}

impl App {
    fn default() -> Self {
        Self {
            exit: false,
            page: Page::Startup,
            speakers: Vec::new(),
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let system= sonos::System::new()?;

        for speaker in system.speakers() {
            terminal.draw(|frame| self.draw(frame))?;
            self.speakers.push(speaker);
        }

        self.page = Page::Control;

        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            let _ = self.handle_events();
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        match self.page {
            Page::Startup => startup::draw(frame, self.speakers.last()),
            Page::Control => control::draw(frame, &mut self.speakers),
        }
    }

    fn handle_events(&mut self) -> io::Result<()> {
        if let event::Event::Key(key_event) = event::read()? {
            if self.handle_shared_event(key_event) {
                return Ok(());
            }

            match self.page {
                Page::Startup => view::startup::handle_event(self, key_event),
                Page::Control => view::control::handle_event(self, key_event),
            }
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
