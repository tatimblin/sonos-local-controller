use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
  layout::{Alignment, Constraint, Direction, Layout, Rect},
  text::{Line, Text},
  widgets::Paragraph,
  Frame,
};

use sonos::Speaker;

pub fn draw(frame: &mut Frame, speaker: Option<&Speaker>) {
  let logo = Text::from(vec![
		Line::from("  ___    ___    _ __     ___    ___ "),
		Line::from("/ __|  / _ \\  | '_ \\   / _ \\  / __|"),
		Line::from("\\__ \\ | (_) | | | | | | (_) | \\__ \\"),
		Line::from("|___/  \\___/  |_| |_|  \\___/  |___/"),
	]);
  let mut body = Text::from("searching...");

  if let Some(speaker) = speaker {
		body = Text::from(speaker.get_info().get_name());
  }

	let inner_layout = Layout::default()
		.direction(Direction::Vertical)
		.constraints([
			Constraint::Length(5),
			Constraint::Length(1),
		]);
  let (logo_area, text_area) = vertically_centered_layout(frame.area(), inner_layout);

  let logo_paragraph = Paragraph::new(logo).alignment(Alignment::Center);
	frame.render_widget(logo_paragraph, logo_area);

  let body_paragraph = Paragraph::new(body).alignment(Alignment::Center);
  frame.render_widget(body_paragraph, text_area);
}

fn vertically_centered_layout(area: Rect, layout: Layout) -> (Rect, Rect) {
	let offset: u16 = get_height_of_layout(&layout);

	let padding = area.height.saturating_sub(offset) / 2;

	let outer_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(padding),
            Constraint::Length(offset),
            Constraint::Length(padding),
        ])
        .split(area);

	let sections = layout.split(outer_layout[1]);

	(sections[0], sections[1])
}

fn get_height_of_layout(layout: &Layout) -> u16 {
	let dummy_rect = Rect::new(0, 0, 0, u16::MAX);
	let inner_sections = layout.split(dummy_rect);
	inner_sections.iter().map(|section| section.height).sum()
}

pub fn handle_event(app: &mut crate::App, key_event: KeyEvent) {
  match key_event.code {
    KeyCode::Char('a') => app.exit(),
    _ => {}
  }
}