use ratatui::{
  layout::{ Alignment, Constraint, Direction, Layout },
  text::Text,
  widgets::Paragraph,
  Frame,
};

use sonos::Speaker;

use crate::widget::{ logo, util };

pub fn draw(frame: &mut Frame, speaker: Option<&Speaker>) {
  let logo = logo();
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
  let (logo_area, text_area) = util::vertically_centered_layout(frame.area(), inner_layout);

  let logo_paragraph = Paragraph::new(logo).alignment(Alignment::Center);
	frame.render_widget(logo_paragraph, logo_area);

  let body_paragraph = Paragraph::new(body).alignment(Alignment::Center);
  frame.render_widget(body_paragraph, text_area);
}
