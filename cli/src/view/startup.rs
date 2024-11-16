use ratatui::{
  layout::{Alignment, Constraint, Direction, Layout, Rect},
  text::{Line, Text},
  widgets::Paragraph,
  Frame,
};

use sonos::Speaker;

pub fn draw_startup_page(frame: &mut Frame, speakers: &[Speaker]) {
  let logo = Text::from(vec![
		Line::from("  ___    ___    _ __     ___    ___ "),
		Line::from("/ __|  / _ \\  | '_ \\   / _ \\  / __|"),
		Line::from("\\__ \\ | (_) | | | | | | (_) | \\__ \\"),
		Line::from("|___/  \\___/  |_| |_|  \\___/  |___/"),
	]);
  let mut body = Text::from("searching...");

  if let Some(speaker) = speakers.get(0) {
		body = Text::from(speaker.name.as_str());
  }

  let (logo_area, text_area) = vertically_centered_layout(frame.area());

  let logo_paragraph = Paragraph::new(logo)
		.alignment(Alignment::Center);
  let body_paragraph = Paragraph::new(body)
		.alignment(Alignment::Center);

  frame.render_widget(logo_paragraph, logo_area);
  frame.render_widget(body_paragraph, text_area);
}

fn vertically_centered_layout(area: Rect) -> (Rect, Rect) {
	let layout = Layout::default()
		.direction(Direction::Vertical)
		.constraints([
			Constraint::Percentage(40), // Empty space at the top
			Constraint::Min(0),         // Space for logo and body
			Constraint::Percentage(40), // Empty space at the bottom
		])
		.split(area);

	let inner_layout = Layout::default()
		.direction(Direction::Vertical)
		.constraints([
			Constraint::Length(5), // Space for logo
			Constraint::Length(1), // Space for body text
		])
		.split(layout[1]);

	(inner_layout[0], inner_layout[1])
}