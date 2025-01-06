use ratatui::layout::{ Constraint, Direction, Layout, Rect };

pub fn vertically_centered_layout(area: Rect, layout: Layout) -> (Rect, Rect) {
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
