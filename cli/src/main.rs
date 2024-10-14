use std::io;

use crossterm::event::{
    self,
    Event,
    KeyCode,
    KeyEvent,
    KeyEventKind,
};

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{
        block::{Position, Title},
        Block,
        Paragraph,
        Widget,
    },
    DefaultTerminal,
    Frame,
};

// use sdk::helloworld::HelloWorld;

fn main() -> io::Result<()> {
    // let hw = HelloWorld {
    //     name: "Tristan".to_string(),
    // };

    // println!("{}", hw.greeting());

    let mut terminal = ratatui::init();
    let app_result = App::default().run(&mut terminal);
    ratatui::restore();
    app_result
}

#[derive(Debug, Default)]
pub struct App {
    exit: bool,
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events();
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
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

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Title::from(" Sonos Rooms ".bold());

        let body = Text::from("body");

        let block = Block::bordered()
            .title(title.alignment(Alignment::Center))
            .border_set(border::THICK);

        Paragraph::new(body)
            .centered()
            .block(block)
            .render(area, buf);
    }
}
