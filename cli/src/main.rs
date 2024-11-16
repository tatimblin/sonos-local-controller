mod view;

use std::io;

use crossterm::event::{
    self,
    Event,
    KeyCode,
    KeyEvent,
    KeyEventKind,
};

use ratatui::{
    layout::{Alignment, Layout},
    style::Stylize,
    symbols::border,
    text::Text,
    widgets::{
        block::Title,
        Block,
        Paragraph,
    },
    DefaultTerminal,
    Frame,
};

use sonos::Speaker;

use view::startup::draw_startup_page;

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
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let system= sonos::System::new()?;
        let mut speakers: Vec<Speaker> = Vec::new();

        for speaker in system.speakers() {
            let speaker_partial_clone = &[Speaker::new_with_name(speaker.name.clone())];
            terminal.draw(|frame| self.draw(frame, speaker_partial_clone))?;
            speakers.push(speaker);
        }

        self.page = Page::Control;

        while !self.exit {
            terminal.draw(|frame| self.draw(frame, &speakers))?;
            let _ = self.handle_events();
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame, speakers: &[Speaker]) {
        match self.page {
            Page::Startup => draw_startup_page(frame, speakers),
            Page::Control => self.draw_control_page(frame),
        }
    }

    

    fn draw_control_page(&self, frame: &mut Frame) {
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

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}
